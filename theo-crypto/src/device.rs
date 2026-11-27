//! Keys kept on one device (REQUIREMENTS.md 6.5): *Stay unlocked* and passkey unlock.
//!
//! Both keep the key that opens a congregation (its root key, or a Cloud account's keys) on the
//! device, sealed with a key the device holds:
//!
//! - **Stay unlocked:** a random *device key* (on the web a non-extractable WebCrypto key; in the
//!   apps a key file in the app's own data folder), only until the chosen time runs out.
//! - **Passkey:** the passkey's PRF output (WebAuthn `prf` extension) → HKDF → the sealing key. The
//!   authenticator computes it only after the fingerprint, face or PIN, so the sealed key opens
//!   only with the passkey.
//!
//! ```text
//! sealed = magic "THDV" (4) | format (1) = 1 | nonce (24) | ciphertext ‖ tag
//! AAD    = magic | format | "theo/v1/device" | put(purpose)
//! ```

use zeroize::Zeroizing;

use crate::util::{self, Reader, Secret32};
use crate::Error;

const MAGIC: &[u8; 4] = b"THDV";
const FORMAT: u8 = 1;

fn aad(purpose: &str) -> Vec<u8> {
    let mut a = MAGIC.to_vec();
    a.push(FORMAT);
    a.extend_from_slice(b"theo/v1/device");
    util::put(&mut a, purpose.as_bytes());
    a
}

/// A fresh random device key.
pub fn new_key() -> Result<Secret32, Error> {
    util::random_secret()
}

/// The sealing key from a passkey's PRF output (32 bytes from the authenticator).
pub fn key_from_prf(prf: &[u8]) -> Result<Secret32, Error> {
    if prf.len() < 32 {
        return Err(Error::InvalidKey);
    }
    Ok(util::hkdf32(prf, "theo/v1/passkey-seal"))
}

/// Seals `plain` for `purpose` (for example `"passkey:<congregation id>"`), so a sealed value
/// cannot be moved to another purpose.
pub fn seal(key: &[u8; 32], purpose: &str, plain: &[u8]) -> Result<Vec<u8>, Error> {
    let mut out = MAGIC.to_vec();
    out.push(FORMAT);
    out.extend_from_slice(&util::seal(key, &aad(purpose), plain)?);
    Ok(out)
}

/// Opens what [`seal`] made. A wrong key or purpose gives [`Error::WrongPasswordOrDamaged`].
pub fn open(key: &[u8; 32], purpose: &str, sealed: &[u8]) -> Result<Zeroizing<Vec<u8>>, Error> {
    let mut r = Reader::new(sealed);
    if r.take(4)? != MAGIC {
        return Err(Error::NotAnEnvelope);
    }
    if r.u8()? != FORMAT {
        return Err(Error::UnknownFormat);
    }
    util::open(key, &aad(purpose), r.rest()).map_err(|_| Error::WrongPasswordOrDamaged)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{KdfParams, Vault};

    #[test]
    fn sealed_keys_open_only_with_their_key_and_purpose() -> Result<(), Error> {
        let (_, root) = Vault::create("cong-1", "pw", KdfParams::MINIMUM)?;
        let key = key_from_prf(&[7u8; 32])?;
        let sealed = seal(&key, "passkey:cong-1", root.to_bytes().as_ref())?;
        let back = crate::RootKey::from_bytes(&open(&key, "passkey:cong-1", &sealed)?)?;
        assert_eq!(back.record_id("people", "1"), root.record_id("people", "1"));
        assert!(open(&key, "passkey:cong-2", &sealed).is_err());
        let other = key_from_prf(&[8u8; 32])?;
        assert!(open(&other, "passkey:cong-1", &sealed).is_err());
        let fresh = new_key()?;
        assert!(open(&fresh, "passkey:cong-1", &sealed).is_err());
        assert!(key_from_prf(&[1u8; 16]).is_err());
        Ok(())
    }
}
