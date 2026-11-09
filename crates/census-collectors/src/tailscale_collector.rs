use std::time::Duration;
use std::fs;
use std::path::Path;
use serde_json::{json, Value};
use anyhow::Result;
use census_core::Collector;

pub struct TailscaleCollector;

impl Collector for TailscaleCollector {
    fn id(&self) -> &'static str {
        "tailscale"
    }

    fn interval(&self) -> Duration {
        Duration::from_secs(30)
    }

    async fn collect(&self) -> Result<Value> {
        let sock = std::env::var("XDG_RUNTIME_DIR")
            .map(|p| format!("{}/tailscaled.sock", p))
            .unwrap_or_else(|_| "/var/run/tailscale/tailscaled.sock".into());
        let available = Path::new(&sock).exists();

        if !available {
            return Ok(json!({
                "available": false,
                "ip": null,
                "status": "not running",
            }));
        }

        let ip = read_tailscale_ip();
        let status = read_tailscale_status();

        Ok(json!({
            "available": true,
            "ip": ip,
            "status": status,
        }))
    }
}

fn read_tailscale_ip() -> Option<String> {
    if let Ok(entries) = fs::read_dir("/sys/class/net") {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name == "tailscale0" {
                let operstate = fs::read_to_string(entry.path().join("operstate"))
                    .map(|s| s.trim().to_string())
                    .unwrap_or_default();
                if operstate == "up" {
                    if let Ok(content) = fs::read_to_string("/proc/net/fib_trie") {
                        let lines: Vec<&str> = content.lines().collect();
                        let mut found_ts = false;
                        for i in 0..lines.len() {
                            if lines[i].contains("tailscale0") {
                                found_ts = true;
                            }
                            if found_ts && lines[i].contains("/32 host LOCAL") && i > 0 {
                                if let Some(ip) = lines[i-1].trim().strip_prefix("|-- ") {
                                    let ip = ip.trim().to_string();
                                    if ip.starts_with("100.") {
                                        return Some(ip);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    None
}

fn read_tailscale_status() -> String {
    fs::read_to_string("/sys/class/net/tailscale0/operstate")
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|_| "unknown".into())
}
