use std::time::{Duration, Instant};
use std::fs;
use std::sync::Mutex;
use serde_json::{json, Value};
use anyhow::Result;
use nofetch_core::{Collector, fs::read_parse};

pub struct NetworkCollector {
    prev: Mutex<Option<(Instant, u64, u64)>>,
}

impl NetworkCollector {
    pub fn new() -> Self {
        Self {
            prev: Mutex::new(None),
        }
    }
}

impl Collector for NetworkCollector {
    fn id(&self) -> &'static str {
        "network"
    }

    fn interval(&self) -> Duration {
        Duration::from_secs(1)
    }

    async fn collect(&self) -> Result<Value> {
        let lan_ip = read_lan_ip();
        let tailnet_ip = read_tailnet_ip();
        let public_ip = read_public_ip();
        let (rx_bytes, tx_bytes, speed) = read_net_stats();

        // Calculate throughput
        let mut prev = self.prev.lock().unwrap();
        let (rx_rate, tx_rate) = if let Some((prev_time, prev_rx, prev_tx)) = *prev {
            let elapsed = prev_time.elapsed().as_secs_f64();
            if elapsed > 0.0 {
                let rx_delta = rx_bytes.saturating_sub(prev_rx);
                let tx_delta = tx_bytes.saturating_sub(prev_tx);
                (rx_delta as f64 / elapsed, tx_delta as f64 / elapsed)
            } else {
                (0.0, 0.0)
            }
        } else {
            (0.0, 0.0)
        };
        *prev = Some((Instant::now(), rx_bytes, tx_bytes));

        Ok(json!({
            "lan_ip": lan_ip,
            "public_ip": public_ip,
            "tailnet_ip": tailnet_ip,
            "gateway": null,
            "rx_bytes": rx_bytes,
            "tx_bytes": tx_bytes,
            "speed_mbps": speed,
            "rx_rate": rx_rate,
            "tx_rate": tx_rate,
        }))
    }
}

fn read_lan_ip() -> Option<String> {
    let mut active_iface = None;
    if let Ok(entries) = fs::read_dir("/sys/class/net") {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name == "lo" || name.starts_with("docker") || name.starts_with("br-")
                || name.starts_with("veth") || name.starts_with("virbr")
                || name.starts_with("tailscale") {
                continue;
            }
            let operstate: String = read_parse(entry.path().join("operstate")).unwrap_or_default();
            if operstate == "up" {
                active_iface = Some(entry.path());
                break;
            }
        }
    }

    if let Some(_iface) = active_iface {
        if let Ok(content) = fs::read_to_string("/proc/net/fib_trie") {
            let lines: Vec<&str> = content.lines().collect();
            let mut candidates = Vec::new();
            for i in 0..lines.len() {
                if lines[i].contains("/32 host LOCAL") && i > 0 {
                    if let Some(ip) = lines[i-1].trim().strip_prefix("|-- ") {
                        let ip = ip.trim().to_string();
                        if !ip.is_empty() && !ip.starts_with("127.") && ip.contains('.') {
                            candidates.push(ip);
                        }
                    }
                }
            }
            for ip in &candidates {
                if ip.starts_with("192.168.") {
                    return Some(ip.clone());
                }
            }
            for ip in &candidates {
                if ip.starts_with("10.") {
                    return Some(ip.clone());
                }
            }
            for ip in &candidates {
                if !ip.starts_with("100.") {
                    return Some(ip.clone());
                }
            }
            candidates.first().cloned()
        } else {
            None
        }
    } else {
        if let Ok(content) = fs::read_to_string("/proc/net/fib_trie") {
            let lines: Vec<&str> = content.lines().collect();
            for i in 0..lines.len() {
                if lines[i].contains("/32 host LOCAL") && i > 0 {
                    if let Some(ip) = lines[i-1].trim().strip_prefix("|-- ") {
                        let ip = ip.trim().to_string();
                        if !ip.is_empty() && !ip.starts_with("127.") && ip.contains('.')
                            && ip.starts_with("192.168.") {
                            return Some(ip);
                        }
                    }
                }
            }
        }
        None
    }
}

fn read_tailnet_ip() -> Option<String> {
    if !std::path::Path::new("/sys/class/net/tailscale0").exists() {
        return None;
    }

    if let Ok(content) = fs::read_to_string("/proc/net/fib_trie") {
        let lines: Vec<&str> = content.lines().collect();
        for i in 0..lines.len() {
            if lines[i].contains("/32 host LOCAL") && i > 0 {
                if let Some(ip) = lines[i-1].trim().strip_prefix("|-- ") {
                    let ip = ip.trim().to_string();
                    if ip.starts_with("100.") && !ip.starts_with("100.64.") {
                        return Some(ip);
                    }
                }
            }
        }
    }
    None
}

fn read_public_ip() -> Option<String> {
    std::process::Command::new("curl")
        .args(["-s", "--max-time", "1", "ifconfig.me"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn read_net_stats() -> (u64, u64, u64) {
    let mut active_iface = None;
    if let Ok(entries) = fs::read_dir("/sys/class/net") {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name == "lo" || name.starts_with("docker") || name.starts_with("br-")
                || name.starts_with("veth") || name.starts_with("virbr")
                || name.starts_with("tailscale") {
                continue;
            }
            let operstate: String = read_parse(entry.path().join("operstate")).unwrap_or_default();
            if operstate == "up" {
                active_iface = Some(entry.path());
                break;
            }
        }
    }

    if let Some(path) = active_iface {
        let rx: u64 = read_parse(path.join("statistics/rx_bytes")).unwrap_or(0);
        let tx: u64 = read_parse(path.join("statistics/tx_bytes")).unwrap_or(0);
        let speed: u64 = read_parse(path.join("speed")).unwrap_or(0);
        (rx, tx, speed)
    } else {
        (0, 0, 0)
    }
}
