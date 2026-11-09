//! The text logos. An art file is plain text; an optional tones file of the same shape says how each
//! cell is painted: `d` dim (the shadowed disc, the stars), `b` bold accent (the core of the crescent),
//! `f` the terminal's own foreground (the orbit), anything else the accent colour.

use census_core::Snapshot;

const ECLIPSE: &str = include_str!("../assets/eclipse.txt");
const ECLIPSE_TONES: &str = include_str!("../assets/eclipse-tones.txt");
const ECLIPSE_SMALL: &str = include_str!("../assets/eclipse-small.txt");
const ECLIPSE_SMALL_TONES: &str = include_str!("../assets/eclipse-small-tones.txt");
const NEUTRAL: &str = "    .-\"\"\"-.\n  .'       '.\n /           \\\n|             |\n \\           /\n  '.       .'\n    '-...-'";

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tone {
    Accent,
    Bold,
    Dim,
    Plain,
}

pub struct Logo {
    pub lines: Vec<&'static str>,
    tones: Vec<&'static str>,
    pub width: usize,
}

impl Logo {
    fn from_text(text: &'static str, tones: &'static str) -> Self {
        let lines: Vec<&'static str> = text.lines().map(|l| l.trim_end()).collect();
        let width = lines.iter().map(|l| l.chars().count()).max().unwrap_or(0);
        Self { lines, tones: tones.lines().collect(), width }
    }

    pub fn height(&self) -> usize {
        self.lines.len()
    }

    /// Row `i` as runs of (tone, text); empty past the last row.
    pub fn row(&self, i: usize) -> Vec<(Tone, String)> {
        let Some(line) = self.lines.get(i) else { return Vec::new() };
        let mut tones = self.tones.get(i).unwrap_or(&"").chars();
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

/// The logo for `choice` (`auto`, `eclipse`, `neutral`, `none`); `small` is the one for the compact layout.
/// `auto` is the Eclipse on SeganOS and a neutral mark everywhere else.
pub fn pick(choice: &str, snapshot: &Snapshot, small: bool) -> Option<Logo> {
    let seganos = snapshot.platform.seganos.is_some() || snapshot.platform.nox.is_some();
    let eclipse = match choice {
        "eclipse" => true,
        "neutral" => false,
        "none" => return None,
        _ => seganos,
    };
    Some(match (eclipse, small) {
        (true, false) => Logo::from_text(ECLIPSE, ECLIPSE_TONES),
        (true, true) => Logo::from_text(ECLIPSE_SMALL, ECLIPSE_SMALL_TONES),
        (false, _) => Logo::from_text(NEUTRAL, ""),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eclipse_art_matches_its_tones_and_fits() {
        for (art, tones, rows, cols) in [(ECLIPSE, ECLIPSE_TONES, 19..=24, 46), (ECLIPSE_SMALL, ECLIPSE_SMALL_TONES, 8..=10, 22)] {
            let logo = Logo::from_text(art, tones);
            assert!(logo.width <= cols, "{} columns", logo.width);
            assert!(rows.contains(&logo.height()), "{} rows", logo.height());
            assert_eq!(logo.lines.len(), logo.tones.len(), "the tones file has the art's rows");
            for (i, line) in logo.lines.iter().enumerate() {
                let joined: String = logo.row(i).into_iter().map(|(_, s)| s).collect();
                assert_eq!(&joined, line);
            }
        }
    }

    #[test]
    fn eclipse_uses_every_tone() {
        let all: String = [ECLIPSE_TONES, ECLIPSE_SMALL_TONES].concat();
        for t in ['a', 'b', 'd', 'f'] {
            assert!(all.contains(t), "no {} tone", t);
        }
    }

    #[test]
    fn tones_split_a_row_into_runs() {
        let logo = Logo { lines: vec!["@#. .  ."], tones: vec!["aaa d  d"], width: 8 };
        assert_eq!(logo.row(0), vec![(Tone::Accent, "@#. ".to_string()), (Tone::Dim, ".  .".to_string())]);
        let logo = Logo { lines: vec!["@%+ ·"], tones: vec!["bba f"], width: 5 };
        assert_eq!(
            logo.row(0),
            vec![(Tone::Bold, "@%".to_string()), (Tone::Accent, "+ ".to_string()), (Tone::Plain, "·".to_string())]
        );
        assert!(logo.row(1).is_empty());
        let plain = Logo::from_text(NEUTRAL, "");
        assert_eq!(plain.row(0), vec![(Tone::Accent, "    .-\"\"\"-.".to_string())]);
    }
}
