//! Never-worse output guard: RTK never emits more tokens than the raw command.
//!
//! One caller is allowed past it. `rtk diff` renders a short message for a
//! difference `str::lines()` cannot show, where the raw fallback is two blobs
//! that look identical and answers the question worse at any size.
//! `INVISIBLE_DIFF_TOKEN_ALLOWANCE` bounds what the message itself may cost.
//! The `file1 -> file2` header it opens with is charged to the caller's own
//! arguments rather than to the allowance, so the printed overage above raw is
//! the allowance plus the length of the two paths.

use crate::core::tracking::estimate_tokens;

/// Tokens `rtk diff` may spend above raw to name a difference that a line-based
/// diff cannot render. Past it the raw text is short enough to read directly.
///
/// Measured on what the message states, not on the `file1 -> file2` header the
/// caller's own arguments determine: an absolute path pair would otherwise eat
/// the whole allowance and drop the diagnostic for reasons unrelated to it.
pub const INVISIBLE_DIFF_TOKEN_ALLOWANCE: usize = 16;

/// Returns `filtered`, or `raw` when `filtered` would emit more tokens.
pub fn never_worse<'a>(raw: &'a str, filtered: &'a str) -> &'a str {
    if estimate_tokens(filtered) > estimate_tokens(raw) {
        raw
    } else {
        filtered
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_filtered_when_smaller() {
        let raw = "a".repeat(400);
        assert_eq!(never_worse(&raw, "ok"), "ok");
    }

    #[test]
    fn falls_back_to_raw_when_filtered_bigger() {
        let raw = "{}";
        let filtered = "{\n  \"pretty\": true\n}";
        assert_eq!(never_worse(raw, filtered), raw);
    }

    #[test]
    fn tie_keeps_filtered() {
        assert_eq!(never_worse("abcd", "wxyz"), "wxyz");
    }

    #[test]
    fn token_boundary_follows_estimate_tokens() {
        assert_eq!(never_worse("abcd", "abcde"), "abcd");
        assert_eq!(never_worse("abcdefgh", "ijklmnop"), "ijklmnop");
    }

    #[test]
    fn empty_raw_returns_raw() {
        assert_eq!(never_worse("", "0 matches"), "");
    }

    #[test]
    fn empty_filtered_returns_filtered() {
        assert_eq!(never_worse("data", ""), "");
    }

    #[test]
    fn both_empty_returns_filtered() {
        assert_eq!(never_worse("", ""), "");
    }
}
