use anyhow::Context;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};

pub(crate) struct MockHttpServer {
    pub(crate) port: u16,
    pub(crate) task: tokio::task::JoinHandle<anyhow::Result<String>>,
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

pub(crate) async fn mock_http_server(
    status: u16,
    reason: &'static str,
    body: &'static str,
) -> anyhow::Result<MockHttpServer> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let port = listener.local_addr()?.port();
    let task = tokio::spawn(async move {
        let (mut socket, _) =
            tokio::time::timeout(std::time::Duration::from_secs(2), listener.accept())
                .await
                .context("timed out waiting for HTTP request")??;
        let request = read_http_request(&mut socket).await?;
        let response = format!(
            "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        socket.write_all(response.as_bytes()).await?;
        Ok(request)
    });

    Ok(MockHttpServer { port, task })
}
