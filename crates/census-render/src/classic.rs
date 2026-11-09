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

pub fn render(snapshot: &Snapshot, config: &Config, opts: &Options, compact: bool) -> String {
    let st = Style { color: opts.color };
    let width = opts.width;
    let mut sections = sections(snapshot, config);
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
            let value = trunc(&r.value, room);
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
                    Tone::Dim => st.dim(&run),
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
    Row { label, value: value.into(), prio }
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

fn sections(s: &Snapshot, config: &Config) -> Vec<Section> {
    let mut out = Vec::new();
    for name in &config.layout.sections {
        let section = match name.as_str() {
            "system" => system(s),
            "hardware" => hardware(s),
            "session" => session(s),
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
    let release = &s.system.kernel;
    let base: String = release.chars().take_while(|c| c.is_ascii_digit() || *c == '.').collect();
    rows.push(row(
        "Kernel",
        match &p.nox {
            Some(v) => format!("Nox {} (Linux {})", v, base.trim_end_matches('.')),
            None => format!("Linux {}", release),
        },
        0,
    ));
    if let Some(m) = &p.kerneld_mode {
        rows.push(row("Profile", cap(m), 1));
    }
    rows.push(row("Uptime", fmt_uptime(s.system.uptime_secs), 0));
    let sw = &s.software;
    let pkgs: Vec<String> = [
        (sw.pacman, "pacman"),
        (sw.flatpak, "flatpak"),
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
        rows.push(row("CPU", format!("{} ({}C/{}T)", h.cpu, h.cpu_cores_physical, h.cpu_cores_logical), 0));
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
        rows.push(row("Memory", format!("{:.1} / {:.1} GiB", h.ram_used_gib, h.ram_total_gib), 0));
    }
    for (i, d) in h.disks.iter().take(3).enumerate() {
        rows.push(row("Disk", format!("{:.0} / {:.0} GiB ({})", d.used_gib, d.total_gib, d.label), if i == 0 { 0 } else { 2 }));
    }
    Section { title: "Hardware", rows }
}

fn session(s: &Snapshot) -> Section {
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
    let many = s.display.monitors.len() > 1;
    for (i, m) in s.display.monitors.iter().enumerate() {
        let name = if many { format!(" ({})", m.name) } else { String::new() };
        rows.push(row("Display", format!("{} @ {} Hz{}", m.resolution, m.refresh_rate, name), if i == 0 { 1 } else { 2 }));
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
