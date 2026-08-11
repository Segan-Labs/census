pub struct Ansi;

impl Ansi {
    pub const RESET: &'static str = "\x1b[0m";
    pub const BOLD: &'static str = "\x1b[1m";

    pub fn color(name: &str) -> &'static str {
        match name {
            "black" => "\x1b[30m",
            "red" => "\x1b[31m",
            "green" => "\x1b[32m",
            "yellow" => "\x1b[33m",
            "blue" => "\x1b[34m",
            "magenta" => "\x1b[35m",
            "cyan" => "\x1b[36m",
            "white" => "\x1b[37m",
            _ => "\x1b[0m",
        }
    }

    pub fn color_rgb(r: u8, g: u8, b: u8) -> String {
        format!("\x1b[38;2;{};{};{}m", r, g, b)
    }

    pub fn bg_color(name: &str) -> &'static str {
        match name {
            "black" => "\x1b[40m",
            "red" => "\x1b[41m",
            "green" => "\x1b[42m",
            "yellow" => "\x1b[43m",
            "blue" => "\x1b[44m",
            "magenta" => "\x1b[45m",
            "cyan" => "\x1b[46m",
            "white" => "\x1b[47m",
            _ => "\x1b[0m",
        }
    }
}
