//! The Eclipse, drawn from geometry at whatever size the layout asks for.
//!
//! One unit is one row; a column is `1 / aspect` units wide, so the picture is round on screen
//! when `aspect` is the terminal's real cell height over width. A lit sphere sits up-left of a dark
//! disc; the crescent is the sphere outside the disc, brightest where the sphere is deepest, with a
//! short glow bleeding into the disc. The disc is faint dots, a tilted dotted orbit with a small body
//! crosses the tail of the crescent, a few stars sit outside, and a faint label sits in the empty
//! lower-left corner. Every length is a ratio of the disc's radius, so the look is the same at 14 rows
//! and at 40.
//!
//! Tones: `b` bold accent (the core), `a` accent, `d` dim, `f` the terminal foreground (the orbit).

/// Bump when the drawing changes, so cached art is redrawn.
pub const ART_VERSION: u32 = 2;

const RAMP: &[u8] = b" .:-=+*#%@";
const LABEL: &str = "T+12:04:09";

/// The drawn art: `rows` lines of exactly `cols` cells, and a tone letter per cell.
pub struct Art {
    pub lines: Vec<String>,
    pub tones: Vec<String>,
    pub cols: usize,
}

/// The ratios that make the big and the small Eclipse; `r` is the disc radius in rows.
struct Shape {
    /// Rows per disc radius, after `bottom` rows are kept for the label under the orbit's foot.
    rows_per_r: f64,
    bottom: f64,
    /// Width of the canvas in radii.
    width_r: f64,
    sun_r: f64,
    /// The sun's centre: x in radii, y above the top of the frame in rows (negative is clipped).
    sun_x: f64,
    sun_top: f64,
    /// The disc centre from the sun centre, in radii.
    offset: (f64, f64),
    depth_w: f64,
    glow_k: f64,
    /// Orbit: centre (x, y), semi-axes (a, b) in radii, tilt and the body's position in degrees.
    orbit: (f64, f64, f64, f64, f64, f64),
    /// Hide the orbit's back half only where the disc covers it (true), or entirely (false).
    back: bool,
    /// Stars per 88 square radii of sky, and the scatter seed.
    stars: (f64, u64),
    label: bool,
}

const BIG: Shape = Shape {
    rows_per_r: 2.11,
    bottom: 1.4,
    width_r: 2.45,
    sun_r: 1.043,
    sun_x: 1.053,
    sun_top: 0.15,
    offset: (0.362, 0.255),
    depth_w: 0.298,
    glow_k: 0.128,
    orbit: (1.106, 1.447, 1.447, 0.34, -24.0, -12.0),
    back: true,
    stars: (6.0, 7),
    label: true,
};

const SMALL: Shape = Shape {
    rows_per_r: 2.3,
    bottom: 0.0,
    width_r: 2.45,
    sun_r: 1.048,
    sun_x: 1.17,
    sun_top: -0.1,
    offset: (0.357, 0.238),
    depth_w: 0.31,
    glow_k: 0.143,
    orbit: (1.24, 1.33, 1.52, 0.333, -24.0, 30.0),
    back: false,
    stars: (1.0, 3),
    label: false,
};

/// A tiny deterministic generator (xorshift64*), so the stars sit where they sat last time.
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1)
    }
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() >> 33) as usize % n.max(1)
    }
}

/// The columns the Eclipse takes at `rows` rows, before drawing it.
pub fn width_for(rows: usize, aspect: f64, small: bool) -> usize {
    let s = if small { &SMALL } else { &BIG };
    (s.width_r * s.radius(rows) * aspect).ceil() as usize
}

impl Shape {
    /// The disc radius in rows for a canvas `rows` tall.
    fn radius(&self, rows: usize) -> f64 {
        (rows.max(4) as f64 - self.bottom) / self.rows_per_r
    }
}

pub fn draw(rows: usize, aspect: f64, small: bool) -> Art {
    let s = if small { &SMALL } else { &BIG };
    let rows = rows.max(4);
    let aspect = if aspect.is_finite() && aspect > 0.5 { aspect } else { 2.0 };
    let cols = width_for(rows, aspect, small);
    let r = s.radius(rows);

    let r_sun = s.sun_r * r;
    let (sx, sy) = (s.sun_x * r, r_sun + s.sun_top);
    let (mx, my) = (sx + s.offset.0 * r, sy + s.offset.1 * r);
    let (depth_w, glow_k) = (s.depth_w * r, s.glow_k * r);

    let mut art = vec![vec![b' '; cols]; rows];
    let mut tone = vec![vec![b'd'; cols]; rows];
    let mut bright = vec![vec![0.0f64; cols]; rows];
    let unit = |c: usize, row: usize| ((c as f64 + 0.5) / aspect, row as f64 + 0.5);

    // The dot grid of the disc: squares on screen, whatever the cell shape.
    let gx = ((2.0 * aspect).round() as usize).max(3);
    let (gy, phase) = (2usize, 2usize);

    for row in 0..rows {
        for c in 0..cols {
            let (x, y) = unit(c, row);
            let dm = (x - mx).hypot(y - my);
            let ds = (x - sx).hypot(y - sy);
            let depth = (r_sun - ds).max(0.0);
            let mut b = (depth / depth_w).min(1.0).powf(0.6);
            if dm <= r {
                b *= (-(r - dm) / glow_k).exp();
            }
            bright[row][c] = b;
            if b >= 0.06 {
                art[row][c] = RAMP[((b * 9.4) as usize).min(9)];
                tone[row][c] = if b >= 0.74 {
                    b'b'
                } else if b >= 0.2 {
                    b'a'
                } else {
                    b'd'
                };
            } else if dm <= r && row % gy == 0 && (c + (row / gy % 2) * phase) % gx == 0 {
                art[row][c] = b'.';
            }
        }
    }

    // Orbit: an ellipse round the disc, the lower half in front, the upper half hidden behind the
    // disc and under the bright part of the crescent. Its left end runs off the frame.
    let (cx, cy, a, b_, theta, body_t) = s.orbit;
    let (cx, cy, a, b_) = (cx * r, cy * r, a * r, b_ * r);
    let (theta, body_t) = (theta.to_radians(), body_t.to_radians());
    let (st, ct) = theta.sin_cos();
    let point = |t: f64| {
        let (ex, ey) = (a * t.cos(), b_ * t.sin());
        (cx + ex * ct - ey * st, cy + ex * st + ey * ct)
    };
    let samples = (cols * 60).max(2400);
    for i in 0..samples {
        let t = std::f64::consts::TAU * i as f64 / samples as f64;
        let (x, y) = point(t);
        if x < 0.0 || y < 0.0 {
            continue;
        }
        let (c, row) = ((x * aspect) as usize, y as usize);
        if c >= cols || row >= rows {
            continue;
        }
        let (ux, uy) = unit(c, row);
        if t.sin() <= 0.0 && (!s.back || (ux - mx).hypot(uy - my) <= r) {
            continue;
        }
        if bright[row][c] >= 0.45 {
            continue;
        }
        art[row][c] = b'*'; // marks a middle dot, written as "·" below
        tone[row][c] = b'f';
    }
    let (bx, by) = point(body_t);
    if bx >= 0.0 && by >= 0.0 {
        let (c, row) = ((bx * aspect) as usize, by as usize);
        if c < cols && row < rows {
            art[row][c] = b'o';
            tone[row][c] = b'b';
        }
    }

    // Stars: a fixed scatter, only in empty sky above the orbit's foot and away from everything else.
    let sky_rows = ((my + r) as usize).saturating_sub(1).clamp(1, rows);
    let want = ((s.stars.0 * (r * r) / 88.0).round() as usize).clamp(1, 16);
    let mut rng = Rng::new(s.stars.1);
    let mut placed = 0;
    for _ in 0..want * 400 {
        if placed >= want {
            break;
        }
        let (c, row) = (rng.below(cols), rng.below(sky_rows));
        let (x, y) = unit(c, row);
        if (x - mx).hypot(y - my) <= r + 1.5 || (x - sx).hypot(y - sy) <= r_sun + 2.0 {
            continue;
        }
        let busy = (row.saturating_sub(1)..(row + 2).min(rows))
            .any(|rr| (c.saturating_sub(2)..(c + 3).min(cols)).any(|cc| art[rr][cc] != b' '));
        if busy {
            continue;
        }
        art[row][c] = if placed % 3 == 0 { b'+' } else { b'.' };
        placed += 1;
    }

    if s.label {
        let row = rows - 1;
        if cols > LABEL.len() + 2 && art[row][..LABEL.len() + 2].iter().all(|&ch| ch == b' ') {
            for (i, ch) in LABEL.bytes().enumerate() {
                art[row][i] = ch;
                tone[row][i] = b'd';
            }
        }
    }

    // The orbit is drawn as middle dots; the ramp's '*' lives inside the crescent, where the orbit
    // never is, so the tone letter tells them apart.
    let lines = art
        .iter()
        .zip(tone.iter())
        .map(|(row, t)| {
            row.iter()
                .zip(t.iter())
                .map(|(&ch, &tn)| if tn == b'f' && ch == b'*' { '\u{b7}' } else { ch as char })
                .collect()
        })
        .collect();
    let tones = tone.iter().map(|row| String::from_utf8_lossy(row).into_owned()).collect();
    Art { lines, tones, cols }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn count(art: &Art, tone: u8) -> usize {
        art.tones.iter().map(|t| t.bytes().filter(|&b| b == tone).count()).sum()
    }

    #[test]
    fn sizes_follow_rows_and_aspect() {
        let a = draw(21, 2.0, false);
        assert_eq!(a.lines.len(), 21);
        assert_eq!(a.cols, 46, "the 21-row art at 2:1 cells is the old 46-column asset");
        assert!(a.lines.iter().all(|l| l.chars().count() == a.cols));
        assert!(a.tones.iter().all(|t| t.len() == a.cols));
        let wide = draw(21, 2.8, false);
        assert_eq!(wide.cols, 64, "taller cells need more columns for the same round disc");
        let tall = draw(34, 2.0, false);
        assert_eq!(tall.cols, 76);
        assert_eq!(width_for(34, 2.0, false), tall.cols);
    }

    #[test]
    fn every_size_has_the_whole_picture() {
        for rows in [14usize, 18, 24, 34, 40] {
            for aspect in [2.0, 2.2, 2.8] {
                let a = draw(rows, aspect, false);
                let bold = count(&a, b'b');
                assert!(bold > rows, "{rows} rows at {aspect}: {bold} core cells");
                assert!(count(&a, b'f') > rows, "{rows} rows at {aspect}: no orbit");
                assert!(a.lines.iter().any(|l| l.contains('o')), "{rows} rows at {aspect}: no body");
                assert!(a.lines[0].trim().len() > 1, "{rows} rows at {aspect}: the top row is empty");
                assert!(a.lines[rows - 1].starts_with(LABEL), "{rows} rows at {aspect}: no label");
                assert!(a.lines.iter().any(|l| l.contains('+')), "{rows} rows at {aspect}: no stars");
            }
        }
    }

    #[test]
    fn small_art_is_the_compact_logo() {
        let a = draw(10, 2.0, true);
        assert_eq!(a.cols, 22);
        assert!(count(&a, b'b') > 5);
        assert!(count(&a, b'f') > 5);
        assert!(!a.lines.iter().any(|l| l.contains("T+")));
    }

    #[test]
    fn stars_are_the_same_every_time() {
        assert_eq!(draw(30, 2.2, false).lines, draw(30, 2.2, false).lines);
    }
}
