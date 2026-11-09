use census_core::Snapshot;

const ECLIPSE: &str = include_str!("../assets/eclipse.txt");
const ECLIPSE_SMALL: &str = include_str!("../assets/eclipse-small.txt");
const NEUTRAL: &str = "    .-\"\"\"-.\n  .'       '.\n /           \\\n|             |\n \\           /\n  '.       .'\n    '-...-'";

pub struct Logo {
    pub lines: Vec<&'static str>,
    pub width: usize,
}

impl Logo {
    fn from_text(text: &'static str) -> Self {
        let lines: Vec<&'static str> = text.lines().map(|l| l.trim_end()).collect();
        let width = lines.iter().map(|l| l.chars().count()).max().unwrap_or(0);
        Self { lines, width }
    }

    pub fn height(&self) -> usize {
        self.lines.len()
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
    Some(Logo::from_text(match (eclipse, small) {
        (true, false) => ECLIPSE,
        (true, true) => ECLIPSE_SMALL,
        (false, _) => NEUTRAL,
    }))
}
