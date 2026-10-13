use anyhow::Context;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};

/// One canned HTTP response: status line, extra headers and body.
pub(crate) struct MockResponse {
    pub(crate) status: &'static str,
    pub(crate) headers: &'static [&'static str],
    pub(crate) body: &'static str,
}

impl MockResponse {
    pub(crate) fn ok(body: &'static str) -> Self {
        Self {
            status: "200 OK",
            headers: &[],
            body,
        }
    }
}

pub(crate) struct MockSequenceServer {
    pub(crate) port: u16,
    pub(crate) task: tokio::task::JoinHandle<anyhow::Result<Vec<String>>>,
}

async fn read_http_request(socket: &mut TcpStream) -> anyhow::Result<String> {
    let mut request = Vec::new();
    let mut chunk = [0_u8; 1024];

    loop {
        if let Some(header_end) = request
            .windows(4)
            .position(|window| window == b"\r\n\r\n")
            .map(|position| position + 4)
        {
            let headers = std::str::from_utf8(&request[..header_end])?;
            let content_length = headers
                .lines()
                .filter_map(|line| line.split_once(':'))
                .find_map(|(name, value)| {
                    name.eq_ignore_ascii_case("content-length")
                        .then(|| value.trim().parse::<usize>().ok())
                        .flatten()
                })
                .unwrap_or_default();
            if request.len() >= header_end + content_length {
                break;
            }
        }

        let bytes_read = socket.read(&mut chunk).await?;
        if bytes_read == 0 {
            break;
        }
        request.extend_from_slice(&chunk[..bytes_read]);
    }

    Ok(String::from_utf8(request)?)
}

/// Serve the given responses to consecutive connections and return the
/// requests that were received.
pub(crate) async fn mock_sequence_server(
    responses: Vec<MockResponse>,
) -> anyhow::Result<MockSequenceServer> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let port = listener.local_addr()?.port();
    let task = tokio::spawn(async move {
        let mut requests = Vec::with_capacity(responses.len());
        for response in responses {
            let (mut socket, _) =
                tokio::time::timeout(std::time::Duration::from_secs(2), listener.accept())
                    .await
                    .context("timed out waiting for HTTP request")??;
            requests.push(read_http_request(&mut socket).await?);

            let mut head = format!(
                "HTTP/1.1 {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n",
                response.status,
                response.body.len()
            );
            for header in response.headers {
                head.push_str(header);
                head.push_str("\r\n");
            }
            socket
                .write_all(format!("{head}\r\n{}", response.body).as_bytes())
                .await?;
        }
        Ok(requests)
    });

    Ok(MockSequenceServer { port, task })
}

pub(crate) fn request_body(request: &str) -> anyhow::Result<serde_json::Value> {
    let (_, body) = request
        .split_once("\r\n\r\n")
        .context("HTTP request did not contain a body separator")?;
    Ok(serde_json::from_str(body)?)
}
