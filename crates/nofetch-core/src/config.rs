use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    pub daemon: DaemonConfig,
    pub layout: LayoutConfig,
    pub theme: ThemeConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
            socket_path: format!("{}/nofetch.sock", runtime),
            collector_interval_secs: 5,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutConfig {
    pub columns: Vec<Vec<String>>,
}

impl Default for LayoutConfig {
    fn default() -> Self {
        Self {
            columns: vec![
                vec!["system".into(), "hardware".into()],
                vec!["status".into(), "drive".into()],
                vec!["software".into(), "display".into()],
                vec!["network".into(), "sensors".into()],
            ],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
            title: "blue".into(),
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
        let config_dir = dirs::config_dir()
            .map(|p| p.join("nofetch"))
            .unwrap_or_else(|| std::path::PathBuf::from(".config/nofetch"));

        let config_path = config_dir.join("config.toml");
        if config_path.exists() {
            let content = std::fs::read_to_string(&config_path).unwrap_or_default();
            toml::from_str(&content).unwrap_or_default()
        } else {
            Self::default()
        }
    }
}
