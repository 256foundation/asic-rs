use std::{net::IpAddr, sync::LazyLock};

use reqwest::{StatusCode, header::HeaderMap};
use serde_json::json;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// Returns true if the error is an expected transient failure from a
/// privileged write — timeout or connection drop. These indicate the miner
/// received and applied the command but didn't respond in time.
pub fn is_expected_write_error(err: &anyhow::Error) -> bool {
    // Read timeout from read_stream_response (tokio::time::timeout elapsed)
    if err.to_string().contains("timed out") {
        return true;
    }
    // IO errors: connection reset, broken pipe, etc.
    if let Some(io_err) = err.downcast_ref::<std::io::Error>() {
        return matches!(
            io_err.kind(),
            std::io::ErrorKind::ConnectionReset
                | std::io::ErrorKind::BrokenPipe
                | std::io::ErrorKind::ConnectionAborted
        );
    }
    false
}

/// Shared HTTP client for discovery and utility requests.
/// Reused across all calls to avoid per-request client construction overhead.
static HTTP_CLIENT: LazyLock<reqwest::Client> = LazyLock::new(|| {
    reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .danger_accept_invalid_certs(true)
        .gzip(true)
        .pool_max_idle_per_host(0)
        .build()
        .expect("Failed to initialize shared HTTP client")
});

#[tracing::instrument(level = "debug")]
pub async fn send_rpc_command(ip: &IpAddr, command: &'static str) -> Option<serde_json::Value> {
    let mut stream = tokio::net::TcpStream::connect(format!("{ip}:4028"))
        .await
        .map_err(|_| tracing::debug!("failed to connect to {ip} rpc"))
        .ok()?;

    let command = format!("{{\"command\":\"{command}\"}}");
    if let Err(err) = stream.write_all(command.as_bytes()).await {
        tracing::debug!("failed to write command to {ip}: {err:?}");
        return None;
    }

    let mut buffer = Vec::new();
    if let Err(err) = stream.read_to_end(&mut buffer).await {
        tracing::debug!("failed to read response from {ip}: {err:?}");
        return None;
    }

    let response = String::from_utf8_lossy(&buffer)
        .into_owned()
        .replace('\0', "");
    tracing::trace!("got response from miner: {response}");

    parse_rpc_result(&response)
}

#[tracing::instrument(level = "debug")]
pub async fn send_web_command(
    ip: &IpAddr,
    command: &'static str,
) -> Option<(String, HeaderMap, StatusCode)> {
    let data = HTTP_CLIENT
        .get(format!("http://{ip}{command}"))
        .send()
        .await
        .map_err(|_| tracing::debug!("failed to connect to {ip} web"))
        .ok()?;

    let headers = data.headers().clone();
    let status = data.status();
    let text = data
        .text()
        .await
        .map_err(|_| tracing::debug!("received no response data from miner"))
        .ok()?;

    tracing::trace!("got response from miner: {text}");
    Some((text, headers, status))
}

#[tracing::instrument(level = "debug")]
pub async fn send_graphql_command(ip: &IpAddr, command: &'static str) -> Option<serde_json::Value> {
    let query = json!({ "query": command });

    let response = HTTP_CLIENT
        .post(format!("http://{}/graphql", ip))
        .header("Content-Type", "application/json")
        .json(&query)
        .send()
        .await
        .ok()?;

    response.json().await.ok()?
}

#[tracing::instrument(level = "debug")]
fn parse_rpc_result(response: &str) -> Option<serde_json::Value> {
    // Fix for WM V1, can have newlines in version which breaks the json parser
    let response = response.replace("\n", "");
    let parsed: Result<serde_json::Value, _> = serde_json::from_str(&response);
    let success_codes = ["S", "I"];

    match parsed.ok() {
        Some(data) => {
            let command_status_generic = data["STATUS"][0]["STATUS"].as_str();
            let command_status_whatsminer = data["STATUS"].as_str();
            let command_status = command_status_generic.or(command_status_whatsminer);

            match command_status {
                Some(status) => {
                    if success_codes.contains(&status) {
                        tracing::trace!("found success code from miner: {status}");
                        Some(data)
                    } else {
                        tracing::debug!("got error status from miner: {status}");
                        None
                    }
                }
                None => {
                    tracing::debug!("could not find result status");
                    None
                }
            }
        }
        None => {
            tracing::debug!("failed to parse response");
            None
        }
    }
}
