use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub daemon: DaemonConfig,
    pub layout: LayoutConfig,
    pub theme: ThemeConfig,
    pub privacy: PrivacyConfig,
}

/// What Census may look up that is about your network. All off: nothing private is
/// collected or printed unless you turn it on here.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct PrivacyConfig {
    pub lan_ip: bool,
    pub public_ip: bool,
    pub tailnet_ip: bool,
}

impl PrivacyConfig {
    pub fn any(&self) -> bool {
        self.lan_ip || self.public_ip || self.tailnet_ip
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DaemonConfig {
    pub socket_path: String,
    pub collector_interval_secs: u64,
}

impl Default for DaemonConfig {
    fn default() -> Self {
        let runtime = dirs::runtime_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| "/tmp".to_string());
        Self {
            socket_path: format!("{}/census.sock", runtime),
            collector_interval_secs: 5,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct LayoutConfig {
    /// `classic` (logo left, list right), `compact` (small logo on top) or `dashboard` (cards).
    pub style: String,
    /// `auto` (Eclipse on SeganOS, a neutral mark elsewhere), `eclipse`, `neutral` or `none`.
    pub logo: String,
    /// Sections of the list layouts, in order: system, hardware, session, audio, network.
    pub sections: Vec<String>,
    /// Cards of the dashboard layout, two per row.
    pub columns: Vec<Vec<String>>,
    /// A terminal cell's height over its width, when the terminal does not say (or says wrong).
    /// Unset: what the terminal reports, else 2.0. `CENSUS_CELL_ASPECT` in the environment wins.
    pub cell_aspect: Option<f64>,
    /// Show USB sticks, SD cards and other removable media as Disk rows.
    pub removable_disks: bool,
}

impl Default for LayoutConfig {
    fn default() -> Self {
        Self {
            style: "classic".into(),
            logo: "auto".into(),
            sections: ["system", "hardware", "session", "audio", "network"]
                .iter()
                .map(|s| s.to_string())
                .collect(),
            columns: vec![
                vec!["system".into(), "hardware".into()],
                vec!["status".into(), "drive".into()],
                vec!["software".into(), "display".into()],
                vec!["network".into(), "sensors".into()],
            ],
            cell_aspect: None,
            removable_disks: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ThemeConfig {
    pub border: String,
    pub title: String,
    pub label: String,
    pub value: String,
    pub progress: String,
    pub warning: String,
    pub critical: String,
    pub reset: String,
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self {
            border: "cyan".into(),
            title: "magenta".into(),
            label: "white".into(),
            value: "white".into(),
            progress: "green".into(),
            warning: "yellow".into(),
            critical: "red".into(),
            reset: "reset".into(),
        }
    }
}

impl Config {
    pub fn load() -> Self {
        let config_dir = std::env::var_os("CENSUS_CONFIG_DIR")
            .map(std::path::PathBuf::from)
            .or_else(|| dirs::config_dir().map(|p| p.join("census")))
            .unwrap_or_else(|| std::path::PathBuf::from(".config/census"));

        let config_path = config_dir.join("config.toml");
        if config_path.exists() {
            let content = std::fs::read_to_string(&config_path).unwrap_or_default();
            toml::from_str(&content).unwrap_or_default()
        } else {
            Self::default()
        }
    }
}
