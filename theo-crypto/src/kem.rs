//! X-Wing (X25519 + ML-KEM-768) key encapsulation (6.1): the hybrid public-key part of grants and
//! sealed records. A secret stays safe as long as either X25519 or ML-KEM-768 is unbroken.

use sha2::{Digest, Sha256};
use x_wing::{Decapsulate, Decapsulator, KeyExport};

use crate::Error;
use crate::util::{self, Secret32};

/// Size of an encoded public key.
pub const PUBLIC_KEY_LEN: usize = x_wing::ENCAPSULATION_KEY_SIZE;
/// Size of an encapsulation (what a recipient needs to recover the shared secret).
pub const CIPHERTEXT_LEN: usize = x_wing::CIPHERTEXT_SIZE;
/// Size of a key id.
pub const KEY_ID_LEN: usize = 8;

/// A public X-Wing key: anyone can encapsulate a secret to it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KemPublicKey(x_wing::EncapsulationKey);

impl KemPublicKey {
    /// The 1216-byte encoding.
    pub fn to_bytes(&self) -> Vec<u8> {
        self.0.to_bytes().to_vec()
    }

    /// Reads [`KemPublicKey::to_bytes`]; refuses malformed keys.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        x_wing::EncapsulationKey::try_from(bytes).map(KemPublicKey).map_err(|_| Error::InvalidKey)
    }

    /// A short, public name for the key: the first 8 bytes of SHA-256 of its encoding. It lets a
    /// reader find its own entry among the recipients of a sealed record.
    pub fn key_id(&self) -> [u8; KEY_ID_LEN] {
        let digest = Sha256::new().chain_update(b"theo/v1/key-id").chain_update(self.0.to_bytes()).finalize();
        let mut id = [0u8; KEY_ID_LEN];
        id.copy_from_slice(&digest[..KEY_ID_LEN]);
        id
    }

    /// Makes a fresh shared secret for the holder of the private key: (encapsulation, secret).
    pub(crate) fn encapsulate(&self) -> Result<(Vec<u8>, Secret32), Error> {
        let randomness = zeroize::Zeroizing::new(util::random::<{ x_wing::ENCAPSULATION_RANDOMNESS_SIZE }>()?);
        let (ct, ss) = self.0.encapsulate_deterministic(&(*randomness).into());
        Ok((ct.to_vec(), util::secret_from(&ss)?))
    }
}

/// A private X-Wing key, kept as its 32-byte seed and wiped on drop.
#[derive(Clone)]
pub struct KemSecretKey(x_wing::DecapsulationKey);

impl std::fmt::Debug for KemSecretKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("KemSecretKey(..)")
    }
}

impl KemSecretKey {
    /// The key for a 32-byte seed (the seed is the whole private key).
    pub fn from_seed(seed: &[u8; 32]) -> Self {
        KemSecretKey(x_wing::DecapsulationKey::from(*seed))
    }

    /// Its public key.
    pub fn public_key(&self) -> KemPublicKey {
        KemPublicKey(self.0.encapsulation_key().clone())
    }

    /// Recovers the shared secret from an encapsulation made to [`KemSecretKey::public_key`].
    /// A wrong key gives a different secret (X-Wing never fails), so the AEAD that follows is what
    /// detects it.
    pub(crate) fn decapsulate(&self, ct: &[u8]) -> Result<Secret32, Error> {
        let ss = self.0.decapsulate_slice(ct).map_err(|_| Error::NotAnEnvelope)?;
        util::secret_from(&ss)
    }
}

/// Size of what [`wrap_to`] returns.
pub(crate) const WRAPPED_LEN: usize = CIPHERTEXT_LEN + 32 + util::TAG_LEN;

/// Wraps a 32-byte `secret` for the holder of `to`: `encapsulation ‖ wrapped secret (48)`.
/// `label` and `aad` say what the secret is for, so a wrapped secret cannot be reused elsewhere.
pub(crate) fn wrap_to(to: &KemPublicKey, label: &str, aad: &[u8], secret: &[u8; 32]) -> Result<Vec<u8>, Error> {
    let (mut out, ss) = to.encapsulate()?;
    let key = util::hkdf32(ss.as_ref(), label);
    // The key comes from a fresh encapsulation and encrypts only this one secret, so a fixed nonce
    // is safe here.
    out.extend_from_slice(&util::seal_with_nonce(&key, &[0u8; util::NONCE_LEN], aad, secret)?);
    Ok(out)
}

/// Opens what [`wrap_to`] made. A wrong key, label or `aad` gives [`Error::NoKey`].
pub(crate) fn unwrap_with(sk: &KemSecretKey, label: &str, aad: &[u8], wrapped: &[u8]) -> Result<Secret32, Error> {
    if wrapped.len() != WRAPPED_LEN {
        return Err(Error::NotAnEnvelope);
    }
    let (ct, sealed) = wrapped.split_at(CIPHERTEXT_LEN);
    let ss = sk.decapsulate(ct)?;
    let key = util::hkdf32(ss.as_ref(), label);
    let secret = util::open_with_nonce(&key, &[0u8; util::NONCE_LEN], aad, sealed).map_err(|_| Error::NoKey)?;
    util::secret_from(&secret)
}
