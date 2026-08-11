use std::time::Duration;
use std::path::Path;
use serde_json::{json, Value};
use anyhow::Result;
use nofetch_core::Collector;

pub struct DockerCollector;

impl Collector for DockerCollector {
    fn id(&self) -> &'static str {
        "docker"
    }

    fn interval(&self) -> Duration {
        Duration::from_secs(5)
    }

    async fn collect(&self) -> Result<Value> {
        let available = Path::new("/var/run/docker.sock").exists();
        Ok(json!({ "available": available }))
    }
}
