use std::time::Duration;
use std::fs;
use serde_json::{json, Value};
use anyhow::Result;
use census_core::{Collector, fs::{read_parse, exists}};

pub struct SensorCollector;

impl Collector for SensorCollector {
    fn id(&self) -> &'static str {
        "sensor"
    }

    fn interval(&self) -> Duration {
        Duration::from_secs(2)
    }

    async fn collect(&self) -> Result<Value> {
        let cpu_temp = read_cpu_temp();
        let gpu_temp = read_gpu_temp();
        let fan_speed = read_fan_speed();

        Ok(json!({
            "cpu_temp_c": cpu_temp,
            "gpu_temp_c": gpu_temp,
            "fan_rpm": fan_speed,
        }))
    }
}

fn read_cpu_temp() -> Option<f64> {
    if let Ok(entries) = fs::read_dir("/sys/class/thermal") {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.to_string_lossy().contains("thermal_zone") {
                let temp: Option<f64> = read_parse(path.join("temp"));
                if let Some(val) = temp {
                    if val > 0.0 {
                        return Some(val / 1000.0);
                    }
                }
            }
        }
    }

    if let Ok(entries) = fs::read_dir("/sys/class/hwmon") {
        for entry in entries.flatten() {
            let name: String = read_parse(entry.path().join("name")).unwrap_or_default();
            if name == "coretemp" || name == "k10temp" {
                let temp: Option<f64> = read_parse(entry.path().join("temp1_input"));
                if let Some(val) = temp {
                    if val > 0.0 {
                        return Some(val / 1000.0);
                    }
                }
            }
        }
    }

    None
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

fn read_fan_speed() -> Option<u32> {
    if let Ok(entries) = fs::read_dir("/sys/class/hwmon") {
        for entry in entries.flatten() {
            let fan_path = entry.path().join("fan1_input");
            if exists(&fan_path) {
                let speed: Option<u32> = read_parse(&fan_path);
                if let Some(val) = speed {
                    if val > 0 {
                        return Some(val);
                    }
                }
            }
        }
    }
    None
}
