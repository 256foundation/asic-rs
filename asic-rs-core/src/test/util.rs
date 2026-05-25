use std::{collections::HashSet, net::IpAddr, panic::AssertUnwindSafe, sync::Arc, time::Duration};

use futures::{FutureExt, StreamExt, pin_mut, stream::FuturesUnordered};

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

    let found = {
        let mut commands: HashSet<MinerCommand> = HashSet::new();
        for fw in registry.iter() {
            for cmd in fw.get_discovery_commands() {
                commands.insert(cmd);
            }
        }

        let mut discovery_tasks = FuturesUnordered::new();
        for command in commands {
            let reg = registry.clone();
            discovery_tasks.push(async move {
                match AssertUnwindSafe(get_miner_type_from_command(ip, command, reg))
                    .catch_unwind()
                    .await
                {
                    Ok(result) => result,
                    Err(_) => {
                        tracing::warn!("discovery command panicked for {ip}");
                        None
                    }
                }
            });
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
                r = discovery_tasks.next() => {
                    if let Some(Some(fw)) = r {
                        found = Some(fw);
                        break;
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
                    r = discovery_tasks.next() => {
                        if let Some(Some(fw)) = r
                            && !fw.is_stock()
                        {
                            found = Some(fw);
                            break;
                        }
                    }
                }
            }
        }

        found
    };

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
