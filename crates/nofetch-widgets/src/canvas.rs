#[derive(Debug, Clone, Copy)]
pub struct Size {
    pub width: usize,
    pub height: usize,
}

#[derive(Debug, Clone)]
pub struct Rect {
    pub x: usize,
    pub y: usize,
    pub width: usize,
    pub height: usize,
}

#[derive(Debug, Clone)]
pub struct Cell {
    pub ch: char,
    pub fg: Option<String>,
    pub bg: Option<String>,
}

impl Cell {
    pub fn new(ch: char) -> Self {
        Self { ch, fg: None, bg: None }
    }

    pub fn with_fg(mut self, fg: &str) -> Self {
        self.fg = Some(fg.to_string());
        self
    }
}

pub struct Canvas {
    pub width: usize,
    pub height: usize,
    cells: Vec<Vec<Cell>>,
}

impl Canvas {
    pub fn new(width: usize, height: usize) -> Self {
        let cells = vec![vec![Cell::new(' '); width]; height];
        Self { width, height, cells }
    }

    pub fn clear(&mut self) {
        for row in &mut self.cells {
            for cell in row.iter_mut() {
                *cell = Cell::new(' ');
            }
        }
    }

    pub fn set(&mut self, x: usize, y: usize, ch: char) {
        if x < self.width && y < self.height {
            self.cells[y][x] = Cell::new(ch);
        }
    }

    pub fn set_cell(&mut self, x: usize, y: usize, cell: Cell) {
        if x < self.width && y < self.height {
            self.cells[y][x] = cell;
        }
    }

    pub fn draw_str(&mut self, x: usize, y: usize, s: &str) {
        for (i, ch) in s.chars().enumerate() {
            self.set(x + i, y, ch);
        }
    }

    pub fn draw_str_colored(&mut self, x: usize, y: usize, s: &str, fg: &str) {
        let mut col = x;
        let mut in_escape = false;
        let mut escape_buf = String::new();

        for ch in s.chars() {
            if ch == '\x1b' {
                in_escape = true;
                escape_buf.clear();
                continue;
            }
            if in_escape {
                escape_buf.push(ch);
                if ch == 'm' {
                    in_escape = false;
                }
                continue;
            }
            if col < self.width && y < self.height {
                self.cells[y][col] = Cell::new(ch).with_fg(fg);
            }
            col += 1;
        }
    }

    pub fn draw_rect(&mut self, rect: &Rect, border: &Border) {
        let x = rect.x;
        let y = rect.y;
        let w = rect.width;
        let h = rect.height;

        if w < 2 || h < 2 {
            return;
        }

        // Top
        self.set(x, y, border.top_left);
        for i in 1..w.saturating_sub(1) {
            self.set(x + i, y, border.horizontal);
        }
        self.set(x + w - 1, y, border.top_right);

        // Sides
        for row in 1..h.saturating_sub(1) {
            self.set(x, y + row, border.vertical);
            self.set(x + w - 1, y + row, border.vertical);
        }

        // Bottom
        self.set(x, y + h - 1, border.bottom_left);
        for i in 1..w.saturating_sub(1) {
            self.set(x + i, y + h - 1, border.horizontal);
        }
        self.set(x + w - 1, y + h - 1, border.bottom_right);
    }

    pub fn render(&self) {
        for row in &self.cells {
            let line: String = row.iter().map(|c| c.ch).collect();
            println!("{}", line);
        }
    }

    pub fn trim_trailing_empty(&mut self) {
        // Trim trailing empty rows
        let mut last_non_empty = 0;
        for (i, row) in self.cells.iter().enumerate() {
            if row.iter().any(|c| c.ch != ' ') {
                last_non_empty = i;
            }
        }
        self.cells.truncate(last_non_empty + 1);
        self.height = last_non_empty + 1;

        // Trim trailing empty columns
        let mut max_col = 0;
        for row in &self.cells {
            for (i, cell) in row.iter().enumerate() {
                if cell.ch != ' ' && i + 1 > max_col {
                    max_col = i + 1;
                }
            }
        }
        if max_col > 0 && max_col < self.width {
            for row in &mut self.cells {
                row.truncate(max_col);
            }
            self.width = max_col;
        }
    }
}

pub struct Border {
    pub top_left: char,
    pub top_right: char,
    pub bottom_left: char,
    pub bottom_right: char,
    pub horizontal: char,
    pub vertical: char,
}

impl Border {
    pub fn rounded() -> Self {
        Self {
            top_left: '\u{256d}',
            top_right: '\u{256e}',
            bottom_left: '\u{2570}',
            bottom_right: '\u{256f}',
            horizontal: '\u{2500}',
            vertical: '\u{2502}',
        }
    }
}
