//! Small building blocks shared by the modules: randomness, HKDF, the AEAD and length-prefixed
//! encodings.

use chacha20poly1305::aead::{Aead, KeyInit, Payload};
use chacha20poly1305::{XChaCha20Poly1305, XNonce};
use hkdf::Hkdf;
use sha2::Sha256;
use zeroize::Zeroizing;

use crate::Error;

/// A 32-byte secret that is wiped when dropped (R-309).
pub type Secret32 = Zeroizing<[u8; 32]>;

pub const NONCE_LEN: usize = 24;
pub const TAG_LEN: usize = 16;

/// Fills a fresh array from the system random generator.
pub fn random<const N: usize>() -> Result<[u8; N], Error> {
    let mut b = [0u8; N];
    getrandom::fill(&mut b).map_err(|_| Error::Random)?;
    Ok(b)
}

/// A fresh random secret.
pub fn random_secret() -> Result<Secret32, Error> {
    let mut s = Zeroizing::new([0u8; 32]);
    getrandom::fill(s.as_mut()).map_err(|_| Error::Random)?;
    Ok(s)
}

/// HKDF-SHA-256: one 32-byte key for `label` (a "theo/v1/..." string) from `ikm`.
pub fn hkdf32(ikm: &[u8], label: &str) -> Secret32 {
    let mut out = Zeroizing::new([0u8; 32]);
    // 32 bytes is far below the HKDF-SHA-256 maximum (8160), so expand cannot fail.
    let _ = Hkdf::<Sha256>::new(None, ikm).expand(label.as_bytes(), out.as_mut());
    out
}

/// Appends `bytes` with a 4-byte little-endian length first, so that joined fields cannot be
/// confused with each other (`"ab" ‖ "c"` differs from `"a" ‖ "bc"`).
pub fn put(out: &mut Vec<u8>, bytes: &[u8]) {
    out.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
    out.extend_from_slice(bytes);
}

/// XChaCha20-Poly1305 with a fresh random nonce (R-303): returns `nonce ‖ ciphertext ‖ tag`.
pub fn seal(key: &[u8; 32], aad: &[u8], plaintext: &[u8]) -> Result<Vec<u8>, Error> {
    let nonce = random::<NONCE_LEN>()?;
    let mut out = nonce.to_vec();
    out.extend_from_slice(&seal_with_nonce(key, &nonce, aad, plaintext)?);
    Ok(out)
}

/// Opens what [`seal`] made.
pub fn open(key: &[u8; 32], aad: &[u8], sealed: &[u8]) -> Result<Zeroizing<Vec<u8>>, Error> {
    if sealed.len() < NONCE_LEN + TAG_LEN {
        return Err(Error::Damaged);
    }
    let (nonce, ct) = sealed.split_at(NONCE_LEN);
    open_with_nonce(key, nonce, aad, ct)
}

/// The AEAD with a given nonce. Only for keys that encrypt exactly one message, or with a nonce
/// from [`random`].
pub fn seal_with_nonce(key: &[u8; 32], nonce: &[u8; NONCE_LEN], aad: &[u8], plaintext: &[u8]) -> Result<Vec<u8>, Error> {
    let cipher = XChaCha20Poly1305::new_from_slice(key).map_err(|_| Error::InvalidKey)?;
    cipher.encrypt(&XNonce::from(*nonce), Payload { msg: plaintext, aad }).map_err(|_| Error::Damaged)
}

/// Opens what [`seal_with_nonce`] made.
pub fn open_with_nonce(key: &[u8; 32], nonce: &[u8], aad: &[u8], ct: &[u8]) -> Result<Zeroizing<Vec<u8>>, Error> {
    let cipher = XChaCha20Poly1305::new_from_slice(key).map_err(|_| Error::InvalidKey)?;
    let nonce = XNonce::try_from(nonce).map_err(|_| Error::Damaged)?;
    cipher.decrypt(&nonce, Payload { msg: ct, aad }).map(Zeroizing::new).map_err(|_| Error::Damaged)
}

/// Copies exactly 32 bytes into a secret.
pub fn secret_from(bytes: &[u8]) -> Result<Secret32, Error> {
    let mut s = Zeroizing::new([0u8; 32]);
    if bytes.len() != 32 {
        return Err(Error::Damaged);
    }
    s.copy_from_slice(bytes);
    Ok(s)
}

/// A reader over bytes that fails closed when the input is too short.
pub struct Reader<'a> {
    rest: &'a [u8],
}

impl<'a> Reader<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        Reader { rest: bytes }
    }

    pub fn take(&mut self, n: usize) -> Result<&'a [u8], Error> {
        if self.rest.len() < n {
            return Err(Error::NotAnEnvelope);
        }
        let (a, b) = self.rest.split_at(n);
        self.rest = b;
        Ok(a)
    }

    pub fn u8(&mut self) -> Result<u8, Error> {
        Ok(self.take(1)?[0])
    }

    pub fn u32(&mut self) -> Result<u32, Error> {
        let b = self.take(4)?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    pub fn rest(self) -> &'a [u8] {
        self.rest
    }
}
