use std::sync::Arc;
use tokio::net::UnixListener;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::RwLock;
use nofetch_core::{Config, Snapshot, Collector};
use nofetch_collectors::*;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let config = Config::load();
    let socket_path = &config.daemon.socket_path;

    let _ = std::fs::remove_file(socket_path);

    let listener = UnixListener::bind(socket_path)?;
    tracing::info!("nofetchd listening on {}", socket_path);

    let snapshot = build_snapshot().await;
    let shared = Arc::new(RwLock::new(snapshot));

    let sched_snapshot = shared.clone();
    let interval = config.daemon.collector_interval_secs;
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(interval)).await;
            let new_snapshot = build_snapshot().await;
            *sched_snapshot.write().await = new_snapshot;
        }
    });

    loop {
        let (mut stream, _) = listener.accept().await?;
        let snap = shared.clone();
        tokio::spawn(async move {
            let mut buffer = vec![0u8; 1024];
            let n = stream.read(&mut buffer).await.unwrap_or(0);

            let request = String::from_utf8_lossy(&buffer[..n]);
            if request.starts_with("GET /snapshot") {
                let snapshot = snap.read().await;
                let json = serde_json::to_string(&*snapshot).unwrap_or_default();
                // Send Content-Length header then body
                let header = format!("Content-Length: {}\r\n\r\n", json.len());
                let _ = stream.write_all(header.as_bytes()).await;
                let _ = stream.write_all(json.as_bytes()).await;
            }
            let _ = stream.shutdown().await;
        });
    }
}

async fn build_snapshot() -> Snapshot {
    let proc = ProcCollector;
    let sys = SysCollector;
    let nvml = NvmlCollector;
    let display = DisplayCollector;
    let sensor = SensorCollector;
    let network = NetworkCollector::new();
    let software = SoftwareCollector;

    let (proc_data, sys_data, nvml_data, display_data, sensor_data, network_data, software_data) = tokio::join!(
        proc.collect(),
        sys.collect(),
        nvml.collect(),
        display.collect(),
        sensor.collect(),
        network.collect(),
        software.collect(),
    );

    let proc = proc_data.unwrap_or(serde_json::json!({}));
    let sys = sys_data.unwrap_or(serde_json::json!({}));
    let nvml = nvml_data.unwrap_or(serde_json::json!({}));
    let display = display_data.unwrap_or(serde_json::json!({}));
    let sensor = sensor_data.unwrap_or(serde_json::json!({}));
    let network = network_data.unwrap_or(serde_json::json!({}));
    let software = software_data.unwrap_or(serde_json::json!({}));

    Snapshot {
        timestamp: chrono::Utc::now(),
        system: nofetch_core::SystemInfo {
            os: sys["os_name"].as_str().unwrap_or("Linux").to_string(),
            kernel: proc["kernel"].as_str().unwrap_or("?").to_string(),
            hostname: proc["hostname"].as_str().unwrap_or("host").to_string(),
            wm: sys["wm"].as_str().unwrap_or("unknown").to_string(),
            shell: sys["shell"].as_str().unwrap_or("unknown").to_string(),
            uptime_secs: proc["uptime_secs"].as_u64().unwrap_or(0),
            os_age_days: proc["os_age_days"].as_u64(),
            load_avg: [
                proc["load_avg"][0].as_f64().unwrap_or(0.0),
                proc["load_avg"][1].as_f64().unwrap_or(0.0),
                proc["load_avg"][2].as_f64().unwrap_or(0.0),
            ],
            distro_icon: sys["distro_icon"].as_str().unwrap_or("\u{f31a}").to_string(),
            updates_available: proc["updates_available"].as_u64().unwrap_or(0) as usize,
            needs_restart: proc["needs_restart"].as_bool().unwrap_or(false),
        },
        hardware: nofetch_core::HardwareInfo {
            mobo: sys["board"].as_str().unwrap_or("Unknown").to_string(),
            cpu: sys["cpu_brand"].as_str().unwrap_or("Unknown CPU").to_string(),
            cpu_cores_physical: sys["cores_physical"].as_u64().unwrap_or(0) as usize,
            cpu_cores_logical: sys["cores_logical"].as_u64().unwrap_or(0) as usize,
            gpu: nvml["gpu_name"].as_str().unwrap_or("Unknown GPU").to_string(),
            ram_used_gib: proc["ram_used_gib"].as_f64().unwrap_or(0.0),
            ram_total_gib: proc["ram_total_gib"].as_f64().unwrap_or(0.0),
            swap_used_gib: proc["swap_used_gib"].as_f64().unwrap_or(0.0),
            swap_total_gib: proc["swap_total_gib"].as_f64().unwrap_or(0.0),
            disks: proc["disks"]
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .map(|d| nofetch_core::DiskInfo {
                            label: d["label"].as_str().unwrap_or("").to_string(),
                            mount: d["mount"].as_str().unwrap_or("").to_string(),
                            used_gib: d["used_gib"].as_f64().unwrap_or(0.0),
                            total_gib: d["total_gib"].as_f64().unwrap_or(0.0),
                        })
                        .collect()
                })
                .unwrap_or_default(),
        },
        display: nofetch_core::DisplayInfo {
            monitors: display["monitors"]
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .map(|m| nofetch_core::MonitorInfo {
                            name: m["name"].as_str().unwrap_or("").to_string(),
                            resolution: m["resolution"].as_str().unwrap_or("").to_string(),
                            refresh_rate: m["refresh_rate"].as_u64().unwrap_or(0) as u32,
                            primary: m["primary"].as_bool().unwrap_or(false),
                        })
                        .collect()
                })
                .unwrap_or_default(),
        },
        network: nofetch_core::NetworkInfo {
            lan_ip: network["lan_ip"].as_str().map(|s| s.to_string()),
            public_ip: network["public_ip"].as_str().map(|s| s.to_string()),
            tailnet_ip: network["tailnet_ip"].as_str().map(|s| s.to_string()),
            gateway: network["gateway"].as_str().map(|s| s.to_string()),
            rx_bytes: network["rx_bytes"].as_u64().unwrap_or(0),
            tx_bytes: network["tx_bytes"].as_u64().unwrap_or(0),
            speed_mbps: network["speed_mbps"].as_u64().unwrap_or(0),
            rx_rate: network["rx_rate"].as_f64().unwrap_or(0.0),
            tx_rate: network["tx_rate"].as_f64().unwrap_or(0.0),
        },
        software: nofetch_core::SoftwareInfo {
            pacman: software["pacman"].as_u64().unwrap_or(0) as usize,
            aur: software["aur"].as_u64().unwrap_or(0) as usize,
            cargo: software["cargo"].as_u64().unwrap_or(0) as usize,
            node: software["node"].as_u64().unwrap_or(0) as usize,
            flatpak: software["flatpak"].as_u64().unwrap_or(0) as usize,
            snap: software["snap"].as_u64().unwrap_or(0) as usize,
            rust_version: software["rust_ver"].as_str().map(|s| s.to_string()),
        },
        sensors: nofetch_core::SensorInfo {
            cpu_temp_c: sensor["cpu_temp_c"].as_f64(),
            gpu_temp_c: sensor["gpu_temp_c"].as_f64().or_else(|| nvml["gpu_temp_c"].as_f64()),
            fan_rpm: sensor["fan_rpm"].as_u64().map(|v| v as u32),
        },
    }
}
