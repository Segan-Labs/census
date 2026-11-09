pub mod canvas;

pub use canvas::*;

pub trait Widget {
    fn measure(&self) -> Size;
    fn draw(&self, canvas: &mut Canvas, rect: &Rect);
}

pub struct KeyVal<'a> {
    pub icon: &'a str,
    pub label: &'a str,
    pub value: String,
}

impl<'a> Widget for KeyVal<'a> {
    fn measure(&self) -> Size {
        // "icon label → value"
        let vis_len = self.icon.len() + 1 + self.label.len() + 3 + self.value.len();
        Size { width: vis_len, height: 1 }
    }

    fn draw(&self, canvas: &mut Canvas, rect: &Rect) {
        let s = format!("{} {} \u{2192} {}", self.icon, self.label, self.value);
        canvas.draw_str_colored(rect.x, rect.y, &s, "");
    }
}

pub struct ProgressBar {
    pub used: f64,
    pub total: f64,
    pub width: usize,
}

impl Widget for ProgressBar {
    fn measure(&self) -> Size {
        Size { width: self.width + 4, height: 1 }
    }

    fn draw(&self, canvas: &mut Canvas, rect: &Rect) {
        if self.total <= 0.0 {
            return;
        }
        let ratio = (self.used / self.total).clamp(0.0, 1.0);
        let filled = (ratio * self.width as f64).round() as usize;
        let color = if ratio < 0.5 { "\x1b[32m" } else if ratio < 0.75 { "\x1b[33m" } else { "\x1b[31m" };

        for i in 0..self.width {
            let ch = if i < filled { '\u{2588}' } else { '\u{2591}' };
            if i < filled {
                canvas.draw_str_colored(rect.x + i, rect.y, &ch.to_string(), color);
            } else {
                canvas.set(rect.x + i, rect.y, ch);
            }
        }
    }
}

pub struct TempDisplay {
    pub temp: f64,
}

impl Widget for TempDisplay {
    fn measure(&self) -> Size {
        Size { width: 8, height: 1 }
    }

    fn draw(&self, canvas: &mut Canvas, rect: &Rect) {
        let color = if self.temp < 50.0 { "\x1b[32m" } else if self.temp < 70.0 { "\x1b[33m" } else { "\x1b[31m" };
        let s = format!("{:.0}\u{00b0}C", self.temp);
        canvas.draw_str_colored(rect.x, rect.y, &s, color);
    }
}

pub struct Card {
    pub title: String,
    pub rows: Vec<String>,
    pub width: usize,
}

impl Card {
    pub fn new(title: &str, rows: Vec<String>, width: usize) -> Self {
        Self { title: title.to_string(), rows, width }
    }
}

impl Widget for Card {
    fn measure(&self) -> Size {
        let content_height = self.rows.len().max(1);
        Size {
            width: self.width,
            height: content_height + 2, // top + bottom borders
        }
    }

    fn draw(&self, canvas: &mut Canvas, rect: &Rect) {
        let border = Border::rounded();
        let inner_w = rect.width.saturating_sub(2);
        let inner_h = rect.height.saturating_sub(2);

        // Draw border
        canvas.draw_rect(rect, &border);

        // Title: ┌── Title ─────┐
        let title_len = self.title.len();
        let fill_len = inner_w.saturating_sub(title_len + 4);
        let title_str = format!("── {} {}", self.title, "\u{2500}".repeat(fill_len));
        canvas.draw_str(rect.x + 1, rect.y, &title_str);

        // Content rows
        for (i, row) in self.rows.iter().enumerate() {
            if i >= inner_h {
                break;
            }
            let y = rect.y + 1 + i;
            // Truncate if needed
            let display: String = row.chars().take(inner_w).collect();
            canvas.draw_str_colored(rect.x + 1, y, &display, "");
        }
    }
}
