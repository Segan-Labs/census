mod classic;
mod dashboard;
pub mod eclipse;
mod logo;

use census_core::{Config, Snapshot};

/// How one render is asked for; anything `None` comes from the config.
pub struct Options {
    pub style: Option<String>,
    pub logo: Option<String>,
    pub width: usize,
    pub height: Option<usize>,
    pub color: bool,
    /// The terminal the client runs in; only the client knows it.
    pub terminal: Option<String>,
    /// A cell's height over its width, so the logo is round on screen; 2.0 when unknown.
    pub cell_aspect: f64,
}

impl Default for Options {
    fn default() -> Self {
        Self { style: None, logo: None, width: 80, height: None, color: false, terminal: None, cell_aspect: 2.0 }
    }
}

/// The layout names `render_to_string` understands.
pub const STYLES: [&str; 3] = ["classic", "compact", "dashboard"];

pub fn render_to_string(snapshot: &Snapshot, config: &Config, opts: &Options) -> Result<String, String> {
    let style = opts.style.as_deref().unwrap_or(&config.layout.style);
    match style {
        "classic" => Ok(classic::render(snapshot, config, opts, false)),
        "compact" => Ok(classic::render(snapshot, config, opts, true)),
        "dashboard" => Ok(dashboard::render(snapshot, config)),
        other => Err(format!("unknown layout '{}' (use {})", other, STYLES.join(", "))),
    }
}
