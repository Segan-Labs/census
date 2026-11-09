use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub timestamp: DateTime<Utc>,
    pub system: SystemInfo,
    pub hardware: HardwareInfo,
    pub display: DisplayInfo,
    pub network: NetworkInfo,
    pub software: SoftwareInfo,
    pub sensors: SensorInfo,
    #[serde(default)]
    pub platform: PlatformInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInfo {
    pub os: String,
    pub kernel: String,
    pub hostname: String,
    pub wm: String,
    pub shell: String,
    pub uptime_secs: u64,
    pub os_age_days: Option<u64>,
    pub load_avg: [f64; 3],
    pub distro_icon: String,
    pub updates_available: usize,
    pub needs_restart: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareInfo {
    pub mobo: String,
    pub cpu: String,
    pub cpu_cores_physical: usize,
    pub cpu_cores_logical: usize,
    pub gpu: String,
    /// The CPU's highest frequency in MHz, from cpufreq.
    #[serde(default)]
    pub cpu_max_mhz: Option<u32>,
    /// The machine's friendly product name ("MSI MPG B550 GAMING PLUS (MS-7C56)"), from DMI.
    #[serde(default)]
    pub host: Option<String>,
    #[serde(default)]
    pub gpu_driver: Option<String>,
    #[serde(default)]
    pub vulkan: Option<String>,
    pub ram_used_gib: f64,
    pub ram_total_gib: f64,
    pub swap_used_gib: f64,
    pub swap_total_gib: f64,
    pub disks: Vec<DiskInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskInfo {
    pub label: String,
    pub mount: String,
    pub used_gib: f64,
    pub total_gib: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayInfo {
    pub monitors: Vec<MonitorInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitorInfo {
    pub name: String,
    /// "WxH" as the screen is used: already swapped for a rotated monitor.
    pub resolution: String,
    pub refresh_rate: u32,
    pub primary: bool,
    /// Make and model ("R24e"); empty when the monitor does not say.
    #[serde(default)]
    pub model: String,
    #[serde(default = "one")]
    pub scale: f64,
    /// Diagonal in inches, when the monitor reports its physical size.
    #[serde(default)]
    pub diagonal_in: Option<f64>,
}

fn one() -> f64 {
    1.0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkInfo {
    pub lan_ip: Option<String>,
    pub public_ip: Option<String>,
    pub tailnet_ip: Option<String>,
    pub gateway: Option<String>,
    pub rx_bytes: u64,
    pub tx_bytes: u64,
    pub speed_mbps: u64,
    pub rx_rate: f64,
    pub tx_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SoftwareInfo {
    pub pacman: usize,
    pub aur: usize,
    pub cargo: usize,
    pub node: usize,
    pub flatpak: usize,
    #[serde(default)]
    pub appimage: usize,
    pub snap: usize,
    pub rust_version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensorInfo {
    pub cpu_temp_c: Option<f64>,
    pub gpu_temp_c: Option<f64>,
    pub fan_rpm: Option<u32>,
}

/// What a SeganOS machine adds. Every field is `None` when the thing is not there.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PlatformInfo {
    /// "SeganOS 1.0", from /etc/seganos-release.
    pub seganos: Option<String>,
    /// Nox's own version, when the running kernel is Nox.
    pub nox: Option<String>,
    /// The kerneld runtime profile: balanced, audio, game, performance.
    pub kerneld_mode: Option<String>,
    /// "Fenestra 1.0 (1127)".
    pub fenestra: Option<String>,
    pub audio: Option<AudioInfo>,
    pub gpmix: Option<GpMixInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioInfo {
    pub rate: u32,
    pub quantum: u32,
    pub latency_ms: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpMixInfo {
    pub running: bool,
    pub version: Option<String>,
}
