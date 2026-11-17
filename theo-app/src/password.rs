//! Rules for a congregation password that the admin chooses (decided 2026-11-17, R-314).
//!
//! The generated passphrase stays the default and is much stronger. The strength meter is shown
//! as advice; these rules are the minimum.

/// Minimum length of a chosen congregation password.
pub const MIN_LEN: usize = 6;

/// Each rule and whether `pw` meets it, in display order.
pub fn rules(pw: &str) -> Vec<(String, bool)> {
    vec![
        (format!("At least {MIN_LEN} characters"), pw.chars().count() >= MIN_LEN),
        ("An uppercase letter".into(), pw.chars().any(char::is_uppercase)),
        ("A lowercase letter".into(), pw.chars().any(char::is_lowercase)),
        ("A number".into(), pw.chars().any(|c| c.is_ascii_digit())),
        ("A symbol, for example ! ? # or -".into(), pw.chars().any(|c| !c.is_alphanumeric() && !c.is_whitespace())),
    ]
}

/// True when every rule is met.
pub fn meets_rules(pw: &str) -> bool {
    rules(pw).iter().all(|(_, ok)| *ok)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_rule_is_needed() {
        assert!(meets_rules("Abc12!"));
        for weak in ["Abc1!", "abc123!", "ABC123!", "Abcdef!", "Abc1234", "Abc 12 3"] {
            assert!(!meets_rules(weak), "{weak} should fail");
        }
    }
}
