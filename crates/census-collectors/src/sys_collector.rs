use std::time::Duration;
use serde_json::{json, Value};
use anyhow::Result;
use census_core::{Collector, ReadExt};

pub struct SysCollector;

impl Collector for SysCollector {
    fn id(&self) -> &'static str {
        "sys"
    }

    fn interval(&self) -> Duration {
        Duration::from_secs(10)
    }

    async fn collect(&self) -> Result<Value> {
        let cpu_brand = read_cpu_brand();
        let (cores_physical, cores_logical) = read_cpu_cores();
        let board = read_board();
        let os_name = read_os_name();
        let wm = std::env::var("XDG_CURRENT_DESKTOP")
            .or_else(|_| std::env::var("DESKTOP_SESSION"))
            .unwrap_or_else(|_| "unknown".into());
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "unknown".into());
        let distro_icon = get_distro_icon();

        Ok(json!({
            "cpu_brand": cpu_brand,
            "cores_physical": cores_physical,
            "cores_logical": cores_logical,
            "board": board,
            "os_name": os_name,
            "wm": wm,
            "shell": shell,
            "distro_icon": distro_icon,
        }))
    }
}

fn read_cpu_brand() -> String {
    let cpuinfo = "/proc/cpuinfo".read_string();
    for line in cpuinfo.lines() {
        if let Some(val) = line.strip_prefix("model name\t:") {
            return clean_cpu_name(val.trim());
        }
    }
    "Unknown CPU".into()
}

fn clean_cpu_name(name: &str) -> String {
    let name = if let Some(pos) = name.find("-Core") {
        let before = &name[..pos];
        if let Some(space_pos) = before.rfind(' ') {
            name[..space_pos].to_string()
        } else {
            before.to_string()
        }
    } else {
        name.to_string()
    };
    let name = name.trim_end_matches(" Processor").to_string();
    name.trim_end_matches(" processor").to_string()
}

fn read_cpu_cores() -> (usize, usize) {
    let cpuinfo = "/proc/cpuinfo".read_string();
    let logical = cpuinfo.lines()
        .filter(|l| l.starts_with("processor"))
        .count();
    let physical = cpuinfo.lines()
        .filter(|l| l.starts_with("core id"))
        .filter_map(|l| l.split(':').nth(1))
        .map(|s| s.trim())
        .collect::<std::collections::HashSet<_>>()
        .len();
    (physical, logical)
}

fn read_board() -> String {
    let vendor = "/sys/devices/virtual/dmi/id/board_vendor".read_string();
    let board = "/sys/devices/virtual/dmi/id/board_name".read_string();
    let mut b = format!("{} {}", vendor, board).trim().to_string();

    let prefixes = [
        "Micro-Star International Co., Ltd. ",
        "ASUSTeK Computer INC. ",
        "Gigabyte Technology Co., Ltd. ",
        "ASRock ",
        "EVGA ",
        "MSI ",
    ];
    for p in &prefixes {
        if let Some(rest) = b.strip_prefix(p) {
            b = rest.to_string();
            break;
        }
    }
    b
}

fn read_os_name() -> String {
    let os_release = "/etc/os-release".read_string();
    for line in os_release.lines() {
        if let Some(val) = line.strip_prefix("PRETTY_NAME=") {
            return val.trim_matches('"').to_string();
        }
    }
    "Linux".into()
}

fn get_distro_icon() -> String {
    let lower = "/etc/os-release".read_string().to_lowercase();
    if lower.contains("arch") { "\u{f303}" }
    else if lower.contains("ubuntu") { "\u{f31b}" }
    else if lower.contains("fedora") { "\u{f30a}" }
    else if lower.contains("debian") { "\u{f306}" }
    else if lower.contains("manjaro") { "\u{f312}" }
    else if lower.contains("centos") { "\u{f304}" }
    else if lower.contains("gentoo") { "\u{f30d}" }
    else if lower.contains("nixos") { "\u{f313}" }
    else if lower.contains("void") { "\u{f32e}" }
    else if lower.contains("alpine") { "\u{f300}" }
    else if lower.contains("opensuse") || lower.contains("suse") { "\u{f314}" }
    else if lower.contains("mint") { "\u{f30e}" }
    else if lower.contains("pop") || lower.contains("_os") { "\u{f32a}" }
    else if lower.contains("garuda") { "\u{f337}" }
    else if lower.contains("endeavour") { "\u{f322}" }
    else if lower.contains("artix") { "\u{f31f}" }
    else if lower.contains("kali") { "\u{f327}" }
    else if lower.contains("rocky") { "\u{f32b}" }
    else if lower.contains("alma") { "\u{f31d}" }
    else if lower.contains("deepin") { "\u{f321}" }
    else { "\u{f31a}" }.into()
}
