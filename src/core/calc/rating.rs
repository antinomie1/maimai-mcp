/// Computes single song rating based on internal constant (ds) and achievement percentage.
pub fn compute_single_rating(ds: f64, achievements: f64) -> u32 {
    let (factor, base_rate) = match achievements {
        ach if ach >= 100.5000 => (22.4, "sssp"),
        ach if ach >= 100.0000 => (21.6, "sss"),
        ach if ach >= 99.5000 => (21.1, "ssp"),
        ach if ach >= 99.0000 => (20.8, "ss"),
        ach if ach >= 98.0000 => (20.3, "sp"),
        ach if ach >= 97.0000 => (20.0, "s"),
        ach if ach >= 94.0000 => (16.8, "aaa"),
        ach if ach >= 90.0000 => (15.2, "aa"),
        ach if ach >= 80.0000 => (13.6, "a"),
        ach if ach >= 75.0000 => (12.0, "bbb"),
        ach if ach >= 70.0000 => (11.2, "bb"),
        ach if ach >= 60.0000 => (9.6, "b"),
        ach if ach >= 50.0000 => (8.0, "c"),
        _ => (0.0, "d"),
    };
    let _ = base_rate;

    let clamped_ach = achievements.min(100.5);
    (ds * clamped_ach * factor / 100.0).floor() as u32
}

/// Returns rate grade string (sssp, sss, etc.)
pub fn get_rate_string(achievements: f64) -> &'static str {
    match achievements {
        ach if ach >= 100.5000 => "sssp",
        ach if ach >= 100.0000 => "sss",
        ach if ach >= 99.5000 => "ssp",
        ach if ach >= 99.0000 => "ss",
        ach if ach >= 98.0000 => "sp",
        ach if ach >= 97.0000 => "s",
        ach if ach >= 94.0000 => "aaa",
        ach if ach >= 90.0000 => "aa",
        ach if ach >= 80.0000 => "a",
        ach if ach >= 75.0000 => "bbb",
        ach if ach >= 70.0000 => "bb",
        ach if ach >= 60.0000 => "b",
        ach if ach >= 50.0000 => "c",
        _ => "d",
    }
}
