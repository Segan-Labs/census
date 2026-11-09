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
            "host": read_host(),
            "cpu_max_mhz": read_cpu_max_mhz(),
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

/// The CPU's top frequency in MHz: cpufreq's `cpuinfo_max_freq` (kHz), else the highest `cpu MHz` in /proc/cpuinfo.
fn read_cpu_max_mhz() -> Option<u32> {
    let khz: u64 = "/sys/devices/system/cpu/cpu0/cpufreq/cpuinfo_max_freq".read_u64();
    if khz > 0 {
        return Some((khz / 1000) as u32);
    }
    "/proc/cpuinfo"
        .read_string()
        .lines()
        .filter_map(|l| l.strip_prefix("cpu MHz"))
        .filter_map(|l| l.split(':').nth(1)?.trim().parse::<f64>().ok())
        .fold(None, |m: Option<f64>, v| Some(m.map_or(v, |m| m.max(v))))
        .map(|v| v as u32)
}

fn read_host() -> Option<String> {
    let d = |n: &str| format!("/sys/class/dmi/id/{}", n).read_string();
    host_name(&d("sys_vendor"), &d("board_vendor"), &d("product_name"), &d("product_version"), &d("board_name"))
}

const PLACEHOLDERS: [&str; 9] = [
    "to be filled", "system product name", "system version", "default string", "not applicable",
    "o.e.m.", "none", "not specified", "unknown",
];

fn usable(s: &str) -> bool {
    let l = s.trim().to_lowercase();
    !l.is_empty() && !PLACEHOLDERS.iter().any(|p| l.starts_with(p)) && !l.starts_with("1.0") && !l.starts_with("rev ")
}

/// A name with words in it ("MPG B550 GAMING PLUS", "ThinkPad X1 Carbon") rather than a model code ("MS-7C56", "20XW0055US").
fn friendly(s: &str) -> bool {
    s.contains(' ') && s.chars().any(|c| c.is_alphabetic())
}

fn short_vendor(v: &str) -> String {
    let l = v.to_lowercase();
    for (key, short) in [
        ("micro-star", "MSI"), ("asustek", "ASUS"), ("gigabyte", "Gigabyte"), ("asrock", "ASRock"),
        ("lenovo", "Lenovo"), ("dell", "Dell"), ("hewlett", "HP"), ("samsung", "Samsung"), ("apple", "Apple"),
    ] {
        if l.contains(key) {
            return short.to_string();
        }
    }
    v.trim_end_matches(['.', ',']).trim_end_matches("Inc").trim_end_matches("Co").trim().trim_end_matches(',').to_string()
}

/// The product name that a person would say: a friendly product version or name first (laptops), then the board name
/// (desktops, where the product name is only a code), then whatever code there is. Firmware placeholders are skipped.
pub fn host_name(sys_vendor: &str, board_vendor: &str, product_name: &str, product_version: &str, board_name: &str) -> Option<String> {
    let cands = [product_version, product_name, board_name];
    let name = cands
        .iter()
        .find(|c| usable(c) && friendly(c))
        .or_else(|| cands.iter().skip(1).find(|c| usable(c)))?
        .trim()
        .to_string();
    let vendor = [sys_vendor, board_vendor].into_iter().find(|v| usable(v)).map(short_vendor).unwrap_or_default();
    if vendor.is_empty() || name.to_lowercase().starts_with(&vendor.to_lowercase()) {
        Some(name)
    } else {
        Some(format!("{} {}", vendor, name))
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn desktop_board_beats_the_model_code() {
        assert_eq!(
            host_name("Micro-Star International Co., Ltd.", "Micro-Star International Co., Ltd.", "MS-7C56", "1.0", "MPG B550 GAMING PLUS (MS-7C56)").as_deref(),
            Some("MSI MPG B550 GAMING PLUS (MS-7C56)")
        );
    }

    #[test]
    fn laptop_version_is_the_product_name() {
        assert_eq!(
            host_name("LENOVO", "LENOVO", "20XW0055US", "ThinkPad X1 Carbon Gen 9", "20XW0055US").as_deref(),
            Some("Lenovo ThinkPad X1 Carbon Gen 9")
        );
    }

    #[test]
    fn placeholders_are_skipped() {
        assert_eq!(host_name("To Be Filled By O.E.M.", "ASUSTeK COMPUTER INC.", "System Product Name", "System Version", "PRIME B450M-A").as_deref(), Some("ASUS PRIME B450M-A"));
        assert_eq!(host_name("To be filled by O.E.M.", "", "To be filled by O.E.M.", "", "To be filled by O.E.M."), None);
    }
}
