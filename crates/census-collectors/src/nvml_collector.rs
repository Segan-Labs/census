use std::time::Duration;
use std::fs;
use serde_json::{json, Value};
use anyhow::Result;
use census_core::{Collector, fs::{read_string, read_parse}};
use crate::gpu_info::{pci_ids_name, read_gpu_driver};

pub struct NvmlCollector;

impl Collector for NvmlCollector {
    fn id(&self) -> &'static str {
        "nvml"
    }

    fn interval(&self) -> Duration {
        Duration::from_secs(5)
    }

    async fn collect(&self) -> Result<Value> {
        let gpu_name = read_gpu_name();
        let gpu_temp = read_gpu_temp();
        let (gpu_driver, vulkan) = read_gpu_driver();

        Ok(json!({
            "gpu_name": gpu_name,
            "gpu_temp_c": gpu_temp,
            "gpu_driver": gpu_driver,
            "vulkan": vulkan,
        }))
    }
}

fn read_gpu_name() -> String {
    if let Ok(entries) = fs::read_dir("/sys/class/drm") {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with("card") && !name.contains("-") {
                let uevent = read_string(entry.path().join("device/uevent"));
                for line in uevent.lines() {
                    if let Some(val) = line.strip_prefix("PCI_ID=") {
                        let parts: Vec<&str> = val.split(':').collect();
                        if parts.len() == 2 {
                            if let Some(n) = pci_ids_name(parts[0], parts[1]) {
                                return n;
                            }
                            return get_pci_device_name(parts[0], parts[1]);
                        }
                    }
                }
            }
        }
    }
    "Unknown".into()
}

fn get_pci_device_name(vendor: &str, device: &str) -> String {
    if vendor.to_lowercase() == "10de" {
        let device_id = device.trim_start_matches("0x");
        match device_id {
            "2484" | "2489" => "GeForce RTX 3060 Ti".into(),
            "2204" | "2205" => "GeForce RTX 3060".into(),
            "2488" => "GeForce RTX 3070".into(),
            "248c" => "GeForce RTX 3080".into(),
            "2208" | "220a" => "GeForce RTX 3090".into(),
            "2684" => "GeForce RTX 4060".into(),
            "2782" | "2783" => "GeForce RTX 4070".into(),
            "2786" | "2787" => "GeForce RTX 4070 Ti".into(),
            "268c" => "GeForce RTX 4080".into(),
            "26b0" | "26b1" => "GeForce RTX 4090".into(),
            "1b38" => "GeForce GTX 1060".into(),
            "1b83" => "GeForce GTX 1070".into(),
            "1b80" => "GeForce GTX 1080".into(),
            "1e84" => "GeForce RTX 2070".into(),
            "1e87" => "GeForce RTX 2080".into(),
            "2184" => "GeForce GTX 1660 Ti".into(),
            _ => format!("NVIDIA GPU ({})", device_id),
        }
    } else {
        format!("PCI Device ({}:{})", vendor, device)
    }
}

fn read_gpu_temp() -> Option<f64> {
    if let Ok(entries) = fs::read_dir("/sys/class/drm") {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with("card") && !name.contains("-") {
                let device = entry.path().join("device");
                if let Ok(hwmon_entries) = fs::read_dir(device.join("hwmon")) {
                    for hwmon in hwmon_entries.flatten() {
                        let temp: Option<f64> = read_parse(hwmon.path().join("temp1_input"));
                        if let Some(val) = temp {
                            if val > 0.0 {
                                return Some(val / 1000.0);
                            }
                        }
                    }
                }
            }
        }
    }
    None
}
