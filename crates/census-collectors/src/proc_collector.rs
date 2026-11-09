use std::time::Duration;
use serde_json::{json, Value};
use anyhow::Result;
use census_core::{Collector, ReadExt};

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

fn check_reboot_required() -> bool {
    std::path::Path::new("/var/run/reboot-required").exists()
        || std::path::Path::new("/run/reboot-required").exists()
}

/// Real, writable-looking filesystems from /proc/mounts, one per device, sized with statvfs.
fn read_disks() -> Vec<serde_json::Value> {
    const FS: [&str; 7] = ["ext4", "btrfs", "xfs", "f2fs", "zfs", "ntfs3", "exfat"];
    let mut seen = std::collections::HashSet::new();
    let mut disks = Vec::new();
    for line in "/proc/mounts".read_string().lines() {
        let mut p = line.split_whitespace();
        let (Some(dev), Some(mount), Some(fs)) = (p.next(), p.next(), p.next()) else { continue };
        if !FS.contains(&fs) || !dev.starts_with('/') || mount.starts_with("/boot") || mount.starts_with("/var/lib/docker") {
            continue;
        }
        if !seen.insert(dev.to_string()) {
            continue;
        }
        let mount = mount.replace("\\040", " ");
        let Ok(c) = std::ffi::CString::new(mount.as_str()) else { continue };
        let mut st: libc::statvfs = unsafe { std::mem::zeroed() };
        if unsafe { libc::statvfs(c.as_ptr(), &mut st) } != 0 {
            continue;
        }
        let total = st.f_blocks as f64 * st.f_frsize as f64;
        let free = st.f_bfree as f64 * st.f_frsize as f64;
        if total <= 0.0 {
            continue;
        }
        let gib = 1073741824.0;
        let label = if mount == "/" {
            "/".to_string()
        } else {
            mount.rsplit('/').next().unwrap_or(&mount).to_string()
        };
        disks.push(json!({
            "label": label,
            "mount": mount,
            "used_gib": ((total - free) / gib * 100.0).round() / 100.0,
            "total_gib": (total / gib * 100.0).round() / 100.0,
        }));
    }
    disks
}
