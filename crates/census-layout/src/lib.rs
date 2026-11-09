use census_core::LayoutConfig;

/// Column maths of the card dashboard (`--layout dashboard`).
pub struct LayoutEngine {
    pub config: LayoutConfig,
    terminal_width: usize,
}

impl LayoutEngine {
    pub fn new(config: LayoutConfig, terminal_width: usize) -> Self {
        Self { config, terminal_width }
    }

    pub fn calculate_columns(&self) -> Vec<usize> {
        // Each row has up to 2 sections (left, right)
        let num_cols = 2;
        let usable_width = self.terminal_width.saturating_sub(2);
        let gap = 4;
        let card_width = usable_width.saturating_sub(gap) / num_cols;
        vec![card_width; num_cols]
    }
}

#[repr(C)]
struct Winsize {
    ws_row: u16,
    ws_col: u16,
    ws_xpixel: u16,
    ws_ypixel: u16,
}

extern "C" {
    fn ioctl(fd: i32, request: u64, ...) -> i32;
}

const TIOCGWINSZ: u64 = 0x5413;

/// The terminal's size as (columns, rows) when stdout is a terminal.
pub fn terminal_size() -> Option<(usize, usize)> {
    use std::os::unix::io::AsRawFd;
    let fd = std::io::stdout().as_raw_fd();
    let mut ws: Winsize = unsafe { std::mem::zeroed() };
    if unsafe { ioctl(fd, TIOCGWINSZ, &mut ws) } == 0 && ws.ws_col > 0 {
        Some((ws.ws_col as usize, ws.ws_row as usize))
    } else {
        None
    }
}

pub fn detect_terminal_width() -> usize {
    terminal_size().map(|(w, _)| w).unwrap_or(80)
}
