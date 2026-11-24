//! Rating math ported from upstream `utils/calc.py`.

const RATE_TABLE: [(f64, f64, &str); 14] = [
    (50.0, 7.0, "D"),
    (60.0, 8.0, "C"),
    (70.0, 9.6, "B"),
    (75.0, 11.2, "BB"),
    (80.0, 12.0, "BBB"),
    (90.0, 13.6, "A"),
    (94.0, 15.2, "AA"),
    (97.0, 16.8, "AAA"),
    (98.0, 20.0, "S"),
    (99.0, 20.3, "Sp"),
    (99.5, 20.8, "SS"),
    (100.0, 21.1, "SSp"),
    (100.5, 21.6, "SSS"),
    (f64::INFINITY, 22.4, "SSSp"),
];

fn base(achievements: f64) -> (f64, &'static str) {
    RATE_TABLE.iter().find(|(limit, _, _)| achievements < *limit).map_or((22.4, "SSSp"), |(_, b, r)| (*b, *r))
}

/// Single chart rating: `floor(ds * min(100.5, ach) / 100 * base)`.
pub fn compute_rating(ds: f64, achievements: f64) -> u32 {
    let (b, _) = base(achievements);
    (ds * (achievements.min(100.5) / 100.0) * b).floor() as u32
}

/// Rank image name (`SSSp`, `AA`, ...).
pub fn compute_rate(achievements: f64) -> &'static str {
    base(achievements).1
}

/// DX score star count from a percentage.
pub fn dx_star(percent: f64) -> u32 {
    match percent {
        p if p < 85.0 => 0,
        p if p < 90.0 => 1,
        p if p < 93.0 => 2,
        p if p < 95.0 => 3,
        p if p < 97.0 => 4,
        _ => 5,
    }
}
