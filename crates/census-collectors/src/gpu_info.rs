use std::fs;

use census_core::fs::read_string;
use serde_json::Value;

/// The marketing name from the system's pci.ids: "GA104 [GeForce RTX 3060 Ti]" becomes "GeForce RTX 3060 Ti".
pub fn pci_ids_name(vendor: &str, device: &str) -> Option<String> {
    let vendor = vendor.trim_start_matches("0x").to_lowercase();
    let device = device.trim_start_matches("0x").to_lowercase();
    let text = ["/usr/share/hwdata/pci.ids", "/usr/share/misc/pci.ids"]
        .iter()
        .find_map(|p| fs::read_to_string(p).ok())?;
    let mut in_vendor = false;
    let mut vendor_name = String::new();
    for line in text.lines() {
        if line.starts_with('#') || line.is_empty() {
            continue;
        }
        if !line.starts_with('\t') {
            if in_vendor {
                return None;
            }
            if let Some(rest) = line.strip_prefix(vendor.as_str()) {
                in_vendor = true;
                vendor_name = rest.trim().to_string();
            }
        } else if in_vendor && !line.starts_with("\t\t") {
            if let Some(rest) = line.trim_start().strip_prefix(device.as_str()) {
                let name = rest.trim();
                if let (Some(a), Some(b)) = (name.rfind('['), name.rfind(']')) {
                    if a < b {
                        return Some(name[a + 1..b].trim_end_matches(" Lite Hash Rate").to_string());
                    }
                }
                return Some(format!("{} {}", vendor_name, name));
            }
        }
    }
    None
}

/// The kernel driver of the first GPU with its version when the driver says one, and the Vulkan
/// API version its ICD file declares. Files only; nothing is run.
pub fn read_gpu_driver() -> (Option<String>, Option<String>) {
    let Ok(entries) = fs::read_dir("/sys/class/drm") else { return (None, None) };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.starts_with("card") || name.contains('-') {
            continue;
        }
        let Ok(link) = fs::read_link(entry.path().join("device/driver")) else { continue };
        let Some(driver) = link.file_name().map(|d| d.to_string_lossy().to_string()) else { continue };
        let (label, icd_hint) = match driver.as_str() {
            "nvidia" => ("NVIDIA", "nvidia"),
            "amdgpu" => ("amdgpu", "radeon"),
            "i915" | "xe" => ("Intel", "intel"),
            "nouveau" => ("nouveau", "nouveau"),
            other => (other, other),
        };
        let version = read_string(format!("/sys/module/{}/version", driver));
        let text = if version.is_empty() { label.to_string() } else { format!("{} {}", label, version) };
        return (Some(text), read_vulkan_version(icd_hint));
    }
    (None, None)
}

fn read_vulkan_version(hint: &str) -> Option<String> {
    for dir in ["/usr/share/vulkan/icd.d", "/etc/vulkan/icd.d"] {
        let Ok(entries) = fs::read_dir(dir) else { continue };
        for e in entries.flatten() {
            if !e.file_name().to_string_lossy().contains(hint) {
                continue;
            }
            let Ok(text) = fs::read_to_string(e.path()) else { continue };
            let Ok(v) = serde_json::from_str::<Value>(&text) else { continue };
            if let Some(api) = v["ICD"]["api_version"].as_str() {
                return Some(api.to_string());
            }
        }
    }
    None
}
