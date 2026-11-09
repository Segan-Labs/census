use std::time::Duration;
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
        Ok(json!({ "monitors": read_monitors() }))
    }
}

/// The compositor's own list first (Fenestra, then Hyprland), the kernel's DRM connectors last.
fn read_monitors() -> Vec<Value> {
    for tool in [["fenestractl", "-j", "monitors"], ["hyprctl", "-j", "monitors"]] {
        if let Some(out) = crate::util::run_sync(tool[0], &tool[1..]) {
            let m = parse_compositor_monitors(&out);
            if !m.is_empty() {
                return m;
            }
        }
    }
    read_drm_monitors()
}

/// Reads the JSON of `fenestractl -j monitors` (`{"monitors": [...]}`) or `hyprctl -j monitors` (`[...]`).
pub fn parse_compositor_monitors(text: &str) -> Vec<Value> {
    let Ok(v) = serde_json::from_str::<Value>(text) else { return Vec::new() };
    let list = v["monitors"].as_array().or_else(|| v.as_array());
    let Some(list) = list else { return Vec::new() };
    list.iter()
        .filter(|m| !m["disabled"].as_bool().unwrap_or(false))
        .enumerate()
        .map(|(i, m)| {
            let (w, h) = oriented(
                m["width"].as_u64().unwrap_or(0),
                m["height"].as_u64().unwrap_or(0),
                m["transform"].as_u64().unwrap_or(0),
            );
            let diagonal = diagonal_in(m["physicalWidth"].as_f64().unwrap_or(0.0), m["physicalHeight"].as_f64().unwrap_or(0.0));
            json!({
                "name": m["name"].as_str().unwrap_or(""),
                "model": monitor_model(m),
                "resolution": format!("{}x{}", w, h),
                "refresh_rate": m["refreshRate"].as_f64().unwrap_or(0.0).round() as u32,
                "scale": m["scale"].as_f64().filter(|s| *s > 0.0).unwrap_or(1.0),
                "diagonal_in": diagonal,
                "primary": i == 0,
            })
        })
        .collect()
}

/// The size as it is used on screen: the 90 and 270 degree transforms (1, 3, 5, 7) swap the sides.
pub fn oriented(w: u64, h: u64, transform: u64) -> (u64, u64) {
    if transform % 2 == 1 { (h, w) } else { (w, h) }
}

/// Screen diagonal in whole inches from the physical size in mm; `None` when the monitor reports none.
pub fn diagonal_in(w_mm: f64, h_mm: f64) -> Option<f64> {
    if w_mm <= 0.0 || h_mm <= 0.0 {
        return None;
    }
    Some(((w_mm * w_mm + h_mm * h_mm).sqrt() / 25.4).round())
}

/// "R24e" from the model; else the description without its serial; else empty.
fn monitor_model(m: &Value) -> String {
    let model = m["model"].as_str().unwrap_or("").trim();
    if !model.is_empty() {
        return model.to_string();
    }
    let desc = m["description"].as_str().unwrap_or("").trim();
    let serial = m["serial"].as_str().unwrap_or("");
    let desc = if serial.is_empty() { desc } else { desc.trim_end_matches(serial).trim() };
    // Compositors fall back to the connector name as the description.
    if desc == m["name"].as_str().unwrap_or("") { String::new() } else { desc.to_string() }
}

/// No compositor to ask: connected DRM connectors with their first (preferred) mode and the EDID's name and size.
fn read_drm_monitors() -> Vec<Value> {
    let Ok(dir) = std::fs::read_dir("/sys/class/drm") else { return Vec::new() };
    let mut names: Vec<String> = dir
        .flatten()
        .map(|e| e.file_name().to_string_lossy().to_string())
        .filter(|n| n.starts_with("card") && n.contains('-'))
        .collect();
    names.sort();
    let mut out = Vec::new();
    for n in names {
        let p = std::path::Path::new("/sys/class/drm").join(&n);
        if census_core::fs::read_string(p.join("status")) != "connected" {
            continue;
        }
        let modes = census_core::fs::read_string(p.join("modes"));
        let Some(mode) = modes.lines().next() else { continue };
        let edid = std::fs::read(p.join("edid")).unwrap_or_default();
        let (model, w_mm, h_mm) = parse_edid(&edid);
        let name = n.split_once('-').map(|x| x.1).unwrap_or(&n).to_string();
        out.push(json!({
            "name": name,
            "model": model,
            "resolution": mode,
            "refresh_rate": 0,
            "scale": 1.0,
            "diagonal_in": diagonal_in(w_mm, h_mm),
            "primary": out.is_empty(),
        }));
    }
    out
}

/// The monitor name descriptor (tag 0xFC) and the physical size (cm in bytes 21 and 22, as mm).
pub fn parse_edid(edid: &[u8]) -> (String, f64, f64) {
    if edid.len() < 128 {
        return (String::new(), 0.0, 0.0);
    }
    let mut name = String::new();
    for off in [54, 72, 90, 108] {
        let d = &edid[off..off + 18];
        if d[0] == 0 && d[1] == 0 && d[3] == 0xFC {
            name = String::from_utf8_lossy(&d[5..18]).split('\n').next().unwrap_or("").trim().to_string();
        }
    }
    (name, edid[21] as f64 * 10.0, edid[22] as f64 * 10.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    const FENESTRA: &str = r#"{"id":1,"ok":true,"monitors":[
      {"name":"DP-1","description":"Lenovo Group Limited R24e UPA69019","make":"Lenovo Group Limited","model":"R24e","serial":"UPA69019",
       "width":1920,"height":1080,"physicalWidth":530,"physicalHeight":290,"refreshRate":60.0,"scale":1.77,"transform":3,"disabled":false},
      {"name":"DP-2","description":"Lenovo Group Limited N27q","make":"Lenovo","model":"N27q","serial":"",
       "width":2560,"height":1440,"physicalWidth":600,"physicalHeight":340,"refreshRate":59.951,"scale":1,"transform":0}]}"#;

    #[test]
    fn rotation_swaps_the_sides() {
        assert_eq!(oriented(1920, 1080, 0), (1920, 1080));
        assert_eq!(oriented(1920, 1080, 1), (1080, 1920));
        assert_eq!(oriented(1920, 1080, 2), (1920, 1080));
        assert_eq!(oriented(1920, 1080, 3), (1080, 1920));
        assert_eq!(oriented(1920, 1080, 7), (1080, 1920));
    }

    #[test]
    fn diagonal_from_millimetres() {
        assert_eq!(diagonal_in(600.0, 340.0), Some(27.0));
        assert_eq!(diagonal_in(530.0, 290.0), Some(24.0));
        assert_eq!(diagonal_in(540.0, 310.0), Some(25.0));
        assert_eq!(diagonal_in(0.0, 0.0), None);
    }

    #[test]
    fn parses_fenestra_json() {
        let m = parse_compositor_monitors(FENESTRA);
        assert_eq!(m.len(), 2);
        assert_eq!(m[0]["model"], "R24e");
        assert_eq!(m[0]["resolution"], "1080x1920");
        assert_eq!(m[0]["scale"], 1.77);
        assert_eq!(m[0]["diagonal_in"], 24.0);
        assert_eq!(m[1]["resolution"], "2560x1440");
        assert_eq!(m[1]["refresh_rate"], 60);
    }

    #[test]
    fn parses_hyprctl_array() {
        let m = parse_compositor_monitors(r#"[{"name":"eDP-1","description":"eDP-1","width":1920,"height":1200,"refreshRate":60.0,"scale":1.5,"transform":0}]"#);
        assert_eq!(m[0]["model"], "");
        assert_eq!(m[0]["diagonal_in"], Value::Null);
    }

    #[test]
    fn edid_name_and_size() {
        let mut e = vec![0u8; 128];
        e[21] = 60;
        e[22] = 34;
        e[72..77].copy_from_slice(&[0, 0, 0, 0xFC, 0]);
        e[77..82].copy_from_slice(b"N27q\n");
        assert_eq!(parse_edid(&e), ("N27q".to_string(), 600.0, 340.0));
    }
}
