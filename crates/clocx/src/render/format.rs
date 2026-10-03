//! Text formatting of numbers, deltas, ages and sparklines, shared by every view.

use jiff::{SignedDuration, Timestamp};

/// Sparkline glyphs, lowest to highest; zero always maps to the first.
const SPARK: [char; 8] = [
    '\u{2581}', '\u{2582}', '\u{2583}', '\u{2584}', '\u{2585}', '\u{2586}', '\u{2587}', '\u{2588}',
];

/// Groups digits in threes: 1234567 -> "1,234,567".
fn grouped(n: u64) -> String {
    let digits = n.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (i, ch) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(ch);
    }
    out
}

/// Formats a count for humans: exact up to 99,999, then 123.4k, 12.3M.
pub fn count(n: u64) -> String {
    match n {
        0..100_000 => grouped(n),
        100_000..1_000_000 => format!("{:.1}k", n as f64 / 1_000.0),
        _ => format!("{:.1}M", n as f64 / 1_000_000.0),
    }
}

/// Formats a signed change with an explicit sign; zero is empty so quiet rows stay quiet.
pub fn delta(n: i64) -> String {
    match n {
        0 => String::new(),
        n if n > 0 => format!("+{}", count(n.unsigned_abs())),
        n => format!("-{}", count(n.unsigned_abs())),
    }
}

/// Formats lines added as "+N" (empty for zero).
pub fn added(n: u64) -> String {
    if n == 0 {
        String::new()
    } else {
        format!("+{}", count(n))
    }
}

/// Formats lines removed as "-N" (empty for zero).
pub fn removed(n: u64) -> String {
    if n == 0 {
        String::new()
    } else {
        format!("-{}", count(n))
    }
}

/// Formats how long ago `then` was relative to `now`: "just now", "5m ago", "3h ago", "2d ago".
pub fn age(then: Timestamp, now: Timestamp) -> String {
    let secs = now.duration_since(then).max(SignedDuration::ZERO).as_secs();
    match secs {
        0..60 => "just now".to_owned(),
        60..3_600 => format!("{}m ago", secs / 60),
        3_600..86_400 => format!("{}h ago", secs / 3_600),
        86_400..2_592_000 => format!("{}d ago", secs / 86_400),
        _ => format!("{}mo ago", secs / 2_592_000),
    }
}

/// Draws values as a block sparkline scaled to the maximum; zero is the lowest bar, any non-zero value is visibly higher.
pub fn sparkline(values: &[u64]) -> String {
    let max = values.iter().copied().max().unwrap_or(0);
    values
        .iter()
        .map(|&v| {
            if v == 0 || max == 0 {
                SPARK[0]
            } else {
                // Non-zero values use levels 1..=7 so they never look like zero.
                let level = 1 + (v.saturating_mul(6) / max) as usize;
                SPARK[level.min(SPARK.len() - 1)]
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_are_grouped_then_abbreviated() {
        assert_eq!(count(0), "0");
        assert_eq!(count(999), "999");
        assert_eq!(count(1_234), "1,234");
        assert_eq!(count(99_999), "99,999");
        assert_eq!(count(123_456), "123.5k");
        assert_eq!(count(12_345_678), "12.3M");
    }

    #[test]
    fn deltas_carry_a_sign_and_zero_is_blank() {
        assert_eq!(delta(0), "");
        assert_eq!(delta(12), "+12");
        assert_eq!(delta(-1_500), "-1,500");
        assert_eq!(added(0), "");
        assert_eq!(added(3), "+3");
        assert_eq!(removed(4), "-4");
    }

    #[test]
    fn ages_use_the_largest_unit() {
        let now: Timestamp = "2026-10-03T12:00:00Z".parse().unwrap();
        let ago = |s: i64| age(now - SignedDuration::from_secs(s), now);
        assert_eq!(ago(5), "just now");
        assert_eq!(ago(300), "5m ago");
        assert_eq!(ago(3 * 3_600), "3h ago");
        assert_eq!(ago(2 * 86_400), "2d ago");
        assert_eq!(ago(-50), "just now");
    }

    #[test]
    fn sparkline_scales_and_keeps_nonzero_visible() {
        assert_eq!(sparkline(&[]), "");
        assert_eq!(sparkline(&[0, 0]), "\u{2581}\u{2581}");
        let s: Vec<char> = sparkline(&[0, 1, 1000]).chars().collect();
        assert_eq!(s[0], SPARK[0]);
        assert_eq!(s[1], SPARK[1]);
        assert_eq!(s[2], SPARK[7]);
    }
}
