//! The list layouts: a logo, and an aligned list grouped into a few sections. No boxes, no bars.
//! Labels are dim, values are the terminal's own foreground, one accent colour (ANSI magenta, so it
//! follows the terminal's theme) marks the logo, the user@host line and the section names; the
//! shadowed part of the Eclipse is dim, like the labels.

use crate::logo::{self, Logo, Tone};
use crate::Options;
use census_core::{Config, Snapshot};

const RESET: &str = "\x1b[0m";
const DIM: &str = "\x1b[2m";
const ACCENT: &str = "\x1b[35m";
const ACCENT_BOLD: &str = "\x1b[1;35m";

/// A list is never wider than this, so a very wide terminal does not stretch the values away from their labels.
const MAX_INFO_WIDTH: usize = 56;
/// Below this much room for the list, the logo is dropped.
const MIN_INFO_WIDTH: usize = 40;
const LOGO_GAP: usize = 3;

struct Row {
    label: &'static str,
    value: String,
    /// A usage percentage shown at the end of `value` as "(NN%)", painted by how full it is.
    pct: Option<u32>,
    /// 0 stays as long as it can; 2 goes first when the terminal is short.
    prio: u8,
}

struct Section {
    title: &'static str,
    rows: Vec<Row>,
}

type Line = String;

struct Style {
    color: bool,
}

impl Style {
    fn paint(&self, code: &str, s: &str) -> String {
        if self.color && !s.is_empty() {
            format!("{}{}{}", code, s, RESET)
        } else {
            s.to_string()
        }
    }
    /// The colour of a usage percentage: green below 60, yellow below 85, red from 85 up.
    fn usage(&self, pct: u32, s: &str) -> String {
        self.paint(usage_color(pct), s)
    }
    fn dim(&self, s: &str) -> String {
        self.paint(DIM, s)
    }
    fn accent(&self, s: &str) -> String {
        self.paint(ACCENT, s)
    }
    fn title(&self, s: &str) -> String {
        self.paint(ACCENT_BOLD, s)
    }
}

pub fn usage_color(pct: u32) -> &'static str {
    if pct < 60 {
        "\x1b[32m"
    } else if pct < 85 {
        "\x1b[33m"
    } else {
        "\x1b[31m"
    }
}

pub fn render(snapshot: &Snapshot, config: &Config, opts: &Options, compact: bool) -> String {
    let st = Style { color: opts.color };
    let width = opts.width;
    let mut sections = sections(snapshot, config, opts);
    let user = std::env::var("USER").unwrap_or_else(|_| "user".into());
    let header = format!("{}@{}", user, snapshot.system.hostname);

    let choice = opts.logo.as_deref().unwrap_or(&config.layout.logo);
    let logo = logo::pick(choice, snapshot, compact);

    if compact {
        render_compact(&mut sections, &header, logo, width, opts.height, &st)
    } else {
        render_classic(&mut sections, &header, logo, width, opts.height, &st)
    }
}

fn render_classic(
    sections: &mut Vec<Section>,
    header: &str,
    logo: Option<Logo>,
    width: usize,
    height: Option<usize>,
    st: &Style,
) -> String {
    let budget = height.map(|h| h.saturating_sub(1));
    // A logo taller than the terminal is dropped before anything else is.
    let logo = logo.filter(|l| budget.is_none_or(|b| l.height() <= b));
    if let Some(b) = budget {
        fit(sections, b, 3);
    }

    let natural = natural_width(sections, header);
    let mut logo = logo;
    if let Some(l) = &logo {
        let room = width.saturating_sub(l.width + LOGO_GAP);
        if room < natural.min(MIN_INFO_WIDTH) {
            logo = None;
        }
    }
    let avail = match &logo {
        Some(l) => width.saturating_sub(l.width + LOGO_GAP),
        None => width,
    }
    .min(MAX_INFO_WIDTH);

    let lw = label_width(sections);
    let mut lines = vec![st.title(header)];
    lines.push(st.dim(&"─".repeat(header.chars().count().min(avail))));
    lines.push(blank());
    for (i, s) in sections.iter().enumerate() {
        if i > 0 {
            lines.push(blank());
        }
        lines.push(st.title(s.title));
        lines.extend(row_lines(s, lw, avail, st));
    }
    join(logo.as_ref(), lines, st)
}

/// A small logo beside the system summary, the other sections below it under thin rules.
fn render_compact(
    sections: &mut Vec<Section>,
    header: &str,
    logo: Option<Logo>,
    width: usize,
    height: Option<usize>,
    st: &Style,
) -> String {
    let budget = height.map(|h| h.saturating_sub(1));
    let logo = logo.filter(|l| budget.is_none_or(|b| l.height() <= b));
    let first = if sections.is_empty() { None } else { Some(sections.remove(0)) };
    let top_rows = first.as_ref().map(|s| s.rows.len()).unwrap_or(0) + 3;
    let top_h = top_rows.max(logo.as_ref().map(|l| l.height()).unwrap_or(0));
    if let Some(b) = budget {
        fit(sections, b.saturating_sub(top_h), 1);
    }

    let mut all: Vec<&Section> = sections.iter().collect();
    if let Some(f) = &first {
        all.insert(0, f);
    }
    let lw = all.iter().flat_map(|s| s.rows.iter()).map(|r| r.label.len()).max().unwrap_or(0);
    let natural = natural_width(sections, header).max(first.as_ref().map(|f| 4 + lw + f.rows.iter().map(|r| r.value.chars().count()).max().unwrap_or(0)).unwrap_or(0));
    let mut logo = logo;
    if let Some(l) = &logo {
        if width.saturating_sub(l.width + LOGO_GAP) < natural.min(MIN_INFO_WIDTH) {
            logo = None;
        }
    }
    let top_avail = match &logo {
        Some(l) => width.saturating_sub(l.width + LOGO_GAP),
        None => width,
    }
    .min(MAX_INFO_WIDTH);
    let avail = width.min(MAX_INFO_WIDTH);

    let mut top = vec![st.title(header)];
    top.push(st.dim(&"─".repeat(header.chars().count().min(top_avail))));
    top.push(blank());
    if let Some(f) = &first {
        top.extend(row_lines(f, lw, top_avail, st));
    }
    let mut out = join(logo.as_ref(), top, st);

    for s in sections.iter() {
        out.push('\n');
        let fill = avail.saturating_sub(s.title.chars().count() + 4);
        out.push_str(&format!("{}{}{}\n", st.dim("──"), st.title(&format!(" {} ", s.title)), st.dim(&"─".repeat(fill))));
        for l in row_lines(s, lw, avail, st) {
            out.push_str(&l);
            out.push('\n');
        }
    }
    out
}

fn blank() -> Line {
    String::new()
}

fn label_width(sections: &[Section]) -> usize {
    sections.iter().flat_map(|s| s.rows.iter()).map(|r| r.label.len()).max().unwrap_or(0)
}

fn natural_width(sections: &[Section], header: &str) -> usize {
    let lw = label_width(sections);
    let v = sections.iter().flat_map(|s| s.rows.iter()).map(|r| r.value.chars().count()).max().unwrap_or(0);
    (2 + lw + 2 + v).max(header.chars().count())
}

fn row_lines(s: &Section, lw: usize, avail: usize, st: &Style) -> Vec<Line> {
    let room = avail.saturating_sub(2 + lw + 2).max(8);
    s.rows
        .iter()
        .map(|r| {
            let mut value = trunc(&r.value, room);
            if let Some(p) = r.pct {
                let tag = format!("({}%)", p);
                if let Some(at) = value.find(&tag) {
                    let painted = format!("({})", st.usage(p, &format!("{}%", p)));
                    value.replace_range(at..at + tag.len(), &painted);
                }
            }
            format!("  {}  {}", st.dim(&format!("{:<w$}", r.label, w = lw)), value)
        })
        .collect()
}

/// The logo (if any) on the left, the lines on the right, top-aligned.
fn join(logo: Option<&Logo>, lines: Vec<Line>, st: &Style) -> String {
    let mut out = String::new();
    let n = lines.len().max(logo.map(|l| l.height()).unwrap_or(0));
    for i in 0..n {
        if let Some(l) = logo {
            let mut w = 0;
            for (tone, run) in l.row(i) {
                w += run.chars().count();
                out.push_str(&match tone {
                    Tone::Accent => st.accent(&run),
                    Tone::Bold => st.title(&run),
                    Tone::Dim => st.dim(&run),
                    Tone::Plain => run,
                });
            }
            out.push_str(&" ".repeat(l.width - w + LOGO_GAP));
        }
        if let Some(line) = lines.get(i) {
            out.push_str(line);
        }
        out.push('\n');
    }
    out
}

fn total_lines(sections: &[Section], base: usize) -> usize {
    base + sections.iter().map(|s| 1 + s.rows.len()).sum::<usize>() + sections.len().saturating_sub(1)
}

/// Drop what matters least until the list fits `max_lines`: the lowest-priority rows first, then whole
/// sections from the end.
fn fit(sections: &mut Vec<Section>, max_lines: usize, base: usize) {
    while total_lines(sections, base) > max_lines && !sections.is_empty() {
        let mut worst: Option<(u8, usize, usize)> = None;
        for (si, s) in sections.iter().enumerate() {
            for (ri, r) in s.rows.iter().enumerate() {
                if r.prio > 0 && worst.is_none_or(|w| (r.prio, si, ri) > w) {
                    worst = Some((r.prio, si, ri));
                }
            }
        }
        match worst {
            Some((_, si, ri)) => {
                sections[si].rows.remove(ri);
            }
            None => {
                sections.pop();
            }
        }
        sections.retain(|s| !s.rows.is_empty());
    }
}

fn trunc(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        format!("{}…", s.chars().take(max.saturating_sub(1)).collect::<String>())
    }
}

fn row(label: &'static str, value: impl Into<String>, prio: u8) -> Row {
    Row { label, value: value.into(), pct: None, prio }
}

/// "9.9 / 31.3 GiB (32%)": used and total in the total's unit, then how full it is.
fn usage_row(label: &'static str, used_gib: f64, total_gib: f64, prio: u8) -> Row {
    let pct = ((used_gib / total_gib) * 100.0).round().clamp(0.0, 100.0) as u32;
    let mut r = row(label, fmt_usage(used_gib, total_gib), prio);
    r.pct = Some(pct);
    r
}

pub fn fmt_usage(used_gib: f64, total_gib: f64) -> String {
    let pct = ((used_gib / total_gib) * 100.0).round().clamp(0.0, 100.0) as u32;
    if total_gib >= 1024.0 {
        format!("{:.2} / {:.2} TiB ({}%)", used_gib / 1024.0, total_gib / 1024.0, pct)
    } else {
        format!("{:.1} / {:.1} GiB ({}%)", used_gib, total_gib, pct)
    }
}

/// The running kernel as it is: "Nox 1.2.0 (Linux 7.3.0)", "SegNEL 1.2.0 (Linux 7.3.0)", else "Linux <release>".
pub fn kernel_label(release: &str) -> String {
    let base: String = release.chars().take_while(|c| c.is_ascii_digit() || *c == '.').collect();
    let base = base.trim_end_matches('.');
    let lower = release.to_lowercase();
    for (key, name) in [("nox", "Nox"), ("segnel", "SegNEL")] {
        if let Some(i) = lower.find(key) {
            let ver: String = release[i + key.len()..]
                .trim_start_matches(['-', '_', '.', ' '])
                .chars()
                .take_while(|c| c.is_ascii_digit() || *c == '.')
                .collect();
            let ver = ver.trim_end_matches('.');
            if !ver.is_empty() {
                return format!("{} {} (Linux {})", name, ver, base);
            }
        }
    }
    format!("Linux {}", release)
}

/// "1920x1080" to "1920×1080".
fn fmt_monitor(m: &census_core::MonitorInfo) -> String {
    let name = if m.model.is_empty() { m.name.clone() } else { m.model.clone() };
    let mut out = format!("{} {}", name, m.resolution.replace('x', "×"));
    if (m.scale - 1.0).abs() > 0.005 {
        let sc = format!("{:.2}", m.scale);
        out.push_str(&format!(" @ {}x", sc.trim_end_matches('0').trim_end_matches('.')));
    }
    if let Some(d) = m.diagonal_in {
        out.push_str(&format!(", {}″", d));
    }
    if m.refresh_rate > 0 {
        out.push_str(&format!(", {} Hz", m.refresh_rate));
    }
    out
}

fn cap(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

fn fmt_uptime(secs: u64) -> String {
    let d = secs / 86400;
    let h = (secs % 86400) / 3600;
    let m = (secs % 3600) / 60;
    if d > 0 {
        format!("{}d {}h {}m", d, h, m)
    } else if h > 0 {
        format!("{}h {}m", h, m)
    } else {
        format!("{}m", m)
    }
}

fn sections(s: &Snapshot, config: &Config, opts: &Options) -> Vec<Section> {
    let mut out = Vec::new();
    for name in &config.layout.sections {
        let section = match name.as_str() {
            "system" => system(s),
            "hardware" => hardware(s),
            "session" => session(s, opts),
            "audio" => audio(s),
            "network" => network(s),
            _ => continue,
        };
        if !section.rows.is_empty() {
            out.push(section);
        }
    }
    out
}

fn system(s: &Snapshot) -> Section {
    let p = &s.platform;
    let mut rows = vec![row("OS", p.seganos.clone().unwrap_or_else(|| s.system.os.clone()), 0)];
    if let Some(h) = &s.hardware.host {
        rows.push(row("Host", h.clone(), 1));
    }
    rows.push(row("Kernel", kernel_label(&s.system.kernel), 0));
    if let Some(m) = &p.kerneld_mode {
        rows.push(row("Profile", cap(m), 1));
    }
    rows.push(row("Uptime", fmt_uptime(s.system.uptime_secs), 0));
    let sw = &s.software;
    let pkgs: Vec<String> = [
        (sw.pacman, "pacman"),
        (sw.flatpak, "flatpak"),
        (sw.appimage, "AppImage"),
        (sw.snap, "snap"),
        (sw.cargo, "cargo"),
    ]
    .iter()
    .filter(|(n, _)| *n > 0)
    .map(|(n, name)| format!("{} {}", n, name))
    .collect();
    if !pkgs.is_empty() {
        rows.push(row("Packages", pkgs.join(", "), 2));
    }
    Section { title: "System", rows }
}

fn hardware(s: &Snapshot) -> Section {
    let h = &s.hardware;
    let mut rows = Vec::new();
    if h.cpu != "Unknown CPU" {
        let freq = h.cpu_max_mhz.filter(|m| *m > 0).map(|m| format!(" @ {:.2} GHz", m as f64 / 1000.0)).unwrap_or_default();
        rows.push(row("CPU", format!("{} ({}C/{}T){}", h.cpu, h.cpu_cores_physical, h.cpu_cores_logical, freq), 0));
    }
    if h.gpu != "Unknown GPU" && h.gpu != "Unknown" {
        rows.push(row("GPU", h.gpu.clone(), 0));
    }
    let driver: Vec<String> = h
        .gpu_driver
        .iter()
        .cloned()
        .chain(h.vulkan.iter().map(|v| format!("Vulkan {}", v)))
        .collect();
    if !driver.is_empty() {
        rows.push(row("Driver", driver.join(" · "), 1));
    }
    if h.ram_total_gib > 0.0 {
        rows.push(usage_row("Memory", h.ram_used_gib, h.ram_total_gib, 0));
    }
    if h.swap_total_gib > 0.0 {
        rows.push(usage_row("Swap", h.swap_used_gib, h.swap_total_gib, 1));
    }
    for (i, d) in h.disks.iter().enumerate() {
        if d.total_gib <= 0.0 {
            continue;
        }
        let mut r = usage_row("Disk", d.used_gib, d.total_gib, if i == 0 { 0 } else { 2 });
        r.value = format!("{} · {}", r.value, d.label);
        rows.push(r);
    }
    Section { title: "Hardware", rows }
}

fn session(s: &Snapshot, opts: &Options) -> Section {
    let mut rows = Vec::new();
    let desktop = std::env::var("XDG_CURRENT_DESKTOP").ok().filter(|d| !d.is_empty());
    match (&s.platform.fenestra, desktop.or_else(|| Some(s.system.wm.clone()).filter(|w| w != "unknown"))) {
        (Some(f), _) => rows.push(row("Compositor", f.clone(), 0)),
        (None, Some(d)) => rows.push(row("Desktop", d, 0)),
        _ => {}
    }
    let shell = std::env::var("SHELL").unwrap_or_else(|_| s.system.shell.clone());
    let shell = shell.rsplit('/').next().unwrap_or("").to_string();
    if !shell.is_empty() && shell != "unknown" {
        rows.push(row("Shell", shell, 2));
    }
    if let Some(t) = opts.terminal.as_ref().filter(|t| !t.is_empty()) {
        rows.push(row("Terminal", t.clone(), 2));
    }
    for (i, m) in s.display.monitors.iter().enumerate() {
        rows.push(row("Display", fmt_monitor(m), if i == 0 { 1 } else { 2 }));
    }
    Section { title: "Session", rows }
}

fn audio(s: &Snapshot) -> Section {
    let mut rows = Vec::new();
    if let Some(a) = &s.platform.audio {
        rows.push(row(
            "PipeWire",
            format!("{} kHz · {} frames · {:.1} ms", trim_khz(a.rate), a.quantum, a.latency_ms),
            0,
        ));
    }
    if let Some(g) = &s.platform.gpmix {
        let v = match (g.running, &g.version) {
            (true, Some(v)) => format!("Running {}", v),
            (true, None) => "Running".to_string(),
            (false, _) => "Not running".to_string(),
        };
        rows.push(row("GP Mix", v, 1));
    }
    Section { title: "Audio", rows }
}

fn trim_khz(rate: u32) -> String {
    let k = rate as f64 / 1000.0;
    if (k - k.round()).abs() < 1e-9 {
        format!("{}", k as u32)
    } else {
        format!("{:.1}", k)
    }
}

/// Only filled when the user turned the addresses on in `[privacy]`.
fn network(s: &Snapshot) -> Section {
    let mut rows = Vec::new();
    if let Some(ip) = &s.network.lan_ip {
        rows.push(row("LAN", ip.clone(), 0));
    }
    if let Some(ip) = &s.network.public_ip {
        rows.push(row("Public", ip.clone(), 0));
    }
    if let Some(ip) = &s.network.tailnet_ip {
        rows.push(row("Tailnet", ip.clone(), 0));
    }
    Section { title: "Network", rows }
}

#[cfg(test)]
mod tests {
    use super::*;
    use census_core::MonitorInfo;

    fn mon(model: &str, res: &str, scale: f64, diag: Option<f64>, hz: u32) -> MonitorInfo {
        MonitorInfo { name: "DP-1".into(), resolution: res.into(), refresh_rate: hz, primary: false, model: model.into(), scale, diagonal_in: diag }
    }

    #[test]
    fn monitor_lines() {
        assert_eq!(fmt_monitor(&mon("R24e", "1080x1920", 1.77, Some(24.0), 60)), "R24e 1080×1920 @ 1.77x, 24″, 60 Hz");
        assert_eq!(fmt_monitor(&mon("N27q", "2560x1440", 1.0, Some(27.0), 60)), "N27q 2560×1440, 27″, 60 Hz");
        assert_eq!(fmt_monitor(&mon("", "1920x1080", 1.5, None, 0)), "DP-1 1920×1080 @ 1.5x");
        assert_eq!(fmt_monitor(&mon("X", "1920x1080", 2.0, None, 144)), "X 1920×1080 @ 2x, 144 Hz");
    }

    #[test]
    fn kernel_naming() {
        assert_eq!(kernel_label("7.3.0-nox1.2.0"), "Nox 1.2.0 (Linux 7.3.0)");
        assert_eq!(kernel_label("7.3.0-segnel1.2.0"), "SegNEL 1.2.0 (Linux 7.3.0)");
        assert_eq!(kernel_label("6.12.1-arch1-1"), "Linux 6.12.1-arch1-1");
    }

    #[test]
    fn usage_thresholds() {
        assert_eq!(usage_color(0), "\x1b[32m");
        assert_eq!(usage_color(59), "\x1b[32m");
        assert_eq!(usage_color(60), "\x1b[33m");
        assert_eq!(usage_color(84), "\x1b[33m");
        assert_eq!(usage_color(85), "\x1b[31m");
        assert_eq!(usage_color(100), "\x1b[31m");
    }

    #[test]
    fn usage_text_and_paint() {
        assert_eq!(fmt_usage(9.9, 31.3), "9.9 / 31.3 GiB (32%)");
        assert_eq!(fmt_usage(0.0, 1833.0), "0.00 / 1.79 TiB (0%)");
        let r = usage_row("Memory", 28.0, 31.0, 0);
        let s = Section { title: "t", rows: vec![r] };
        let colored = row_lines(&s, 6, 60, &Style { color: true }).join("");
        assert!(colored.contains("(\x1b[31m90%\x1b[0m)"), "{:?}", colored);
        let plain = row_lines(&s, 6, 60, &Style { color: false }).join("");
        assert!(plain.contains("(90%)") && !plain.contains('\x1b'));
    }
}
