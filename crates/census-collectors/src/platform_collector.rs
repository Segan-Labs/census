//! What SeganOS adds to a machine: the Nox kernel and its runtime profile, PipeWire's latency,
//! the Fenestra compositor, GP Mix. Each fact is `None` when it is not there, so on any other
//! distro the same code finds nothing and shows nothing.

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use census_core::fs::read_string;
use census_core::{AudioInfo, GpMixInfo, PlatformInfo};
use serde_json::Value;

use crate::util::run_sync;

const KERNELD_SOCKET: &str = "/run/segan-kerneld.sock";
const KERNELD_MODE_FILE: &str = "/var/lib/segan-kerneld/mode";
const SEGANOS_RELEASE: &str = "/etc/seganos-release";
/// Facts that need a program to be run are looked up again after this long, not on every tick.
const SLOW_TTL: Duration = Duration::from_secs(30);

#[derive(Clone)]
struct SlowFacts {
    audio: Option<AudioInfo>,
    fenestra: Option<String>,
}

#[derive(Default)]
pub struct PlatformCollector {
    slow: Mutex<Option<(Instant, SlowFacts)>>,
}

impl PlatformCollector {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn collect(&self) -> PlatformInfo {
        let slow = self.slow_facts();
        PlatformInfo {
            seganos: read_seganos_release(),
            nox: nox_version(&read_string("/proc/sys/kernel/osrelease")),
            kerneld_mode: kerneld_mode(),
            fenestra: slow.fenestra,
            audio: slow.audio,
            gpmix: gpmix(),
        }
    }

    fn slow_facts(&self) -> SlowFacts {
        let mut guard = self.slow.lock().unwrap_or_else(|e| e.into_inner());
        if let Some((at, facts)) = guard.as_ref() {
            if at.elapsed() < SLOW_TTL {
                return facts.clone();
            }
        }
        let facts = SlowFacts {
            audio: pipewire_audio(),
            fenestra: fenestra_version(),
        };
        *guard = Some((Instant::now(), facts.clone()));
        facts
    }
}

/// "SeganOS 1.0" from /etc/seganos-release: os-release style (`VERSION_ID`, `VERSION`) or just a version on the first line.
fn read_seganos_release() -> Option<String> {
    let text = std::fs::read_to_string(SEGANOS_RELEASE).ok()?;
    parse_seganos_release(&text)
}

pub fn parse_seganos_release(text: &str) -> Option<String> {
    let mut version = None;
    for line in text.lines() {
        let line = line.trim();
        if let Some((k, v)) = line.split_once('=') {
            let v = v.trim().trim_matches('"').to_string();
            if (k == "VERSION_ID" || k == "VERSION") && !v.is_empty() && version.is_none() {
                version = Some(v);
            }
        }
    }
    if version.is_none() {
        let first = text.lines().next().unwrap_or("").trim();
        if !first.is_empty() && !first.contains('=') {
            version = Some(first.trim_start_matches("SeganOS").trim().to_string());
        }
    }
    Some(match version {
        Some(v) if !v.is_empty() => format!("SeganOS {}", v),
        _ => "SeganOS".to_string(),
    })
}

/// Nox's own version out of the kernel release: "7.3.0-nox1.2.0" gives "1.2.0". `None` when the
/// kernel is not Nox (a "segnel" kernel is the old name and is shown as SegNEL, not Nox).
pub fn nox_version(release: &str) -> Option<String> {
    let lower = release.to_lowercase();
    let at = lower.find("nox").map(|i| i + 3)?;
    let rest = &release[at..];
    let ver: String = rest
        .trim_start_matches(['-', '_', '.', ' '])
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.')
        .collect();
    let ver = ver.trim_end_matches('.').to_string();
    Some(if ver.is_empty() { release.to_string() } else { ver })
}

/// kerneld answers `status` on its socket with a JSON line; the mode file is the fallback when the socket is not there.
fn kerneld_mode() -> Option<String> {
    if let Some(m) = kerneld_mode_from_socket() {
        return Some(m);
    }
    let m = read_string(KERNELD_MODE_FILE);
    (!m.is_empty()).then_some(m)
}

fn kerneld_mode_from_socket() -> Option<String> {
    let mut s = UnixStream::connect(KERNELD_SOCKET).ok()?;
    s.set_read_timeout(Some(Duration::from_millis(250))).ok()?;
    s.set_write_timeout(Some(Duration::from_millis(250))).ok()?;
    s.write_all(b"status\n").ok()?;
    let mut line = String::new();
    BufReader::new(s).read_line(&mut line).ok()?;
    let v: Value = serde_json::from_str(line.trim()).ok()?;
    v["mode"].as_str().map(|m| m.to_string())
}

/// PipeWire's default clock from `pw-metadata -n settings`: a forced rate or quantum wins over the default.
fn pipewire_audio() -> Option<AudioInfo> {
    let out = run_sync("pw-metadata", &["-n", "settings"])?;
    parse_pw_settings(&out)
}

pub fn parse_pw_settings(out: &str) -> Option<AudioInfo> {
    let get = |key: &str| -> Option<u32> {
        let needle = format!("key:'{}' value:'", key);
        let line = out.lines().rev().find(|l| l.contains(&needle))?;
        let rest = &line[line.find(&needle)? + needle.len()..];
        rest.split('\'').next()?.parse().ok()
    };
    let nonzero = |v: Option<u32>| v.filter(|n| *n > 0);
    let rate = nonzero(get("clock.force-rate")).or(nonzero(get("clock.rate")))?;
    let quantum = nonzero(get("clock.force-quantum")).or(nonzero(get("clock.quantum")))?;
    Some(AudioInfo {
        rate,
        quantum,
        latency_ms: quantum as f64 / rate as f64 * 1000.0,
    })
}

/// "Fenestra 1.0 (1127)" from `fenestractl -j version`.
fn fenestra_version() -> Option<String> {
    let out = run_sync("fenestractl", &["-j", "version"])?;
    let v: Value = serde_json::from_str(out.trim()).ok()?;
    v["info"]["fenestra"].as_str().map(|s| s.to_string())
}

/// GP Mix (the `segan-mixer` binary): running when a process has that name; installed when it is on PATH.
/// Its version comes from the path it runs from (`segan-mixer-0.3.0-rc.abc123-x86_64/`) when that says one.
fn gpmix() -> Option<GpMixInfo> {
    if let Ok(dir) = std::fs::read_dir("/proc") {
        for e in dir.flatten() {
            let name = e.file_name();
            let name = name.to_string_lossy();
            if !name.bytes().all(|b| b.is_ascii_digit()) {
                continue;
            }
            if read_string(e.path().join("comm")) == "segan-mixer" {
                let exe = std::fs::read_link(e.path().join("exe"))
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_default();
                return Some(GpMixInfo { running: true, version: version_from_path(&exe) });
            }
        }
    }
    let on_path = std::env::var_os("PATH")
        .map(|p| std::env::split_paths(&p).any(|d| d.join("segan-mixer").exists()))
        .unwrap_or(false);
    on_path.then_some(GpMixInfo { running: false, version: None })
}

pub fn version_from_path(path: &str) -> Option<String> {
    let at = path.find("segan-mixer-")? + "segan-mixer-".len();
    let rest = &path[at..];
    let seg = rest.split('/').next().unwrap_or(rest);
    let v = seg.split("-x86_64").next().unwrap_or(seg);
    let v = v.split("-aarch64").next().unwrap_or(v);
    (v.chars().next()?.is_ascii_digit()).then(|| v.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nox_from_kernel_release() {
        assert_eq!(nox_version("7.3.0-nox1.2.0").as_deref(), Some("1.2.0"));
        assert_eq!(nox_version("7.3.0-segnel1.2.0"), None);
        assert_eq!(nox_version("6.18.2-arch1-1"), None);
    }

    #[test]
    fn seganos_release_forms() {
        assert_eq!(parse_seganos_release("VERSION_ID=1.0\n").as_deref(), Some("SeganOS 1.0"));
        assert_eq!(parse_seganos_release("SeganOS 2.1\n").as_deref(), Some("SeganOS 2.1"));
        assert_eq!(parse_seganos_release("").as_deref(), Some("SeganOS"));
    }

    #[test]
    fn pipewire_latency() {
        let out = "update: id:0 key:'clock.rate' value:'48000' type:''\n\
                   update: id:0 key:'clock.quantum' value:'1024' type:''\n\
                   update: id:0 key:'clock.force-quantum' value:'64' type:''\n\
                   update: id:0 key:'clock.force-rate' value:'0' type:''\n";
        let a = parse_pw_settings(out).unwrap();
        assert_eq!((a.rate, a.quantum), (48000, 64));
        assert!((a.latency_ms - 1.333).abs() < 0.01);
    }

    #[test]
    fn mixer_version_from_path() {
        assert_eq!(
            version_from_path("/h/pkg/segan-mixer-0.3.0-rc.578c6f0-x86_64/usr/bin/segan-mixer").as_deref(),
            Some("0.3.0-rc.578c6f0")
        );
        assert_eq!(version_from_path("/usr/bin/segan-mixer"), None);
    }
}
