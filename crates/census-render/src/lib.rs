mod classic;
mod dashboard;
mod logo;

use census_core::{Config, Snapshot};

/// How one render is asked for; anything `None` comes from the config.
#[derive(Default)]
pub struct Options {
    pub style: Option<String>,
    pub logo: Option<String>,
    pub width: usize,
    pub height: Option<usize>,
    pub color: bool,
    /// The terminal the client runs in; only the client knows it.
    pub terminal: Option<String>,
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
