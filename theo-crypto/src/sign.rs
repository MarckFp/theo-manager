//! Hybrid signatures (R-323, phase 4): ML-DSA-65 + Ed25519. A signature is valid only when
//! **both** halves verify, so it holds as long as either algorithm is unbroken.
//!
//! ```text
//! signature = format (1) = 0x01 | ML-DSA-65 signature (3309) | Ed25519 signature (64)
//! public    = ML-DSA-65 verifying key (1952) | Ed25519 verifying key (32)
//! ```
//!
//! Both halves sign the same message under a context string (for example `"report"`), so a
//! signature made for one purpose is never valid for another.

use ed25519_dalek::Signer as _;
use ml_dsa::{EncodedSignature, EncodedVerifyingKey, MlDsa65};

use crate::Error;
use crate::util;

const FORMAT: u8 = 1;
/// ML-DSA-65 signature size.
const MLDSA_SIG_LEN: usize = 3309;
/// ML-DSA-65 verifying key size.
const MLDSA_VK_LEN: usize = 1952;
/// Size of a hybrid signature.
pub const SIGNATURE_LEN: usize = 1 + MLDSA_SIG_LEN + 64;
/// Size of an encoded hybrid verifying key.
pub const VERIFYING_KEY_LEN: usize = MLDSA_VK_LEN + 32;
/// Longest context string.
pub const MAX_CONTEXT_LEN: usize = 200;

/// The message Ed25519 signs (ML-DSA has its own context parameter).
fn ed_message(context: &str, msg: &[u8]) -> Vec<u8> {
    let mut m = b"theo/v1/sign".to_vec();
    util::put(&mut m, context.as_bytes());
    m.extend_from_slice(msg);
    m
}

/// The private half: both signing keys, wiped on drop.
pub struct HybridSigner {
    mldsa: ml_dsa::SigningKey<MlDsa65>,
    ed: ed25519_dalek::SigningKey,
}

impl std::fmt::Debug for HybridSigner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("HybridSigner(..)")
    }
}

impl HybridSigner {
    /// The keys for two 32-byte seeds.
    pub fn from_seeds(mldsa_seed: &[u8; 32], ed_seed: &[u8; 32]) -> Self {
        HybridSigner {
            mldsa: ml_dsa::SigningKey::from_seed(&ml_dsa::Seed::from(*mldsa_seed)),
            ed: ed25519_dalek::SigningKey::from_bytes(ed_seed),
        }
    }

    /// The public half.
    pub fn verifier(&self) -> HybridVerifier {
        HybridVerifier { mldsa: self.mldsa.expanded_key().verifying_key(), ed: self.ed.verifying_key() }
    }

    /// Signs `msg` for `context` (at most [`MAX_CONTEXT_LEN`] bytes).
    pub fn sign(&self, context: &str, msg: &[u8]) -> Result<Vec<u8>, Error> {
        if context.len() > MAX_CONTEXT_LEN {
            return Err(Error::InvalidKey);
        }
        let pq = self.mldsa.expanded_key().sign_deterministic(msg, context.as_bytes()).map_err(|_| Error::InvalidKey)?;
        let classic = self.ed.sign(&ed_message(context, msg));
        let mut out = Vec::with_capacity(SIGNATURE_LEN);
        out.push(FORMAT);
        out.extend_from_slice(&pq.encode());
        out.extend_from_slice(&classic.to_bytes());
        Ok(out)
    }
}

/// The public half: anyone can check a signature with it.
#[derive(Clone, Debug, PartialEq)]
pub struct HybridVerifier {
    mldsa: ml_dsa::VerifyingKey<MlDsa65>,
    ed: ed25519_dalek::VerifyingKey,
}

impl HybridVerifier {
    /// The encoding (`ML-DSA-65 key ‖ Ed25519 key`).
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = self.mldsa.encode().to_vec();
        out.extend_from_slice(self.ed.as_bytes());
        out
    }

    /// Reads [`HybridVerifier::to_bytes`].
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        if bytes.len() != VERIFYING_KEY_LEN {
            return Err(Error::InvalidKey);
        }
        let (pq, classic) = bytes.split_at(MLDSA_VK_LEN);
        let pq = EncodedVerifyingKey::<MlDsa65>::try_from(pq).map_err(|_| Error::InvalidKey)?;
        let classic: [u8; 32] = classic.try_into().map_err(|_| Error::InvalidKey)?;
        Ok(HybridVerifier {
            mldsa: ml_dsa::VerifyingKey::decode(&pq),
            ed: ed25519_dalek::VerifyingKey::from_bytes(&classic).map_err(|_| Error::InvalidKey)?,
        })
    }

    /// Checks a signature from [`HybridSigner::sign`]. Both halves must be valid.
    pub fn verify(&self, context: &str, msg: &[u8], signature: &[u8]) -> Result<(), Error> {
        if signature.len() != SIGNATURE_LEN || signature[0] != FORMAT {
            return Err(Error::BadSignature);
        }
        let (pq, classic) = signature[1..].split_at(MLDSA_SIG_LEN);
        let pq = EncodedSignature::<MlDsa65>::try_from(pq)
            .ok()
            .and_then(|e| ml_dsa::Signature::<MlDsa65>::decode(&e))
            .ok_or(Error::BadSignature)?;
        let classic: [u8; 64] = classic.try_into().map_err(|_| Error::BadSignature)?;
        let classic = ed25519_dalek::Signature::from_bytes(&classic);
        let pq_ok = self.mldsa.verify_with_context(msg, context.as_bytes(), &pq);
        let classic_ok = self.ed.verify_strict(&ed_message(context, msg), &classic).is_ok();
        if pq_ok && classic_ok { Ok(()) } else { Err(Error::BadSignature) }
    }
}
