//! The logos. The Eclipse is drawn at run time (see `eclipse`) at the height the layout asks for and
//! kept in a small on-disk cache, so a warm `census` only reads a file. A tone letter per cell says
//! how it is painted: `d` dim (the shadowed disc, the stars), `b` bold accent (the core of the
//! crescent), `f` the terminal's own foreground (the orbit), anything else the accent colour.

use crate::eclipse;
use census_core::Snapshot;

const NEUTRAL: &str = "    .-\"\"\"-.\n  .'       '.\n /           \\\n|             |\n \\           /\n  '.       .'\n    '-...-'";

/// The Eclipse is never drawn shorter or taller than this in the classic layout.
pub const MIN_ROWS: usize = 14;
pub const MAX_ROWS: usize = 40;
/// The compact layout's logo, beside the first section.
pub const SMALL_MIN_ROWS: usize = 8;
pub const SMALL_MAX_ROWS: usize = 12;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tone {
    Accent,
    Bold,
    Dim,
    Plain,
}

pub struct Logo {
    pub lines: Vec<String>,
    tones: Vec<String>,
    pub width: usize,
}

impl Logo {
    fn from_text(text: &str, tones: &str) -> Self {
        let lines: Vec<String> = text.lines().map(|l| l.trim_end().to_string()).collect();
        let width = lines.iter().map(|l| l.chars().count()).max().unwrap_or(0);
        Self { lines, tones: tones.lines().map(str::to_string).collect(), width }
    }

    fn from_art(art: eclipse::Art) -> Self {
        Self { lines: art.lines, tones: art.tones, width: art.cols }
    }

    pub fn height(&self) -> usize {
        self.lines.len()
    }

    /// Row `i` as runs of (tone, text); empty past the last row.
    pub fn row(&self, i: usize) -> Vec<(Tone, String)> {
        let Some(line) = self.lines.get(i) else { return Vec::new() };
        let mut tones = self.tones.get(i).map(|t| t.as_str()).unwrap_or("").chars();
        let mut runs: Vec<(Tone, String)> = Vec::new();
        for ch in line.chars() {
            let tone = match tones.next() {
                Some('d') => Tone::Dim,
                Some('b') => Tone::Bold,
                Some('f') => Tone::Plain,
                _ => Tone::Accent,
            };
            match runs.last_mut() {
                // A space has no colour, so it joins whichever run is open.
                Some((t, s)) if *t == tone || ch == ' ' => s.push(ch),
                _ => runs.push((tone, ch.to_string())),
            }
        }
        runs
    }
}

/// Which logo `choice` (`auto`, `eclipse`, `neutral`, `none`) means on this machine.
/// `auto` is the Eclipse on SeganOS and a neutral mark everywhere else.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Choice {
    Eclipse,
    Neutral,
    None,
}

pub fn choose(choice: &str, snapshot: &Snapshot) -> Choice {
    let seganos = snapshot.platform.seganos.is_some() || snapshot.platform.nox.is_some();
    match choice {
        "eclipse" => Choice::Eclipse,
        "neutral" => Choice::Neutral,
        "none" => Choice::None,
        _ if seganos => Choice::Eclipse,
        _ => Choice::Neutral,
    }
}

/// The columns the logo takes at `rows` rows, without drawing it.
pub fn width_for(choice: Choice, rows: usize, aspect: f64, small: bool) -> usize {
    match choice {
        Choice::Eclipse => eclipse::width_for(rows, aspect, small),
        Choice::Neutral => neutral().width,
        Choice::None => 0,
    }
}

/// The logo at `rows` rows (the neutral mark has its own height).
pub fn build(choice: Choice, rows: usize, aspect: f64, small: bool) -> Option<Logo> {
    match choice {
        Choice::Eclipse => Some(Logo::from_art(cached(rows, aspect, small))),
        Choice::Neutral => Some(neutral()),
        Choice::None => None,
    }
}

fn neutral() -> Logo {
    Logo::from_text(NEUTRAL, "")
}

/// The art for (rows, cell aspect, size), from `$CENSUS_CACHE_DIR` or the user's cache directory
/// when it was drawn before, else drawn now and saved for next time.
fn cached(rows: usize, aspect: f64, small: bool) -> eclipse::Art {
    let dir = std::env::var_os("CENSUS_CACHE_DIR")
        .map(std::path::PathBuf::from)
        .or_else(|| dirs::cache_dir().map(|d| d.join("census")));
    cached_in(dir, rows, aspect, small)
}

fn cached_in(dir: Option<std::path::PathBuf>, rows: usize, aspect: f64, small: bool) -> eclipse::Art {
    let cols = eclipse::width_for(rows, aspect, small);
    let file = dir.as_ref().map(|d| {
        d.join(format!(
            "eclipse-v{}-{}x{}-{:.2}{}.txt",
            eclipse::ART_VERSION,
            rows,
            cols,
            aspect,
            if small { "-small" } else { "" }
        ))
    });
    if let Some(art) = file.as_ref().and_then(|f| std::fs::read_to_string(f).ok()).and_then(|t| parse(&t, rows, cols)) {
        return art;
    }
    let art = eclipse::draw(rows, aspect, small);
    if let (Some(dir), Some(file)) = (dir, file) {
        if std::fs::create_dir_all(&dir).is_ok() {
            let mut text = String::with_capacity(rows * (cols + 1) * 2);
            for l in art.lines.iter().chain(art.tones.iter()) {
                text.push_str(l);
                text.push('\n');
            }
            let tmp = file.with_extension(format!("{}.tmp", std::process::id()));
            if std::fs::write(&tmp, text).is_ok() {
                let _ = std::fs::rename(&tmp, &file);
            }
        }
    }
    art
}

/// A cache file: `rows` art lines, then `rows` tone lines, each `cols` cells. Anything else is ignored.
fn parse(text: &str, rows: usize, cols: usize) -> Option<eclipse::Art> {
    let all: Vec<&str> = text.lines().collect();
    if all.len() != rows * 2 {
        return None;
    }
    let (lines, tones) = all.split_at(rows);
    if lines.iter().any(|l| l.chars().count() != cols) || tones.iter().any(|t| t.len() != cols) {
        return None;
    }
    Some(eclipse::Art {
        lines: lines.iter().map(|l| l.to_string()).collect(),
        tones: tones.iter().map(|t| t.to_string()).collect(),
        cols,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eclipse_rows_are_their_tones() {
        for (rows, small) in [(21, false), (34, false), (10, true)] {
            let logo = build(Choice::Eclipse, rows, 2.0, small).unwrap();
            assert_eq!(logo.height(), rows);
            assert_eq!(logo.lines.len(), logo.tones.len());
            for (i, line) in logo.lines.iter().enumerate() {
                let joined: String = logo.row(i).into_iter().map(|(_, s)| s).collect();
                assert_eq!(&joined, line);
            }
            let all: String = logo.tones.concat();
            for t in ['a', 'b', 'd', 'f'] {
                assert!(all.contains(t), "no {} tone at {} rows", t, rows);
            }
        }
    }

    #[test]
    fn cache_round_trips() {
        let dir = std::env::temp_dir().join(format!("census-logo-test-{}", std::process::id()));
        let at = |rows, aspect| cached_in(Some(dir.clone()), rows, aspect, false);
        let first = at(16, 2.2);
        let files: Vec<_> = std::fs::read_dir(&dir).unwrap().flatten().map(|e| e.file_name()).collect();
        assert_eq!(files.len(), 1, "{:?}", files);
        assert_eq!(files[0].to_string_lossy(), "eclipse-v2-16x38-2.20.txt");
        let again = at(16, 2.2);
        assert_eq!(first.lines, again.lines);
        assert_eq!(first.tones, again.tones);
        // A damaged file is ignored and replaced.
        std::fs::write(dir.join(&files[0]), "garbage\n").unwrap();
        let third = at(16, 2.2);
        assert_eq!(first.lines, third.lines);
        assert_eq!(std::fs::read_to_string(dir.join(&files[0])).unwrap().lines().count(), 32);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn tones_split_a_row_into_runs() {
        let logo = Logo { lines: vec!["@#. .  .".into()], tones: vec!["aaa d  d".into()], width: 8 };
        assert_eq!(logo.row(0), vec![(Tone::Accent, "@#. ".to_string()), (Tone::Dim, ".  .".to_string())]);
        let logo = Logo { lines: vec!["@%+ ·".into()], tones: vec!["bba f".into()], width: 5 };
        assert_eq!(
            logo.row(0),
            vec![(Tone::Bold, "@%".to_string()), (Tone::Accent, "+ ".to_string()), (Tone::Plain, "·".to_string())]
        );
        assert!(logo.row(1).is_empty());
        let plain = neutral();
        assert_eq!(plain.row(0), vec![(Tone::Accent, "    .-\"\"\"-.".to_string())]);
    }
}
