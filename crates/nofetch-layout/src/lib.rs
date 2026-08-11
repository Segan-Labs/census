use nofetch_core::LayoutConfig;

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
        let card_width = (usable_width - gap) / num_cols;
        vec![card_width; num_cols]
    }
}

pub fn detect_terminal_width() -> usize {
    #[cfg(unix)]
    {
        use std::os::unix::io::AsRawFd;
        use std::mem;

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

        let fd = std::io::stdout().as_raw_fd();
        let mut ws: Winsize = unsafe { mem::zeroed() };
        if unsafe { ioctl(fd, TIOCGWINSZ, &mut ws) } == 0 && ws.ws_col > 0 {
            return ws.ws_col as usize;
        }
    }
    80
}
