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

/// What the terminal says about itself.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Geometry {
    pub cols: usize,
    pub rows: usize,
    /// A cell's height over its width, when the terminal reports its size in pixels too.
    pub cell_aspect: Option<f64>,
}

/// The terminal's size when stdout is a terminal.
pub fn terminal_geometry() -> Option<Geometry> {
    use std::os::unix::io::AsRawFd;
    let fd = std::io::stdout().as_raw_fd();
    let mut ws: Winsize = unsafe { std::mem::zeroed() };
    if unsafe { ioctl(fd, TIOCGWINSZ, &mut ws) } == 0 && ws.ws_col > 0 {
        Some(geometry(ws.ws_col, ws.ws_row, ws.ws_xpixel, ws.ws_ypixel))
    } else {
        None
    }
}

fn geometry(cols: u16, rows: u16, xpixel: u16, ypixel: u16) -> Geometry {
    let cell_aspect = (rows > 0 && xpixel > 0 && ypixel > 0)
        .then(|| (ypixel as f64 / rows as f64) / (xpixel as f64 / cols as f64))
        .filter(|a| (1.0..=4.0).contains(a));
    Geometry { cols: cols as usize, rows: rows as usize, cell_aspect }
}

/// The terminal's size as (columns, rows) when stdout is a terminal.
pub fn terminal_size() -> Option<(usize, usize)> {
    terminal_geometry().map(|g| (g.cols, g.rows))
}

pub fn detect_terminal_width() -> usize {
    terminal_size().map(|(w, _)| w).unwrap_or(80)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cell_aspect_from_pixels() {
        let g = geometry(253, 46, 2530, 1288);
        assert!((g.cell_aspect.unwrap() - 2.8).abs() < 1e-9, "{:?}", g);
        assert_eq!(geometry(80, 24, 0, 0).cell_aspect, None, "no pixel size, no aspect");
        assert_eq!(geometry(80, 24, 800, 24).cell_aspect, None, "a nonsense aspect is ignored");
    }
}
