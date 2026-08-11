use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;
use anyhow::{Result, Context};
use nofetch_core::Snapshot;

pub async fn get_snapshot(socket_path: &str) -> Result<Snapshot> {
    let mut stream = UnixStream::connect(socket_path)
        .await
        .context("Failed to connect to nofetchd")?;

    // Send GET /snapshot request
    let request = "GET /snapshot\r\n";
    stream.write_all(request.as_bytes()).await?;

    // Read response header first
    let mut header_buf = Vec::new();
    let mut temp = [0u8; 1];
    let mut header_done = false;

    // Read until we find \r\n\r\n
    while !header_done {
        let n = stream.read(&mut temp).await?;
        if n == 0 {
            break;
        }
        header_buf.push(temp[0]);
        if header_buf.len() >= 4
            && header_buf[header_buf.len()-4..] == *b"\r\n\r\n"
        {
            header_done = true;
        }
    }

    let header = String::from_utf8(header_buf)?;

    // Parse Content-Length
    let content_length: usize = header
        .lines()
        .find(|l| l.to_lowercase().starts_with("content-length:"))
        .and_then(|l| l.split(':').nth(1))
        .and_then(|v| v.trim().parse().ok())
        .unwrap_or(0);

    if content_length == 0 {
        anyhow::bail!("No Content-Length in response");
    }

    // Read exactly content_length bytes
    let mut body = vec![0u8; content_length];
    stream.read_exact(&mut body).await?;

    let snapshot: Snapshot = serde_json::from_slice(&body)
        .context("Failed to parse snapshot JSON")?;
    Ok(snapshot)
}

pub fn is_daemon_running(socket_path: &str) -> bool {
    std::path::Path::new(socket_path).exists()
}
