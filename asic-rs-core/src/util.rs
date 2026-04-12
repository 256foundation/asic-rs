use std::{net::IpAddr, sync::LazyLock, time::Duration};

use reqwest::{StatusCode, header::HeaderMap};
use serde_json::json;
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt},
    net::{TcpStream, ToSocketAddrs},
};

use crate::errors::RPCError;

/// Default read timeout for RPC stream responses.
pub const DEFAULT_RPC_TIMEOUT: Duration = Duration::from_secs(5);

/// Returns true if the error is an expected transient failure from a
/// privileged write — timeout or connection drop. These indicate the miner
/// received and applied the command but didn't respond in time.
pub fn is_expected_write_error(err: &anyhow::Error) -> bool {
    err.downcast_ref::<RPCError>()
        .is_some_and(|e| e.is_transient())
}

/// Shared HTTP client for discovery and utility requests.
/// Reused across all calls to avoid per-request client construction overhead.
static HTTP_CLIENT: LazyLock<reqwest::Client> = LazyLock::new(|| {
    reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .danger_accept_invalid_certs(true)
        .gzip(true)
        .timeout(DEFAULT_RPC_TIMEOUT)
        .pool_max_idle_per_host(0)
        .build()
        .expect("Failed to initialize shared HTTP client")
});

/// Connect to a miner TCP endpoint with a bounded timeout.
pub async fn connect_tcp_stream<A>(addr: A, timeout: Duration) -> anyhow::Result<TcpStream>
where
    A: ToSocketAddrs,
{
    tokio::time::timeout(timeout, TcpStream::connect(addr))
        .await
        .map_err(|_| RPCError::ConnectionTimeout)?
        .map_err(RPCError::from)
        .map_err(Into::into)
}

/// Read a complete RPC response from a stream.
///
/// Miners typically terminate responses with `\0` or `\n` but keep the TCP
/// connection open, so `read_to_end` would block forever. This reads in
/// chunks and stops when a terminator is found, the stream closes, or the
/// timeout expires (e.g. when a miner reboots mid-response).
pub async fn read_stream_response(
    stream: &mut (impl AsyncRead + Unpin),
    timeout: Duration,
) -> anyhow::Result<String> {
    tokio::time::timeout(timeout, async {
        let mut response = String::new();
        let mut buffer = [0u8; 8192];

        loop {
            let bytes_read = stream.read(&mut buffer).await.map_err(RPCError::from)?;
            if bytes_read == 0 {
                break;
            }

            let chunk = String::from_utf8_lossy(&buffer[..bytes_read]);
            response.push_str(&chunk);

            if response.contains('\0') || response.ends_with('\n') {
                break;
            }
        }

        Ok(response.trim_end_matches(['\0', '\n']).to_owned())
    })
    .await
    .map_err(|_| RPCError::ReadTimeout)?
}

/// Read exactly `buf.len()` bytes from a stream with a timeout.
pub async fn read_exact_with_timeout(
    stream: &mut (impl AsyncRead + Unpin),
    buf: &mut [u8],
    timeout: Duration,
) -> anyhow::Result<()> {
    tokio::time::timeout(timeout, stream.read_exact(buf))
        .await
        .map_err(|_| RPCError::ReadTimeout)?
        .map_err(RPCError::from)?;
    Ok(())
}

/// Write a complete RPC request with a timeout.
pub async fn write_all_with_timeout(
    stream: &mut (impl AsyncWrite + Unpin),
    buf: &[u8],
    timeout: Duration,
) -> anyhow::Result<()> {
    tokio::time::timeout(timeout, stream.write_all(buf))
        .await
        .map_err(|_| RPCError::WriteTimeout)?
        .map_err(RPCError::from)?;
    Ok(())
}

#[tracing::instrument(level = "debug")]
pub async fn send_rpc_command(ip: &IpAddr, command: &'static str) -> Option<serde_json::Value> {
    let mut stream = connect_tcp_stream((*ip, 4028), DEFAULT_RPC_TIMEOUT)
        .await
        .map_err(|_| tracing::debug!("failed to connect to {ip} rpc"))
        .ok()?;

    let command = format!("{{\"command\":\"{command}\"}}");
    if let Err(err) =
        write_all_with_timeout(&mut stream, command.as_bytes(), DEFAULT_RPC_TIMEOUT).await
    {
        tracing::debug!("failed to write command to {ip}: {err:?}");
        return None;
    }

    let response = read_stream_response(&mut stream, DEFAULT_RPC_TIMEOUT).await;
    let _ = stream.shutdown().await;
    let response = match response {
        Ok(r) => r,
        Err(err) => {
            tracing::debug!("failed to read response from {ip}: {err:?}");
            return None;
        }
    };
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::errors::RPCError;
    use tokio::io::AsyncWriteExt;

    #[tokio::test]
    async fn null_terminated_response() {
        // Arrange
        let (mut writer, mut reader) = tokio::io::duplex(8192);
        tokio::spawn(async move {
            writer.write_all(b"{\"STATUS\":\"S\"}\0").await.unwrap();
        });

        // Act
        let result = read_stream_response(&mut reader, Duration::from_secs(5))
            .await
            .unwrap();

        // Assert
        assert_eq!(result, "{\"STATUS\":\"S\"}");
    }

    #[tokio::test]
    async fn newline_terminated_response() {
        // Arrange
        let (mut writer, mut reader) = tokio::io::duplex(8192);
        tokio::spawn(async move {
            writer.write_all(b"{\"STATUS\":\"S\"}\n").await.unwrap();
        });

        // Act
        let result = read_stream_response(&mut reader, Duration::from_secs(5))
            .await
            .unwrap();

        // Assert
        assert_eq!(result, "{\"STATUS\":\"S\"}");
    }

    #[tokio::test]
    async fn multi_chunk_response() {
        // Arrange
        let (mut writer, mut reader) = tokio::io::duplex(64);
        tokio::spawn(async move {
            writer.write_all(b"{\"STATUS\":").await.unwrap();
            writer.write_all(b"\"S\"}\0").await.unwrap();
        });

        // Act
        let result = read_stream_response(&mut reader, Duration::from_secs(5))
            .await
            .unwrap();

        // Assert
        assert_eq!(result, "{\"STATUS\":\"S\"}");
    }

    #[tokio::test]
    async fn empty_response_on_stream_close() {
        // Arrange
        let (writer, mut reader) = tokio::io::duplex(8192);
        drop(writer);

        // Act
        let result = read_stream_response(&mut reader, Duration::from_secs(5))
            .await
            .unwrap();

        // Assert
        assert_eq!(result, "");
    }

    #[tokio::test]
    async fn both_terminators_trimmed() {
        // Arrange
        let (mut writer, mut reader) = tokio::io::duplex(8192);
        tokio::spawn(async move {
            writer.write_all(b"{\"STATUS\":\"S\"}\0\n").await.unwrap();
        });

        // Act
        let result = read_stream_response(&mut reader, Duration::from_secs(5))
            .await
            .unwrap();

        // Assert
        assert_eq!(result, "{\"STATUS\":\"S\"}");
    }

    #[tokio::test]
    async fn read_timeout_fires() {
        // Arrange — duplex with no data written, simulating a miner that rebooted
        let (_writer, mut reader) = tokio::io::duplex(8192);

        // Act
        let result = read_stream_response(&mut reader, Duration::from_millis(100)).await;

        // Assert
        let err = result.unwrap_err();
        assert!(
            err.downcast_ref::<RPCError>()
                .is_some_and(|e| matches!(e, RPCError::ReadTimeout))
        );
    }
}
