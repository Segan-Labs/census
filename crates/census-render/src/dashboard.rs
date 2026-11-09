use census_core::{Snapshot, Config};
use census_widgets::{Canvas, Card, Widget, Rect};
use census_layout::{LayoutEngine, detect_terminal_width};

pub fn render(snapshot: &Snapshot, config: &Config) -> String {
    let term_width = detect_terminal_width();
    let engine = LayoutEngine::new(config.layout.clone(), term_width);
    let card_widths = engine.calculate_columns();

    // Build cards first
    let mut left_cards: Vec<Card> = Vec::new();
    let mut right_cards: Vec<Card> = Vec::new();

    for row_sections in &engine.config.columns {
        if let Some(section) = row_sections.first() {
            left_cards.push(render_card(section, snapshot, card_widths[0]));
        }
        if let Some(section) = row_sections.get(1) {
            right_cards.push(render_card(section, snapshot, *card_widths.get(1).unwrap_or(&48)));
        }
    }

    // Calculate row heights
    let gap = 4;
    let num_rows = left_cards.len().max(right_cards.len());
    let mut row_heights: Vec<usize> = Vec::new();

    for i in 0..num_rows {
        let left_h = left_cards.get(i).map(|c| c.measure().height).unwrap_or(0);
        let right_h = right_cards.get(i).map(|c| c.measure().height).unwrap_or(0);
        row_heights.push(left_h.max(right_h));
    }

    // Calculate canvas size
    let total_height = row_heights.iter().sum::<usize>() + num_rows + 5;
    let total_width = card_widths.iter().sum::<usize>() + gap * card_widths.len().saturating_sub(1);

    let mut canvas = Canvas::new(total_width + 10, total_height);

    // Header
    let header = format!(
        "\u{f007} {}@{}",
        std::env::var("USER").unwrap_or_else(|_| "user".into()),
        snapshot.system.hostname
    );
    let pad_left = (total_width.saturating_sub(header.len())) / 2;
    canvas.draw_str(pad_left, 0, &header);

    // Render rows
    let mut y = 2;
    for (i, &h) in row_heights.iter().enumerate() {
        if let Some(card) = left_cards.get(i) {
            let rect = Rect { x: 0, y, width: card_widths[0], height: h };
            card.draw(&mut canvas, &rect);
        }

        if let Some(card) = right_cards.get(i) {
            let x = card_widths[0] + gap;
            let w = *card_widths.get(1).unwrap_or(&48);
            let rect = Rect { x, y, width: w, height: h };
            card.draw(&mut canvas, &rect);
        }

        y += h + 1;
    }

    // Trim trailing empty lines
    canvas.trim_trailing_empty();

    canvas.to_text()
}

fn render_card(section: &str, snapshot: &Snapshot, width: usize) -> Card {
    match section {
        "system" => card_system(snapshot, width),
        "status" => card_status(snapshot, width),
        "hardware" => card_hardware(snapshot, width),
        "drive" => card_drive(snapshot, width),
        "software" => card_software(snapshot, width),
        "display" => card_display(snapshot, width),
        "network" => card_network(snapshot, width),
        "sensors" => card_sensors(snapshot, width),
        _ => Card::new("?", vec![], width),
    }
}

fn card_system(s: &Snapshot, w: usize) -> Card {
    let rows = vec![
        kv(&s.system.distro_icon, "OS", &s.system.os),
        kv("\u{e62e}", "Kernel", &s.system.kernel),
        kv("\u{f233}", "WM", &s.system.wm),
        kv("\u{e795}", "Shell", &s.system.shell),
    ];
    Card::new("System", rows, w)
}

fn card_status(s: &Snapshot, w: usize) -> Card {
    let packages = s.software.pacman + s.software.aur + s.software.cargo + s.software.node;
    let rows = vec![
        kv("\u{f0996}", "Uptime", &fmt_uptime(s.system.uptime_secs)),
        kv("\u{f437}", "Load", &format!("{:.2} {:.2} {:.2}", s.system.load_avg[0], s.system.load_avg[1], s.system.load_avg[2])),
        kv("\u{f0e93}", "OS Age", &format!("{} days", s.system.os_age_days.unwrap_or(0))),
        kv("\u{f18c}", "Packages", &packages.to_string()),
    ];
    Card::new("Status", rows, w)
}

fn card_hardware(s: &Snapshot, w: usize) -> Card {
    let mut rows = vec![
        kv("\u{f015}", "Mobo", &trunc(&s.hardware.mobo, 30)),
        kv("\u{f2db}", "CPU", &format!("{} ({}C/{}T)", trunc(&s.hardware.cpu, 24), s.hardware.cpu_cores_physical, s.hardware.cpu_cores_logical)),
        kv("\u{f08ae}", "GPU", &trunc(&s.hardware.gpu, 30)),
    ];

    let ram_pct = if s.hardware.ram_total_gib > 0.0 { s.hardware.ram_used_gib / s.hardware.ram_total_gib } else { 0.0 };
    rows.push(format!("\u{f0e4} RAM     \u{2192} {} {:.2}G / {:.2}G", bar_str(ram_pct, 12), s.hardware.ram_used_gib, s.hardware.ram_total_gib));

    let swap_pct = if s.hardware.swap_total_gib > 0.0 { s.hardware.swap_used_gib / s.hardware.swap_total_gib } else { 0.0 };
    rows.push(format!("\u{f187} SWAP    \u{2192} {} {:.2}G / {:.2}G", bar_str(swap_pct, 12), s.hardware.swap_used_gib, s.hardware.swap_total_gib));

    Card::new("Hardware", rows, w)
}

fn card_drive(s: &Snapshot, w: usize) -> Card {
    let mut rows = Vec::new();
    for disk in &s.hardware.disks {
        let pct = if disk.total_gib > 0.0 { disk.used_gib / disk.total_gib } else { 0.0 };
        rows.push(format!(
            "\u{f0a0} {:<7} \u{2192} {} {:.2}G / {:.2}G",
            disk.label,
            bar_str(pct, 12),
            disk.used_gib,
            disk.total_gib
        ));
    }
    Card::new("Drive", rows, w)
}

fn card_software(s: &Snapshot, w: usize) -> Card {
    let mut rows = Vec::new();
    if s.software.pacman > 0 { rows.push(kv("\u{f18c}", "Pacman", &s.software.pacman.to_string())); }
    if s.software.aur > 0 { rows.push(kv("\u{f303}", "AUR", &s.software.aur.to_string())); }
    if s.software.cargo > 0 { rows.push(kv("\u{e7a8}", "Cargo", &s.software.cargo.to_string())); }
    if s.software.node > 0 { rows.push(kv("\u{e718}", "Node", &s.software.node.to_string())); }
    if s.software.flatpak > 0 { rows.push(kv("\u{f300}", "Flatpak", &s.software.flatpak.to_string())); }
    if s.software.appimage > 0 { rows.push(kv("\u{f300}", "AppImage", &s.software.appimage.to_string())); }
    if s.software.snap > 0 { rows.push(kv("\u{e731}", "Snap", &s.software.snap.to_string())); }
    if let Some(v) = &s.software.rust_version { rows.push(kv("\u{e7a8}", "Rust", v)); }
    Card::new("Software", rows, w)
}

fn card_display(s: &Snapshot, w: usize) -> Card {
    let rows: Vec<String> = s.display.monitors.iter().map(|m| {
        let tag = if m.primary { " (Primary)" } else { "" };
        format!("\u{f26c}  {:<10} \u{2192} {}@{}Hz{}", m.name, m.resolution, m.refresh_rate, tag)
    }).collect();
    Card::new("Display", rows, w)
}

fn card_network(s: &Snapshot, w: usize) -> Card {
    let mut rows = Vec::new();
    if let Some(ip) = &s.network.lan_ip { rows.push(kv("\u{f0ac}", "LAN", ip)); }
    if let Some(ip) = &s.network.public_ip { rows.push(kv("\u{f0ac}", "Public", ip)); }
    if let Some(ip) = &s.network.tailnet_ip { rows.push(kv("\u{f0ac}", "Tailnet", ip)); }
    Card::new("Network", rows, w)
}

fn card_sensors(s: &Snapshot, w: usize) -> Card {
    let mut rows = Vec::new();
    if let Some(t) = s.sensors.cpu_temp_c {
        let color = if t < 50.0 { "\x1b[32m" } else if t < 70.0 { "\x1b[33m" } else { "\x1b[31m" };
        rows.push(format!("\u{f0e4} CPU    \u{2192} {}{:.0}\u{00b0}C\x1b[0m", color, t));
    }
    if let Some(t) = s.sensors.gpu_temp_c {
        let color = if t < 50.0 { "\x1b[32m" } else if t < 70.0 { "\x1b[33m" } else { "\x1b[31m" };
        rows.push(format!("\u{f08ae} GPU    \u{2192} {}{:.0}\u{00b0}C\x1b[0m", color, t));
    }
    Card::new("Sensors", rows, w)
}

fn kv(icon: &str, label: &str, value: &str) -> String {
    format!("{} {:<7} \u{2192} {}", icon, label, value)
}

fn trunc(s: &str, max: usize) -> String {
    if s.chars().count() <= max { s.to_string() } else {
        format!("{}…", s.chars().take(max - 1).collect::<String>())
    }
}

fn fmt_uptime(secs: u64) -> String {
    let h = secs / 3600;
    let m = (secs % 3600) / 60;
    if h > 0 { format!("{}h {}m", h, m) } else { format!("{}m", m) }
}

fn bar_str(pct: f64, width: usize) -> String {
    let filled = (pct.clamp(0.0, 1.0) * width as f64).round() as usize;
    let color = if pct < 0.5 { "\x1b[32m" } else if pct < 0.75 { "\x1b[33m" } else { "\x1b[31m" };
    format!("{}{}{}\x1b[0m{}", color, "\u{2588}".repeat(filled), "\u{2591}".repeat(width - filled), "")
}
