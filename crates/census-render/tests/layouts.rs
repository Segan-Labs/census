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
fn removable_media_are_hidden_unless_asked() {
    let out = lines(140, Some(50), "classic").join("\n");
    assert!(out.contains("· /"), "{}", out);
    assert!(!out.contains("Ventoy"), "a USB stick is not a Disk by default");
    let mut config = Config::default();
    config.layout.removable_disks = true;
    let shown = render_to_string(&snapshot(), &config, &opts(140, Some(50))).unwrap();
    assert!(shown.contains("Ventoy"), "{}", shown);
}

/// Where the list starts, in columns, and how many columns the logo's widest row takes.
fn logo_and_list_columns(out: &[String]) -> (usize, usize) {
    let list_col = out[0].find("segan@").map(|b| out[0][..b].chars().count()).unwrap();
    let widest = out
        .iter()
        .map(|l| l.chars().take(list_col).collect::<String>().trim_end().chars().count())
        .max()
        .unwrap();
    (widest, list_col)
}

#[test]
fn logo_takes_the_height_of_the_list() {
    let out = lines(140, Some(50), "classic");
    let (widest, list_col) = logo_and_list_columns(&out);
    let logo_rows = out.iter().filter(|l| l.chars().take(list_col).any(|c| c != ' ')).count();
    assert!(out.len() > 20, "{} rows", out.len());
    assert_eq!(logo_rows, out.len(), "every list row has logo beside it");
    assert!(out.last().unwrap().starts_with("T+12:04:09"), "{:?}", out.last());
    // Five columns of air between the logo's widest row and the list.
    assert_eq!(list_col, widest + 5, "logo ends at {}, list at {}", widest, list_col);
}

#[test]
fn logo_shrinks_before_the_list_is_cut() {
    // At 96 columns the full-height logo would leave the list no room, so it is drawn shorter.
    let out = lines(96, Some(50), "classic");
    let (_, list_col) = logo_and_list_columns(&out);
    let logo_rows = out.iter().filter(|l| l.chars().take(list_col).any(|c| c != ' ')).count();
    assert!(logo_rows >= 14 && logo_rows < out.len(), "{} logo rows of {}", logo_rows, out.len());
    assert!(out.iter().any(|l| l.contains("Vulkan 1.4.351")), "the list keeps its full width");
}

#[test]
fn unknown_layout_is_an_error() {
    let mut o = opts(80, None);
    o.style = Some("grid".into());
    assert!(render_to_string(&snapshot(), &Config::default(), &o).is_err());
}
