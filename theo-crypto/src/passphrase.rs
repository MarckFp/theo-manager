//! Generated congregation passwords (R-314): six words from the EFF large word list, each chosen
//! uniformly with the operating system's secure random numbers. 6 × log2(7,776) ≈ 77.5 bits.
//!
//! Word list: "EFF's Long Wordlist" by the Electronic Frontier Foundation
//! (<https://www.eff.org/dice>), licensed under CC BY 3.0 US
//! (<https://creativecommons.org/licenses/by/3.0/us/>). Stored here as one word per line, without
//! the dice numbers (`eff_large_wordlist.txt`, from the file with SHA-256
//! `addd35536511597a02fa0a9ff1e5284677b8883b83e986e43f15a3db996b903e`).

use crate::util;
use crate::Error;

const LIST: &str = include_str!("eff_large_wordlist.txt");

/// How many words a generated passphrase has.
pub const WORDS: usize = 6;

/// The words of the list (7,776).
pub fn words() -> impl Iterator<Item = &'static str> {
    LIST.lines()
}

/// A uniform random number below `n` (rejection sampling, so no word is more likely).
fn below(n: u32) -> Result<u32, Error> {
    let limit = u32::MAX - u32::MAX % n;
    loop {
        let x = u32::from_le_bytes(util::random::<4>()?);
        if x < limit {
            return Ok(x % n);
        }
    }
}

/// A new passphrase of [`WORDS`] words joined by `-`.
pub fn generate() -> Result<String, Error> {
    let list: Vec<&str> = words().collect();
    let n = u32::try_from(list.len()).map_err(|_| Error::Damaged)?;
    let mut out = Vec::with_capacity(WORDS);
    for _ in 0..WORDS {
        out.push(list[below(n)? as usize]);
    }
    Ok(out.join("-"))
}

/// The strength of a generated passphrase, in bits.
pub fn entropy_bits() -> f64 {
    WORDS as f64 * (words().count() as f64).log2()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn passphrases_use_the_whole_list_and_are_strong() -> Result<(), Error> {
        let list: Vec<&str> = words().collect();
        assert_eq!(list.len(), 7776);
        assert_eq!(list.iter().collect::<std::collections::BTreeSet<_>>().len(), 7776);
        assert!(entropy_bits() > 77.0);
        let a = generate()?;
        let b = generate()?;
        assert_ne!(a, b);
        // Six words (four list words contain a hyphen themselves, so count by the list).
        let mut rest = a.as_str();
        for i in 0..WORDS {
            let w = list.iter().filter(|w| rest.starts_with(*w) && (rest.len() == w.len() || rest[w.len()..].starts_with('-'))).max_by_key(|w| w.len());
            let w = w.ok_or(Error::Damaged)?;
            rest = &rest[w.len()..];
            if i + 1 < WORDS {
                rest = rest.strip_prefix('-').ok_or(Error::Damaged)?;
            }
        }
        assert!(rest.is_empty());
        // Every index is reachable and roughly uniform.
        let mut seen = [0u32; 6];
        for _ in 0..6000 {
            seen[below(6)? as usize] += 1;
        }
        assert!(seen.iter().all(|c| (800..1200).contains(c)), "{seen:?}");
        Ok(())
    }
}
