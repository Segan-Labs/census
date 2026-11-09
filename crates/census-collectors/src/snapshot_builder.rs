//! One place that runs every collector and assembles a `Snapshot`, used by `censusd` on its
//! schedule and by `census` when no daemon is running.

use census_core::{
    Collector, Config, DiskInfo, DisplayInfo, HardwareInfo, MonitorInfo, NetworkInfo, SensorInfo,
    Snapshot, SoftwareInfo, SystemInfo,
};
use serde_json::{json, Value};

use crate::*;

pub struct SnapshotBuilder {
    network: NetworkCollector,
    platform: PlatformCollector,
    collect_network: bool,
}

impl SnapshotBuilder {
    pub fn new(config: &Config) -> Self {
        Self {
            network: NetworkCollector::new(config.privacy.clone()),
            platform: PlatformCollector::new(),
            collect_network: config.privacy.any(),
        }
    }

    pub async fn build(&self) -> Snapshot {
        let network_fut = async {
            if self.collect_network {
                self.network.collect().await.unwrap_or(json!({}))
            } else {
                json!({})
            }
        };
        let (proc, sys, nvml, display, sensor, network, software) = tokio::join!(
            ProcCollector.collect(),
            SysCollector.collect(),
            NvmlCollector.collect(),
            DisplayCollector.collect(),
            SensorCollector.collect(),
            network_fut,
            SoftwareCollector.collect(),
        );
        let platform = self.platform.collect();
        assemble(
            proc.unwrap_or(json!({})),
            sys.unwrap_or(json!({})),
            nvml.unwrap_or(json!({})),
            display.unwrap_or(json!({})),
            sensor.unwrap_or(json!({})),
            network,
            software.unwrap_or(json!({})),
            platform,
        )
    }
}

#[allow(clippy::too_many_arguments)]
fn assemble(
    proc: Value,
    sys: Value,
    nvml: Value,
    display: Value,
    sensor: Value,
    network: Value,
    software: Value,
    platform: census_core::PlatformInfo,
) -> Snapshot {
    let s = |v: &Value, k: &str| v[k].as_str().map(|s| s.to_string());
    Snapshot {
        timestamp: chrono::Utc::now(),
        system: SystemInfo {
            os: s(&sys, "os_name").unwrap_or_else(|| "Linux".into()),
            kernel: s(&proc, "kernel").unwrap_or_else(|| "?".into()),
            hostname: s(&proc, "hostname").unwrap_or_else(|| "host".into()),
            wm: s(&sys, "wm").unwrap_or_else(|| "unknown".into()),
            shell: s(&sys, "shell").unwrap_or_else(|| "unknown".into()),
            uptime_secs: proc["uptime_secs"].as_u64().unwrap_or(0),
            os_age_days: proc["os_age_days"].as_u64(),
            load_avg: [
                proc["load_avg"][0].as_f64().unwrap_or(0.0),
                proc["load_avg"][1].as_f64().unwrap_or(0.0),
                proc["load_avg"][2].as_f64().unwrap_or(0.0),
            ],
            distro_icon: s(&sys, "distro_icon").unwrap_or_else(|| "\u{f31a}".into()),
            updates_available: 0,
            needs_restart: proc["needs_restart"].as_bool().unwrap_or(false),
        },
        hardware: HardwareInfo {
            mobo: s(&sys, "board").unwrap_or_else(|| "Unknown".into()),
            cpu: s(&sys, "cpu_brand").unwrap_or_else(|| "Unknown CPU".into()),
            cpu_cores_physical: sys["cores_physical"].as_u64().unwrap_or(0) as usize,
            cpu_cores_logical: sys["cores_logical"].as_u64().unwrap_or(0) as usize,
            cpu_max_mhz: sys["cpu_max_mhz"].as_u64().map(|v| v as u32),
            host: s(&sys, "host"),
            gpu: s(&nvml, "gpu_name").unwrap_or_else(|| "Unknown GPU".into()),
            gpu_driver: s(&nvml, "gpu_driver"),
            vulkan: s(&nvml, "vulkan"),
            ram_used_gib: proc["ram_used_gib"].as_f64().unwrap_or(0.0),
            ram_total_gib: proc["ram_total_gib"].as_f64().unwrap_or(0.0),
            swap_used_gib: proc["swap_used_gib"].as_f64().unwrap_or(0.0),
            swap_total_gib: proc["swap_total_gib"].as_f64().unwrap_or(0.0),
            disks: proc["disks"]
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .map(|d| DiskInfo {
                            label: s(d, "label").unwrap_or_default(),
                            mount: s(d, "mount").unwrap_or_default(),
                            used_gib: d["used_gib"].as_f64().unwrap_or(0.0),
                            total_gib: d["total_gib"].as_f64().unwrap_or(0.0),
                            removable: d["removable"].as_bool().unwrap_or(false),
                        })
                        .collect()
                })
                .unwrap_or_default(),
        },
        display: DisplayInfo {
            monitors: display["monitors"]
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .map(|m| MonitorInfo {
                            name: s(m, "name").unwrap_or_default(),
                            resolution: s(m, "resolution").unwrap_or_default(),
                            refresh_rate: m["refresh_rate"].as_u64().unwrap_or(0) as u32,
                            primary: m["primary"].as_bool().unwrap_or(false),
                            model: s(m, "model").unwrap_or_default(),
                            scale: m["scale"].as_f64().unwrap_or(1.0),
                            diagonal_in: m["diagonal_in"].as_f64(),
                        })
                        .collect()
                })
                .unwrap_or_default(),
        },
        network: NetworkInfo {
            lan_ip: s(&network, "lan_ip"),
            public_ip: s(&network, "public_ip"),
            tailnet_ip: s(&network, "tailnet_ip"),
            gateway: s(&network, "gateway"),
            rx_bytes: network["rx_bytes"].as_u64().unwrap_or(0),
            tx_bytes: network["tx_bytes"].as_u64().unwrap_or(0),
            speed_mbps: network["speed_mbps"].as_u64().unwrap_or(0),
            rx_rate: network["rx_rate"].as_f64().unwrap_or(0.0),
            tx_rate: network["tx_rate"].as_f64().unwrap_or(0.0),
        },
        software: SoftwareInfo {
            pacman: software["pacman"].as_u64().unwrap_or(0) as usize,
            aur: software["aur"].as_u64().unwrap_or(0) as usize,
            cargo: software["cargo"].as_u64().unwrap_or(0) as usize,
            node: software["node"].as_u64().unwrap_or(0) as usize,
            flatpak: software["flatpak"].as_u64().unwrap_or(0) as usize,
            appimage: software["appimage"].as_u64().unwrap_or(0) as usize,
            snap: software["snap"].as_u64().unwrap_or(0) as usize,
            rust_version: s(&software, "rust_ver"),
        },
        sensors: SensorInfo {
            cpu_temp_c: sensor["cpu_temp_c"].as_f64(),
            gpu_temp_c: sensor["gpu_temp_c"].as_f64().or_else(|| nvml["gpu_temp_c"].as_f64()),
            fan_rpm: sensor["fan_rpm"].as_u64().map(|v| v as u32),
        },
        platform,
    }
}
