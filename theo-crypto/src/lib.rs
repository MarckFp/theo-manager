//! # theo-crypto
//!
//! The cryptography of Theo Manager (`REQUIREMENTS.md` section 6). Everything is encrypted on the
//! device; the server only ever sees ciphertext, salts and public keys.
//!
//! | Module | What it does |
//! |---|---|
//! | [`vault`] | Congregation password → Argon2id → unwraps the 32-byte **root key**; optional recovery key (R-311) |
//! | [`keyring`] | Versioned **key domains** (R-316) wrapped by the root key; record keys, blind indexes, domain X-Wing keys |
//! | [`envelope`] | The record format of 6.4: suite 0x01 (domain key) and 0x02 (sealed to X-Wing recipients, R-327), AAD binding, Padmé |
//! | [`device`] | Keys kept sealed on one device: *Stay unlocked* and passkey (PRF) unlock |
//! | [`account`] | Account password → `auth_key` + `account_kek`; personal keys (X-Wing, ML-DSA-65, Ed25519) |
//! | [`grant`] | A domain secret X-Wing-encapsulated to one member (R-321) |
//! | [`invite`] | Invite tokens: the proof the server checks, and the sealed bundle of domain keys (5.6) |
//! | [`sign`] | Hybrid ML-DSA-65 + Ed25519 signatures (R-323) |
//! | [`passphrase`] | Generated congregation passwords: 6 words of the EFF large list, ≈ 77.5 bits (R-314) |
//! | [`kem`] | X-Wing (X25519 + ML-KEM-768) keys |
//!
//! Symmetric parts use XChaCha20-Poly1305 with a fresh random 24-byte nonce (R-303), HKDF-SHA-256
//! with `"theo/v1/…"` labels, and Argon2id. Public-key parts are always **hybrid** (classical +
//! NIST PQC), so they hold as long as either half is unbroken. Secrets live in `Zeroizing` types
//! (R-309) and unknown formats fail closed (R-307).
//!
//! ## Backup envelope
//!
//! The **password envelope** for congregation backups: Argon2id turns the congregation password
//! into a key, and XChaCha20-Poly1305 encrypts the data and detects any change to it.
//!
//! Envelope format (all integers little-endian):
//!
//! ```text
//! magic "THBK" (4) | version (1) = 1 | argon2 m_kib (4) | t (4) | p (4) | salt (16) | nonce (24)
//! | ciphertext ‖ tag (16)
//! ```
//!
//! Everything before the ciphertext is the header. It is authenticated as associated data, so
//! nobody can lower the Argon2 parameters or swap the salt without the tag failing.

pub mod account;
pub mod device;
pub mod envelope;
pub mod grant;
pub mod invite;
pub mod kem;
pub mod keyring;
pub mod passphrase;
pub mod sign;
mod util;
pub mod vault;

pub use account::{account_salt, AccountKeys, PersonalKeys, PublicIdentity};
pub use envelope::RecordContext;
pub use kem::{KemPublicKey, KemSecretKey};
pub use keyring::{DomainId, DomainKeys, DomainSecret, Keyring};
pub use sign::{HybridSigner, HybridVerifier};
pub use vault::{RecoveryKey, RootKey, Vault};

use argon2::{Algorithm, Argon2, Params, Version};
use chacha20poly1305::aead::{Aead, KeyInit, Payload};
use chacha20poly1305::{XChaCha20Poly1305, XNonce};
use zeroize::Zeroizing;

const MAGIC: &[u8; 4] = b"THBK";
const VERSION: u8 = 1;
const SALT_LEN: usize = 16;
const NONCE_LEN: usize = 24;
const HEADER_LEN: usize = 4 + 1 + 12 + SALT_LEN + NONCE_LEN;

/// Argon2id cost. Stored in every envelope, so it can be raised later (R-302).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KdfParams {
    /// Memory in KiB.
    pub m_kib: u32,
    /// Passes.
    pub t: u32,
    /// Lanes.
    pub p: u32,
}

impl KdfParams {
    /// RFC 9106 second recommended option: 64 MiB, 3 passes, 4 lanes (R-301).
    pub const DEFAULT: KdfParams = KdfParams { m_kib: 64 * 1024, t: 3, p: 4 };
    /// OWASP minimum; anything below it is refused (R-301).
    pub const MINIMUM: KdfParams = KdfParams { m_kib: 19 * 1024, t: 2, p: 1 };

    /// Refuses parameters below [`KdfParams::MINIMUM`].
    pub fn check(self) -> Result<(), Error> {
        if self.m_kib < Self::MINIMUM.m_kib || self.t < Self::MINIMUM.t || self.p < Self::MINIMUM.p {
            return Err(Error::WeakParameters);
        }
        Ok(())
    }
}

/// Why an envelope could not be made or opened.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    /// The bytes are not a Theo Manager envelope.
    NotAnEnvelope,
    /// Made by a newer app version.
    UnsupportedVersion,
    /// The Argon2 parameters are invalid or too weak.
    WeakParameters,
    /// Wrong password, or the file was changed or damaged.
    WrongPasswordOrDamaged,
    /// The system could not give random bytes.
    Random,
    /// The data was changed or damaged (its authentication tag does not match).
    Damaged,
    /// A key has the wrong size or shape, or an input is too long.
    InvalidKey,
    /// The envelope format is newer than this app (R-307).
    UnknownFormat,
    /// The encryption suite is unknown (R-307).
    UnknownSuite,
    /// This device does not hold the key needed (domain version, or not a recipient).
    NoKey,
    /// A signature is not valid.
    BadSignature,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Error::NotAnEnvelope => "This file is not a Theo Manager backup.",
            Error::UnsupportedVersion => "This backup was made by a newer version of Theo Manager. Update the app first.",
            Error::WeakParameters => "This backup uses invalid encryption settings.",
            Error::WrongPasswordOrDamaged => "Wrong password, or the file was changed or damaged.",
            Error::Random => "The device could not create random numbers.",
            Error::Damaged => "The data was changed or damaged.",
            Error::InvalidKey => "A key or value has an invalid size or form.",
            Error::UnknownFormat => "This data was made by a newer version of Theo Manager. Update the app first.",
            Error::UnknownSuite => "This data uses an unknown encryption method.",
            Error::NoKey => "You do not have the key to open this data.",
            Error::BadSignature => "The signature is not valid.",
        })
    }
}

impl std::error::Error for Error {}

pub(crate) fn derive_key(password: &str, salt: &[u8], params: KdfParams) -> Result<Zeroizing<[u8; 32]>, Error> {
    let p = Params::new(params.m_kib, params.t, params.p, Some(32)).map_err(|_| Error::WeakParameters)?;
    let mut key = Zeroizing::new([0u8; 32]);
    Argon2::new(Algorithm::Argon2id, Version::V0x13, p)
        .hash_password_into(password.as_bytes(), salt, key.as_mut())
        .map_err(|_| Error::WeakParameters)?;
    Ok(key)
}

fn header(params: KdfParams, salt: &[u8; SALT_LEN], nonce: &[u8; NONCE_LEN]) -> Vec<u8> {
    let mut h = Vec::with_capacity(HEADER_LEN);
    h.extend_from_slice(MAGIC);
    h.push(VERSION);
    h.extend_from_slice(&params.m_kib.to_le_bytes());
    h.extend_from_slice(&params.t.to_le_bytes());
    h.extend_from_slice(&params.p.to_le_bytes());
    h.extend_from_slice(salt);
    h.extend_from_slice(nonce);
    h
}

/// Encrypts `plaintext` with a key derived from `password`.
pub fn seal_with_password(password: &str, plaintext: &[u8], params: KdfParams) -> Result<Vec<u8>, Error> {
    let mut salt = [0u8; SALT_LEN];
    let mut nonce = [0u8; NONCE_LEN];
    getrandom::fill(&mut salt).map_err(|_| Error::Random)?;
    getrandom::fill(&mut nonce).map_err(|_| Error::Random)?;
    let key = derive_key(password, &salt, params)?;
    let mut out = header(params, &salt, &nonce);
    let cipher = XChaCha20Poly1305::new_from_slice(key.as_ref()).map_err(|_| Error::WeakParameters)?;
    let ct = cipher
        .encrypt(&XNonce::from(nonce), Payload { msg: plaintext, aad: &out })
        .map_err(|_| Error::WrongPasswordOrDamaged)?;
    out.extend_from_slice(&ct);
    Ok(out)
}

/// Decrypts an envelope made by [`seal_with_password`].
pub fn open_with_password(password: &str, sealed: &[u8]) -> Result<Vec<u8>, Error> {
    if sealed.len() < HEADER_LEN + 16 || &sealed[..4] != MAGIC {
        return Err(Error::NotAnEnvelope);
    }
    if sealed[4] != VERSION {
        return Err(Error::UnsupportedVersion);
    }
    let u32_at = |i: usize| u32::from_le_bytes([sealed[i], sealed[i + 1], sealed[i + 2], sealed[i + 3]]);
    let params = KdfParams { m_kib: u32_at(5), t: u32_at(9), p: u32_at(13) };
    params.check()?;
    let salt = &sealed[17..17 + SALT_LEN];
    let nonce = &sealed[17 + SALT_LEN..HEADER_LEN];
    let key = derive_key(password, salt, params)?;
    let cipher = XChaCha20Poly1305::new_from_slice(key.as_ref()).map_err(|_| Error::WeakParameters)?;
    cipher
        .decrypt(&XNonce::try_from(nonce).map_err(|_| Error::NotAnEnvelope)?, Payload { msg: &sealed[HEADER_LEN..], aad: &sealed[..HEADER_LEN] })
        .map_err(|_| Error::WrongPasswordOrDamaged)
}

#[cfg(test)]
mod tests {
    use super::*;

    // The minimum allowed cost, to keep the tests fast.
    const TEST: KdfParams = KdfParams::MINIMUM;

    #[test]
    fn round_trip() {
        let sealed = seal_with_password("correct horse", b"congregation data", TEST).unwrap_or_default();
        assert_eq!(&sealed[..4], b"THBK");
        assert_eq!(open_with_password("correct horse", &sealed).unwrap_or_default(), b"congregation data");
    }

    #[test]
    fn wrong_password_and_changes_are_detected() {
        let sealed = seal_with_password("correct horse", b"congregation data", TEST).unwrap_or_default();
        assert_eq!(open_with_password("wrong", &sealed), Err(Error::WrongPasswordOrDamaged));
        // One changed byte in the ciphertext, or in the header (here the salt).
        for i in [sealed.len() - 1, 20] {
            let mut bad = sealed.clone();
            bad[i] ^= 1;
            assert_eq!(open_with_password("correct horse", &bad), Err(Error::WrongPasswordOrDamaged));
        }
    }

    #[test]
    fn weak_or_foreign_files_are_refused() {
        assert_eq!(open_with_password("x", b"hello, this is not a backup at all, really not"), Err(Error::NotAnEnvelope));
        let mut sealed = seal_with_password("pw", b"data", TEST).unwrap_or_default();
        // Lower the memory below the minimum: refused before any decryption.
        sealed[5..9].copy_from_slice(&1024u32.to_le_bytes());
        assert_eq!(open_with_password("pw", &sealed), Err(Error::WeakParameters));
        // Two backups of the same data differ (random salt and nonce).
        let a = seal_with_password("pw", b"data", TEST).unwrap_or_default();
        let b = seal_with_password("pw", b"data", TEST).unwrap_or_default();
        assert_ne!(a, b);
    }
}
