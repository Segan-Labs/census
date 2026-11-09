use std::time::Duration;
use std::process::Command;
use serde_json::{json, Value};
use anyhow::Result;
use census_core::Collector;

pub struct DisplayCollector;

impl Collector for DisplayCollector {
    fn id(&self) -> &'static str {
        "display"
    }

    fn interval(&self) -> Duration {
        Duration::from_secs(30)
    }

    async fn collect(&self) -> Result<Value> {
        // Run hyprctl only once, parse both text and JSON from same output
        let monitors = read_monitors();
        Ok(json!({ "monitors": monitors }))
    }
}

fn read_monitors() -> Vec<Value> {
    let m = read_fenestra_monitors();
    if !m.is_empty() {
        return m;
    }
    read_hyprctl_monitors()
}

fn read_fenestra_monitors() -> Vec<Value> {
    let Some(out) = crate::util::run_sync("fenestractl", &["-j", "monitors"]) else { return Vec::new() };
    let Ok(v) = serde_json::from_str::<Value>(&out) else { return Vec::new() };
    let Some(list) = v["monitors"].as_array() else { return Vec::new() };
    list.iter()
        .enumerate()
        .map(|(i, m)| {
            json!({
                "name": m["name"].as_str().unwrap_or(""),
                "resolution": format!("{}x{}", m["width"].as_u64().unwrap_or(0), m["height"].as_u64().unwrap_or(0)),
                "refresh_rate": m["refreshRate"].as_f64().unwrap_or(0.0).round() as u32,
                "primary": i == 0,
            })
        })
        .collect()
}

fn read_hyprctl_monitors() -> Vec<Value> {
    let output = Command::new("hyprctl")
        .arg("monitors")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .unwrap_or_default();

    let mut monitors = Vec::new();
    let mut current_name = String::new();
    let mut found_first = false;

    for line in output.lines() {
        let l = line.trim();
        if l.starts_with("Monitor ") {
            current_name = l.split_whitespace().nth(1).unwrap_or("").to_string();
        } else if l.contains('@') && l.contains('x') && !current_name.is_empty() {
            let parts: Vec<&str> = l.split('@').collect();
            if parts.len() == 2 {
                let res = parts[0].trim();
                let hz: u32 = parts[1].split('.').next()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0);
                // First monitor in focused workspace is typically primary
                let primary = !found_first;
                found_first = true;
                monitors.push(json!({
                    "name": current_name,
                    "resolution": res,
                    "refresh_rate": hz,
                    "primary": primary,
                }));
                current_name.clear();
            }
        }
    }

    monitors
}
