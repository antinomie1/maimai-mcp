//! Small helpers shared across modules: Python-compatible formatting,
//! column-width truncation, UTC+8 calendar and a tiny PRNG.

use std::time::{SystemTime, UNIX_EPOCH};

/// Formats a float like Python's `str(float)` (`13.0`, `13.7`, `180.0`).
pub fn py_float(v: f64) -> String {
    if v.is_finite() && v.fract() == 0.0 && v.abs() < 1e16 { format!("{v:.1}") } else { format!("{v}") }
}

/// Python `round(v, n)` (correctly rounded on the binary value).
pub fn round_n(v: f64, n: usize) -> f64 {
    format!("{v:.n$}").parse().unwrap_or(v)
}

/// Python `f"{round(v, 2)}"`.
pub fn py_round(v: f64, n: usize) -> String {
    py_float(round_n(v, n))
}

const WIDTHS: [(u32, u32); 38] = [
    (126, 1),
    (159, 0),
    (687, 1),
    (710, 0),
    (711, 1),
    (727, 0),
    (733, 1),
    (879, 0),
    (1154, 1),
    (1161, 0),
    (4347, 1),
    (4447, 2),
    (7467, 1),
    (7521, 0),
    (8369, 1),
    (8426, 0),
    (9000, 1),
    (9002, 2),
    (11021, 1),
    (12350, 2),
    (12351, 1),
    (12438, 2),
    (12442, 0),
    (19893, 2),
    (19967, 1),
    (55203, 2),
    (63743, 1),
    (64106, 2),
    (65039, 1),
    (65059, 0),
    (65131, 2),
    (65279, 1),
    (65376, 2),
    (65500, 1),
    (65510, 2),
    (120831, 1),
    (262141, 2),
    (1114109, 1),
];

fn char_width(c: char) -> u32 {
    let o = c as u32;
    if o == 0xE || o == 0xF {
        return 0;
    }
    WIDTHS.iter().find(|(n, _)| o <= *n).map_or(1, |(_, w)| *w)
}

pub fn column_width(s: &str) -> u32 {
    s.chars().map(char_width).sum()
}

/// Keeps characters while the accumulated column width stays `<= len`.
pub fn change_column_width(s: &str, len: u32) -> String {
    let mut total = 0;
    s.chars()
        .filter(|c| {
            total += char_width(*c);
            total <= len
        })
        .collect()
}

/// `title` truncated with `...` when wider than `max` columns (upstream pattern).
pub fn ellipsis(s: &str, max: u32) -> String {
    if column_width(s) > max { format!("{}...", change_column_width(s, max - 1)) } else { s.to_string() }
}

pub fn unix_now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs()
}

/// Seconds since epoch shifted to UTC+8 (maimai CN server time).
pub fn cn_now() -> u64 {
    unix_now() + 8 * 3600
}

/// (year, month, day) of a day count since 1970-01-01.
pub fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (yoe + era * 400 + i64::from(m <= 2), m, d)
}

/// Current CN date (year, month, day).
pub fn cn_date() -> (i64, u32, u32) {
    civil_from_days((cn_now() / 86_400) as i64)
}

/// Current CN date-time `YYYY-MM-DD HH:MM:SS`.
pub fn cn_datetime() -> String {
    let now = cn_now();
    let (y, m, d) = civil_from_days((now / 86_400) as i64);
    let s = now % 86_400;
    format!("{y:04}-{m:02}-{d:02} {:02}:{:02}:{:02}", s / 3600, s / 60 % 60, s % 60)
}

/// Upstream `qqhash`: daily deterministic hash of a QQ number.
pub fn qq_hash(qq: u64) -> u64 {
    let (_, m, d) = cn_date();
    let days = u64::from(d) + 31 * u64::from(m) + 77;
    days.wrapping_mul(qq) >> 8
}

/// xorshift64* PRNG; good enough for picking songs.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1)
    }

    pub fn from_time() -> Self {
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos();
        Self::new(nanos as u64)
    }

    pub fn next_u64(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 { 0 } else { (self.next_u64() % n as u64) as usize }
    }

    pub fn choice<'a, T>(&mut self, items: &'a [T]) -> Option<&'a T> {
        if items.is_empty() { None } else { items.get(self.below(items.len())) }
    }

    /// Random sample of up to `k` items without replacement.
    pub fn sample<T: Clone>(&mut self, items: &[T], k: usize) -> Vec<T> {
        let mut pool: Vec<T> = items.to_vec();
        let k = k.min(pool.len());
        for i in 0..k {
            let j = i + self.below(pool.len() - i);
            pool.swap(i, j);
        }
        pool.truncate(k);
        pool
    }
}
