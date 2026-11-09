//! Which terminal the client runs in. The daemon has none, so this is read at render time:
//! `TERM_PROGRAM` (unless it is the generic "Terminal"), else the nearest ancestor process that is not a shell, multiplexer or wrapper.

/// Processes that sit between the terminal and `census`, or end the walk.
const SKIP: [&str; 14] = [
    "sh", "bash", "zsh", "fish", "nu", "dash", "ksh", "tmux", "tmux: server", "screen", "sudo", "su", "script", "env",
];
const STOP: [&str; 6] = ["systemd", "init", "login", "sshd", "sshd-session", "(sd-pam)"];

pub fn detect() -> Option<String> {
    if let Some(t) = std::env::var("TERM_PROGRAM").ok().filter(|t| !t.is_empty() && t != "tmux" && t != "Terminal") {
        return Some(pretty(&t));
    }
    walk(std::process::id(), |pid| {
        let stat = std::fs::read_to_string(format!("/proc/{}/stat", pid)).ok()?;
        parse_stat(&stat)
    })
}

/// (comm, parent pid) out of /proc/<pid>/stat, whose comm sits in parentheses and may hold spaces.
pub fn parse_stat(stat: &str) -> Option<(String, u32)> {
    let open = stat.find('(')?;
    let close = stat.rfind(')')?;
    let comm = stat[open + 1..close].to_string();
    let ppid = stat[close + 1..].split_whitespace().nth(1)?.parse().ok()?;
    Some((comm, ppid))
}

/// Walk up from `start`'s parent; `lookup` gives (comm, ppid) of a pid.
pub fn walk(start: u32, lookup: impl Fn(u32) -> Option<(String, u32)>) -> Option<String> {
    let (_, mut pid) = lookup(start)?;
    for _ in 0..32 {
        if pid <= 1 {
            return None;
        }
        let (comm, ppid) = lookup(pid)?;
        if STOP.contains(&comm.as_str()) {
            return None;
        }
        if !SKIP.contains(&comm.as_str()) && !comm.starts_with('-') {
            return Some(pretty(&comm));
        }
        pid = ppid;
    }
    None
}

fn pretty(name: &str) -> String {
    match name {
        "vscode" => "VS Code".into(),
        "iTerm.app" => "iTerm2".into(),
        "Apple_Terminal" => "Terminal".into(),
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table(pid: u32) -> Option<(String, u32)> {
        let t = [(100, "census", 90), (90, "zsh", 80), (80, "segan-terminal", 1), (70, "systemd", 1)];
        t.iter().find(|r| r.0 == pid).map(|r| (r.1.to_string(), r.2))
    }

    #[test]
    fn finds_the_terminal_above_the_shell() {
        assert_eq!(walk(100, table).as_deref(), Some("segan-terminal"));
    }

    #[test]
    fn stops_at_the_service_manager() {
        let t = |pid: u32| match pid {
            100 => Some(("census".to_string(), 90)),
            90 => Some(("zsh".to_string(), 70)),
            70 => Some(("systemd".to_string(), 1)),
            _ => None,
        };
        assert_eq!(walk(100, t), None);
    }

    #[test]
    fn stat_with_spaces_in_comm() {
        assert_eq!(parse_stat("5 (tmux: server) S 1 5 5 0"), Some(("tmux: server".to_string(), 1)));
    }
}
