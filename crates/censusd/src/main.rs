use std::sync::{Arc, RwLock};

use census_collectors::SnapshotBuilder;
use census_core::Config;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixListener;
use tokio::signal::unix::{signal, SignalKind};

/// The reply for `GET /snapshot`, built once per refresh so a request only copies bytes.
async fn build_reply(builder: &SnapshotBuilder) -> Arc<Vec<u8>> {
    let json = serde_json::to_string(&builder.build().await).unwrap_or_default();
    let mut reply = format!("Content-Length: {}\r\n\r\n", json.len()).into_bytes();
    reply.extend_from_slice(json.as_bytes());
    Arc::new(reply)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let config = Config::load();
    let socket_path = config.daemon.socket_path.clone();

    let _ = std::fs::remove_file(&socket_path);
    let listener = UnixListener::bind(&socket_path)?;
    tracing::info!("censusd listening on {}", socket_path);

    let builder = Arc::new(SnapshotBuilder::new(&config));
    let shared = Arc::new(RwLock::new(build_reply(&builder).await));

    let interval = config.daemon.collector_interval_secs.max(1);
    {
        let shared = shared.clone();
        let builder = builder.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(interval)).await;
                let reply = build_reply(&builder).await;
                *shared.write().unwrap_or_else(|e| e.into_inner()) = reply;
            }
        });
    }

    let mut term = signal(SignalKind::terminate())?;
    loop {
        tokio::select! {
            _ = term.recv() => break,
            _ = tokio::signal::ctrl_c() => break,
            accepted = listener.accept() => {
                let (mut stream, _) = accepted?;
                let reply = shared.read().unwrap_or_else(|e| e.into_inner()).clone();
                tokio::spawn(async move {
                    let mut buffer = [0u8; 256];
                    let n = stream.read(&mut buffer).await.unwrap_or(0);
                    if buffer[..n].starts_with(b"GET /snapshot") {
                        let _ = stream.write_all(&reply).await;
                    }
                    let _ = stream.shutdown().await;
                });
            }
        }
    }
    let _ = std::fs::remove_file(&socket_path);
    Ok(())
}
