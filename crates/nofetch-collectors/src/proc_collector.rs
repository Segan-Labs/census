use std::time::Duration;
use serde_json::{json, Value};
use anyhow::Result;
use nofetch_core::{Collector, ReadExt};

pub struct ProcCollector;

impl Collector for ProcCollector {
    fn id(&self) -> &'static str {
        "proc"
    }

    fn interval(&self) -> Duration {
        Duration::from_secs(1)
    }

    async fn collect(&self) -> Result<Value> {
        let hostname = "/proc/sys/kernel/hostname".read_string();
        let kernel = "/proc/sys/kernel/osrelease".read_string();

        let uptime_secs = {
            let content = "/proc/uptime".read_string();
            content.split_whitespace()
                .next()
                .and_then(|s| s.parse::<f64>().ok())
                .map(|f| f as u64)
                .unwrap_or(0)
        };

        let load_avg = {
            let content = "/proc/loadavg".read_string();
            let parts: Vec<&str> = content.split_whitespace().collect();
            if parts.len() >= 3 {
                [
                    parts[0].parse().unwrap_or(0.0),
                    parts[1].parse().unwrap_or(0.0),
                    parts[2].parse().unwrap_or(0.0),
                ]
            } else {
                [0.0, 0.0, 0.0]
            }
        };

        let (ram_total, ram_used) = read_meminfo();
        let (swap_total, swap_used) = read_swapinfo();
        let os_age = read_os_age();
        let disks = read_disks();
        let updates = count_updates();
        let needs_restart = check_reboot_required();

        Ok(json!({
            "hostname": hostname,
            "kernel": kernel,
            "uptime_secs": uptime_secs,
            "load_avg": load_avg,
            "ram_total_gib": ram_total,
            "ram_used_gib": ram_used,
            "swap_total_gib": swap_total,
            "swap_used_gib": swap_used,
            "os_age_days": os_age,
            "disks": disks,
            "updates_available": updates,
            "needs_restart": needs_restart,
        }))
    }
}

fn read_meminfo() -> (f64, f64) {
    let content = "/proc/meminfo".read_string();
    let mut total = 0u64;
    let mut available = 0u64;

    for line in content.lines() {
        if let Some(val) = line.strip_prefix("MemTotal:") {
            total = val.split_whitespace().next()
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);
        } else if let Some(val) = line.strip_prefix("MemAvailable:") {
            available = val.split_whitespace().next()
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);
        }
    }

    let total_gib = total as f64 / 1024.0 / 1024.0;
    let used_gib = (total - available) as f64 / 1024.0 / 1024.0;
    (total_gib, used_gib)
}

fn read_swapinfo() -> (f64, f64) {
    let content = "/proc/meminfo".read_string();
    let mut total = 0u64;
    let mut free = 0u64;

    for line in content.lines() {
        if let Some(val) = line.strip_prefix("SwapTotal:") {
            total = val.split_whitespace().next()
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);
        } else if let Some(val) = line.strip_prefix("SwapFree:") {
            free = val.split_whitespace().next()
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);
        }
    }

    let total_gib = total as f64 / 1024.0 / 1024.0;
    let used_gib = (total - free) as f64 / 1024.0 / 1024.0;
    (total_gib, used_gib)
}

fn read_os_age() -> Option<u64> {
    std::fs::metadata("/etc/passwd")
        .ok()?
        .modified()
        .ok()?
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .map(|d| {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default();
            (now.as_secs() - d.as_secs()) / 86400
        })
}

fn count_updates() -> usize {
    // Check pacman updates
    if let Ok(output) = std::process::Command::new("pacman").args(["-Qu"]).output() {
        if let Ok(stdout) = String::from_utf8(output.stdout) {
            return stdout.lines().count();
        }
    }
    0
}

fn check_reboot_required() -> bool {
    std::path::Path::new("/var/run/reboot-required").exists()
        || std::path::Path::new("/run/reboot-required").exists()
}

fn read_disks() -> Vec<serde_json::Value> {
    let targets = [
        ("/", "/"),
        ("/Project", "Project"),
        ("/run/media/curl-segan/HDD", "HDD"),
        ("/run/media/curl-segan/DATA_STORAGE", "Home"),
    ];
    let mut disks = Vec::new();

    // Parse df output
    if let Ok(output) = std::process::Command::new("df").arg("-B1").output() {
        if let Ok(stdout) = String::from_utf8(output.stdout) {
            for line in stdout.lines().skip(1) {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() < 6 { continue; }
                let mount = parts[5];

                if let Some((_, label)) = targets.iter().find(|(m, _)| *m == mount) {
                    let total: u64 = parts[1].parse().unwrap_or(0);
                    let used: u64 = parts[2].parse().unwrap_or(0);
                    if total > 0 {
                        disks.push(json!({
                            "label": label,
                            "mount": mount,
                            "used_gib": (used as f64 / 1073741824.0 * 100.0).round() / 100.0,
                            "total_gib": (total as f64 / 1073741824.0 * 100.0).round() / 100.0,
                        }));
                    }
                }
            }
        }
    }

    disks
}
