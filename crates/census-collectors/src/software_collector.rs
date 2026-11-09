use std::time::Duration;
use serde_json::{json, Value};
use anyhow::Result;
use census_core::{Collector, ReadExt, fs::{exists, read_dir_dirs, read_dir_count}};

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
            "cargo": read_dir_count(cargo_bin()),
            "node": count_node(),
            "flatpak": count_flatpak(),
            "appimage": count_appimages(),
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

/// Installed apps (not runtimes), system-wide plus the user's own.
fn count_flatpak() -> usize {
    let home = std::env::var("HOME").unwrap_or_default();
    read_dir_dirs("/var/lib/flatpak/app") + read_dir_dirs(format!("{}/.local/share/flatpak/app", home))
}

/// `*.AppImage` files in the places people keep them. Backups like `x.AppImage.old` do not count.
fn count_appimages() -> usize {
    let home = std::env::var("HOME").unwrap_or_default();
    ["Applications", ".local/bin", "AppImages"]
        .iter()
        .map(|d| count_appimages_in(std::path::Path::new(&home).join(d)))
        .sum()
}

pub fn count_appimages_in<P: AsRef<std::path::Path>>(dir: P) -> usize {
    std::fs::read_dir(dir)
        .map(|r| {
            r.filter_map(|e| e.ok())
                .filter(|e| e.file_name().to_string_lossy().to_lowercase().ends_with(".appimage") && e.path().is_file())
                .count()
        })
        .unwrap_or(0)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_appimages_and_apps_only() {
        let d = std::env::temp_dir().join(format!("census-ai-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        for f in ["a.AppImage", "b.appimage", "c.AppImage.old", "d.txt"] {
            std::fs::write(d.join(f), "").unwrap();
        }
        assert_eq!(count_appimages_in(&d), 2);
        assert_eq!(count_appimages_in(d.join("missing")), 0);
        let apps = d.join("app");
        std::fs::create_dir_all(apps.join("org.a.One")).unwrap();
        std::fs::create_dir_all(apps.join("org.b.Two")).unwrap();
        assert_eq!(read_dir_dirs(&apps), 2);
        std::fs::remove_dir_all(&d).unwrap();
    }
}
