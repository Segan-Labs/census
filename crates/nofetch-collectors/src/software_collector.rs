use std::time::Duration;
use serde_json::{json, Value};
use anyhow::Result;
use nofetch_core::{Collector, ReadExt, fs::{exists, read_dir_dirs, read_dir_count}};

pub struct SoftwareCollector;

impl Collector for SoftwareCollector {
    fn id(&self) -> &'static str {
        "software"
    }

    fn interval(&self) -> Duration {
        Duration::from_secs(60)
    }

    async fn collect(&self) -> Result<Value> {
        Ok(json!({
            "pacman": read_dir_dirs("/var/lib/pacman/local"),
            "aur": count_aur(),
            "cargo": read_dir_count(&cargo_bin()),
            "node": count_node(),
            "flatpak": count_flatpak(),
            "snap": count_snap(),
            "rust_ver": get_rust_version(),
        }))
    }
}

fn cargo_bin() -> String {
    let home = std::env::var("HOME").unwrap_or_default();
    format!("{}/.cargo/bin", home)
}

fn count_aur() -> usize {
    0
}

fn count_node() -> usize {
    let mut count = 0;
    if exists("/usr/lib/node_modules") {
        count += read_dir_dirs("/usr/lib/node_modules");
    }
    let home = std::env::var("HOME").unwrap_or_default();
    let npm_global = format!("{}/.npm-global/lib/node_modules", home);
    if exists(&npm_global) {
        count += read_dir_dirs(&npm_global);
    }
    count
}

fn count_flatpak() -> usize {
    let mut count = 0;
    if exists("/var/lib/flatpak/app") {
        count += read_dir_dirs("/var/lib/flatpak/app");
    }
    let home = std::env::var("HOME").unwrap_or_default();
    let user_flatpak = format!("{}/.local/share/flatpak/app", home);
    if exists(&user_flatpak) {
        count += read_dir_dirs(&user_flatpak);
    }
    count
}

fn count_snap() -> usize {
    if exists("/snap") {
        std::fs::read_dir("/snap")
            .map(|r| {
                r.filter_map(|e| e.ok())
                    .filter(|e| {
                        let name = e.file_name().to_string_lossy().to_string();
                        e.path().is_dir() && name != "snapd" && !name.starts_with('.')
                    })
                    .count()
            })
            .unwrap_or(0)
    } else {
        0
    }
}

fn get_rust_version() -> Option<String> {
    let toolchain = "rust-toolchain".read_string();
    if !toolchain.is_empty() {
        return Some(toolchain);
    }

    if let Ok(content) = std::fs::read_to_string("rust-toolchain.toml") {
        if let Some(channel) = content.lines().find(|l| l.contains("channel")) {
            let ver = channel.split('=').nth(1)?.trim().trim_matches('"').trim_matches('\'').to_string();
            if !ver.is_empty() {
                return Some(ver);
            }
        }
    }
    None
}
