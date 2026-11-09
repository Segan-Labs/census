use census_core::{Config, Snapshot};
use census_render::{render_to_string, Options};

fn snapshot() -> Snapshot {
    serde_json::from_str(include_str!("snapshot.json")).unwrap()
}

fn opts(width: usize, height: Option<usize>) -> Options {
    Options { width, height, color: false, ..Options::default() }
}

fn lines(width: usize, height: Option<usize>, style: &str) -> Vec<String> {
    let mut o = opts(width, height);
    o.style = Some(style.to_string());
    render_to_string(&snapshot(), &Config::default(), &o)
        .unwrap()
        .lines()
        .map(|l| l.to_string())
        .collect()
}

#[test]
fn wide_terminal_has_logo_left_and_list_right() {
    let out = lines(100, Some(40), "classic");
    assert!(out[0].contains("@host"), "header beside the logo: {:?}", out[0]);
    assert!(out.iter().any(|l| l.contains("Nox 1.2.0")));
    assert!(out.iter().any(|l| l.contains("Fenestra 1.0")));
    assert!(out.iter().any(|l| l.contains("1.3 ms")));
    assert!(out[0].starts_with(' '), "logo columns come first");
}

#[test]
fn narrow_terminal_drops_the_logo() {
    let out = lines(60, Some(40), "classic");
    assert!(out[0].starts_with("segan") || out[0].contains("@host"));
    assert!(out[0].starts_with(|c: char| c.is_alphanumeric()), "list flush left: {:?}", out[0]);
}

#[test]
fn short_terminal_shows_fewer_lines() {
    let out = lines(100, Some(12), "classic");
    assert!(out.len() < 12, "{} lines for 12 rows", out.len());
    assert!(out.iter().any(|l| l.contains("OS")));
}

#[test]
fn nothing_private_by_default() {
    let all = lines(100, Some(60), "classic").join("\n");
    for word in ["Public", "Tailnet", "LAN"] {
        assert!(!all.contains(word), "{} must be opt-in", word);
    }
}

#[test]
fn no_box_drawing_in_the_default_layout() {
    let all = lines(100, Some(60), "classic").join("\n");
    for c in ['┌', '┐', '└', '┘', '│', '█', '░'] {
        assert!(!all.contains(c));
    }
}

#[test]
fn unknown_layout_is_an_error() {
    let mut o = opts(80, None);
    o.style = Some("grid".into());
    assert!(render_to_string(&snapshot(), &Config::default(), &o).is_err());
}
