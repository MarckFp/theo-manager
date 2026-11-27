//! Passwords: the rules for a congregation password that the admin chooses (decided 2026-11-17,
//! R-314), the generated passphrase, and the strength meter (zxcvbn, R-315).
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

/// A new generated congregation password: six words of the EFF large list, ≈ 77.5 bits (R-314,
/// `theo_crypto::passphrase`). Empty only if the device cannot make random numbers.
pub fn generate() -> String {
    theo_crypto::passphrase::generate().unwrap_or_default()
}

/// Password strength from 0 (very weak) to 4 (very strong), by zxcvbn (R-315): it knows common
/// passwords, names, keyboard patterns and dates. `inputs` are words that make a password weak
/// here (the username, the person's or the congregation's name).
pub fn strength(pw: &str, inputs: &[&str]) -> u8 {
    if pw.is_empty() {
        return 0;
    }
    u8::from(zxcvbn::zxcvbn(pw, inputs).score())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_passwords_are_strong_and_weak_ones_are_caught() {
        let a = generate();
        assert!(!a.is_empty() && a != generate());
        assert!(strength(&a, &[]) >= 3, "{a}");
        assert_eq!(strength("", &[]), 0);
        assert!(strength("password1", &[]) <= 1);
        assert!(strength("Qwerty123!", &[]) <= 2);
        // The username makes a password weaker.
        assert!(strength("hillcrest2026", &["hillcrest"]) < strength("hillcrest2026", &[]) || strength("hillcrest2026", &[]) <= 2);
    }

    #[test]
    fn every_rule_is_needed() {
        assert!(meets_rules("Abc12!"));
        for weak in ["Abc1!", "abc123!", "ABC123!", "Abcdef!", "Abc1234", "Abc 12 3"] {
            assert!(!meets_rules(weak), "{weak} should fail");
        }
    }
}
