use std::fs;
use std::path::Path;
use std::str::FromStr;

pub fn read_string<P: AsRef<Path>>(path: P) -> String {
    fs::read_to_string(path)
        .unwrap_or_default()
        .trim()
        .to_owned()
}

pub fn read_parse<T, P>(path: P) -> Option<T>
where
    T: FromStr,
    P: AsRef<Path>,
{
    fs::read_to_string(path)
        .ok()
        .and_then(|v| v.trim().parse::<T>().ok())
}

pub fn read_parse_or<T, P>(path: P, default: T) -> T
where
    T: FromStr,
    P: AsRef<Path>,
{
    read_parse(path).unwrap_or(default)
}

pub fn exists<P: AsRef<Path>>(path: P) -> bool {
    path.as_ref().exists()
}

pub fn read_dir_count<P: AsRef<Path>>(path: P) -> usize {
    fs::read_dir(path)
        .map(|r| r.filter_map(|e| e.ok()).count())
        .unwrap_or(0)
}

pub fn read_dir_dirs<P: AsRef<Path>>(path: P) -> usize {
    fs::read_dir(path)
        .map(|r| r.filter_map(|e| e.ok()).filter(|e| e.path().is_dir()).count())
        .unwrap_or(0)
}

pub trait ReadExt {
    fn read_string(&self) -> String;
    fn read_u64(&self) -> u64;
    fn read_f64(&self) -> f64;
}

impl ReadExt for str {
    fn read_string(&self) -> String {
        read_string(self)
    }

    fn read_u64(&self) -> u64 {
        read_parse_or(self, 0u64)
    }

    fn read_f64(&self) -> f64 {
        read_parse_or(self, 0.0)
    }
}

impl ReadExt for String {
    fn read_string(&self) -> String {
        read_string(self.as_str())
    }

    fn read_u64(&self) -> u64 {
        read_parse_or(self.as_str(), 0u64)
    }

    fn read_f64(&self) -> f64 {
        read_parse_or(self.as_str(), 0.0)
    }
}
