use std::{collections::HashSet, net::IpAddr, sync::Arc, time::Duration};

use futures::{FutureExt, pin_mut};
use tokio::task::JoinSet;

use crate::{
    data::command::MinerCommand,
    traits::{entry::FirmwareEntry, identification::WebResponse, miner::Miner},
    util::{send_rpc_command, send_web_command},
};

pub async fn get_miner(
    ip: IpAddr,
    firmware: Arc<dyn FirmwareEntry>,
) -> anyhow::Result<Option<Box<dyn Miner>>> {
    let registry: Arc<[Arc<dyn FirmwareEntry>]> = Arc::new([firmware]);

    let mut commands: HashSet<MinerCommand> = HashSet::new();
    for fw in registry.iter() {
        for cmd in fw.get_discovery_commands() {
            commands.insert(cmd);
        }
    }

    let mut discovery_tasks = JoinSet::new();
    for command in commands {
        let reg = registry.clone();
        discovery_tasks.spawn(get_miner_type_from_command(ip, command, reg));
    }

    let id_timeout = tokio::time::sleep(Duration::from_secs(5)).fuse();
    pin_mut!(id_timeout);

    let mut found: Option<Arc<dyn FirmwareEntry>> = None;

    loop {
        if discovery_tasks.is_empty() {
            break;
        }
        tokio::select! {
            _ = &mut id_timeout => break,
            r = discovery_tasks.join_next() => {
                match r.unwrap_or(Ok(None)) {
                    Ok(Some(fw)) if !fw.is_stock() => {
                        found = Some(fw);
                        break;
                    }
                    Ok(Some(fw)) => {
                        found = Some(fw);
                        break;
                    }
                    _ => continue,
                }
            }
        }
    }

    // If we found a stock firmware, wait a short window for non-stock to respond
    if found.as_ref().map(|f| f.is_stock()).unwrap_or(false) {
        let upgrade_window = tokio::time::sleep(Duration::from_millis(300)).fuse();
        pin_mut!(upgrade_window);

        loop {
            if discovery_tasks.is_empty() {
                break;
            }
            tokio::select! {
                _ = &mut id_timeout => break,
                _ = &mut upgrade_window => break,
                r = discovery_tasks.join_next() => {
                    if let Ok(Some(fw)) = r.unwrap_or(Ok(None))
                        && !fw.is_stock()
                    {
                        found = Some(fw);
                        break;
                    }
                }
            }
        }
    }

    discovery_tasks.abort_all();
    while discovery_tasks.join_next().await.is_some() {}

    match found {
        Some(fw) => match fw.build_miner(ip, None).await {
            Ok(miner) => Ok(Some(miner)),
            Err(e) => {
                tracing::debug!("failed to build miner for {ip}: {e}");
                Ok(None)
            }
        },
        None => {
            tracing::debug!("failed to identify {ip}");
            Ok(None)
        }
    }
}

async fn get_miner_type_from_command(
    ip: IpAddr,
    command: MinerCommand,
    registry: Arc<[Arc<dyn FirmwareEntry>]>,
) -> Option<Arc<dyn FirmwareEntry>> {
    match command {
        MinerCommand::RPC { command, .. } => {
            let response = send_rpc_command(&ip, command).await?;
            let upper = response.to_string().to_uppercase();
            registry.iter().find(|fw| fw.identify_rpc(&upper)).cloned()
        }
        MinerCommand::WebAPI { command, .. } => {
            let (body, headers, status) = send_web_command(&ip, command).await?;
            let auth_header = headers
                .get("www-authenticate")
                .and_then(|h| h.to_str().ok())
                .unwrap_or("");
            let algo_header = headers
                .get("algorithm")
                .and_then(|h| h.to_str().ok())
                .unwrap_or("");
            let redirect_header = headers
                .get("location")
                .and_then(|h| h.to_str().ok())
                .unwrap_or("");
            let web_resp = WebResponse {
                body: &body,
                auth_header,
                algo_header,
                redirect_header,
                status: status.as_u16(),
            };
            registry
                .iter()
                .find(|fw| fw.identify_web(&web_resp))
                .cloned()
        }
        _ => None,
    }
}
