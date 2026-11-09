use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::time::Duration;

use anyhow::{Context, Result};
use census_core::Snapshot;

/// Ask `censusd` for the latest snapshot. Blocking and tiny on purpose: this is the whole cost of a render.
pub fn get_snapshot(socket_path: &str) -> Result<Snapshot> {
    let mut stream = UnixStream::connect(socket_path).context("Failed to connect to censusd")?;
    stream.set_read_timeout(Some(Duration::from_secs(2)))?;
    stream.set_write_timeout(Some(Duration::from_secs(2)))?;
    stream.write_all(b"GET /snapshot\r\n")?;

    // The reply is "Content-Length: N\r\n\r\n" and N bytes of JSON, then the daemon closes.
    let mut reply = Vec::with_capacity(8192);
    stream.read_to_end(&mut reply)?;
    let split = reply
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .context("Malformed reply from censusd")?;
    let header = std::str::from_utf8(&reply[..split])?;
    let len: usize = header
        .lines()
        .find(|l| l.to_lowercase().starts_with("content-length:"))
        .and_then(|l| l.split(':').nth(1))
        .and_then(|v| v.trim().parse().ok())
        .unwrap_or(0);
    anyhow::ensure!(len > 0, "No Content-Length in response");
    let body = reply
        .get(split + 4..split + 4 + len)
        .context("Short reply from censusd")?;
    serde_json::from_slice(body).context("Failed to parse snapshot JSON")
}
