//! Passphrase normalization.
//!
//! The same human-visible passphrase must derive the same key across platforms.

use unicode_normalization::UnicodeNormalization;

/// Normalize a passphrase before it is used by KDF.
///
/// Rules:
/// - Unicode NFKC normalization.
/// - Trim leading/trailing whitespace.
/// - Convert all Unicode whitespace runs into a single ASCII space.
/// - Do not lowercase or otherwise modify non-whitespace content.
pub fn normalize_passphrase(input: &str) -> String {
    let nfkc: String = input.nfkc().collect();
    let mut out = String::new();
    let mut in_ws = false;

    for ch in nfkc.trim().chars() {
        if ch.is_whitespace() {
            if !in_ws && !out.is_empty() {
                out.push(' ');
            }
            in_ws = true;
        } else {
            out.push(ch);
            in_ws = false;
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_collapses_spaces_and_nfkc() {
        assert_eq!(normalize_passphrase("  松鼠\u{3000}\t河流  "), "松鼠 河流");
        assert_eq!(normalize_passphrase("ＡＢＣ  １２３"), "ABC 123");
    }
}
