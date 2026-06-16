use tokio::net::TcpListener;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// Waits for the OAuth callback from Spotify on the local redirect port.
///
/// Listens on `127.0.0.1:8080` for a single HTTP request, extracts the
/// `code` and `state` query parameters from the callback URL, and responds
/// with a simple success page.
pub async fn wait_for_callback() -> Result<(String, String), Box<dyn std::error::Error + Send + Sync>> {
    let listener = TcpListener::bind("127.0.0.1:8080").await?;
    let (mut stream, _) = listener.accept().await?;

    let mut buf = [0u8; 4096];
    let n = stream.read(&mut buf).await?;
    let request = String::from_utf8_lossy(&buf[..n]);

    // Parse: GET /callback?code=...&state=... HTTP/1.1
    let path = request.lines()
        .next().unwrap_or("")
        .split_whitespace().nth(1).unwrap_or("");
    let full_url = format!("http://127.0.0.1:8080{}", path);
    let parsed = url::Url::parse(&full_url)?;

    let code = parsed.query_pairs()
        .find(|(k, _)| k == "code")
        .map(|(_, v)| v.into_owned())
        .ok_or("No code in callback URL")?;
    let state = parsed.query_pairs()
        .find(|(k, _)| k == "state")
        .map(|(_, v)| v.into_owned())
        .unwrap_or_default();

    let response = "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\n\r\n\
                    <h1>Authorized! You can close this tab.</h1>";
    stream.write_all(response.as_bytes()).await?;

    Ok((code, state))
}