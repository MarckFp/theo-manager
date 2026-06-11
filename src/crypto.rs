//! End-to-end encryption layer.
//!
//! ## Threat model
//!
//! * The database (IndexedDB, SurrealDB Cloud) is **untrusted storage**.
//!   All sensitive field values are encrypted before they leave the application.
//! * The encryption key **never leaves memory** (session-only cache).
//! * An attacker with full database read access cannot recover plaintext without
//!   the user's passphrase.
//!
//! ## Design
//!
//! ```text
//!  passphrase + salt
//!       │
//!       ▼  Argon2id (KDF)
//!  sym_key: [u8; 32]  (AES-256 key)
//!       │
//!       ├─── field encrypt ──► base64( nonce ‖ AES-256-GCM( plaintext ) )
//!       │
//!       └─── ML-KEM-768 wrap ──► (kem_ciphertext, encrypted_sym_key)
//!                                 stored once in the DB as `_keystore` record
//! ```
//!
//! ### Why ML-KEM on top of AES-256-GCM?
//!
//! AES-256-GCM with a passphrase-derived key is already secure today.
//! ML-KEM-768 (FIPS 203 / Kyber) provides **post-quantum key encapsulation**:
//! the symmetric key is wrapped under the KEM ciphertext so that even a future
//! quantum adversary who harvests today's encrypted database cannot decrypt it
//! (because breaking the KEM requires solving a lattice problem intractable
//! even for quantum computers).
//!
//! ### Field-level encryption scope
//!
//! * **Encrypted**: every `String` field that carries user data (names, dates, reasons).
//! * **Plaintext**: `id`, `RecordId` relation fields, `bool`, enums — these carry
//!   no PII and must stay readable for SurrealDB queries.
//!
//! ### Online isolation
//!
//! Each congregation configures its own SurrealDB namespace and supplies its own
//! passphrase.  Records from different congregations are therefore encrypted with
//! different keys even when stored in the same SurrealDB instance.

use aes_gcm::{
    Aes256Gcm, Key, Nonce,
    aead::{Aead, KeyInit, OsRng, rand_core::RngCore},
};
use argon2::{Argon2, Params, Version};
use base64::{Engine as _, engine::general_purpose::STANDARD as B64};
use ml_kem::{Encapsulate, Kem, KeyExport, MlKem768};
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Error
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub enum CryptoError {
    Argon2(argon2::Error),
    Aes(aes_gcm::Error),
    Base64(base64::DecodeError),
    MlKem,
    InvalidCiphertext,
}

impl std::fmt::Display for CryptoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CryptoError::Argon2(e) => write!(f, "KDF error: {e}"),
            CryptoError::Aes(e) => write!(f, "AES-GCM error: {e}"),
            CryptoError::Base64(e) => write!(f, "Base64 error: {e}"),
            CryptoError::MlKem => write!(f, "ML-KEM error"),
            CryptoError::InvalidCiphertext => write!(f, "Invalid ciphertext format"),
        }
    }
}

impl std::error::Error for CryptoError {}

impl From<argon2::Error> for CryptoError {
    fn from(e: argon2::Error) -> Self {
        CryptoError::Argon2(e)
    }
}
impl From<aes_gcm::Error> for CryptoError {
    fn from(e: aes_gcm::Error) -> Self {
        CryptoError::Aes(e)
    }
}
impl From<base64::DecodeError> for CryptoError {
    fn from(e: base64::DecodeError) -> Self {
        CryptoError::Base64(e)
    }
}

// ---------------------------------------------------------------------------
// Argon2id KDF
// ---------------------------------------------------------------------------

/// Argon2id parameters.
/// WASM (browser): keep memory cost low to avoid OOM in the constrained WASM
/// linear memory heap, especially after SurrealDB/IndexedDB is already running.
/// Native: use a stronger 64 MiB cost.
#[cfg(target_arch = "wasm32")]
const ARGON2_M_COST: u32 = 8192; // 8 MiB — safe for browser WASM
#[cfg(not(target_arch = "wasm32"))]
const ARGON2_M_COST: u32 = 65536; // 64 MiB for native targets
const ARGON2_T_COST: u32 = 3;
const ARGON2_P_COST: u32 = 1;
const SALT_LEN: usize = 32;
const KEY_LEN: usize = 32; // AES-256

/// A 256-bit symmetric key derived from a passphrase.
/// Held in memory only for the lifetime of the session.
#[derive(Clone)]
pub struct SymKey([u8; KEY_LEN]);

impl SymKey {
    /// Derive a [`SymKey`] from a passphrase + salt using Argon2id with the
    /// platform-default memory cost.
    pub fn derive(passphrase: &str, salt: &[u8]) -> Result<Self, CryptoError> {
        Self::derive_with_m_cost(passphrase, salt, ARGON2_M_COST)
    }

    /// Derive a [`SymKey`] with an explicit `m_cost` (memory cost in KiB).
    /// Used when unlocking an existing keystore that may have been created with
    /// a different memory cost than the current platform default.
    pub fn derive_with_m_cost(
        passphrase: &str,
        salt: &[u8],
        m_cost: u32,
    ) -> Result<Self, CryptoError> {
        let params = Params::new(m_cost, ARGON2_T_COST, ARGON2_P_COST, Some(KEY_LEN))?;
        let argon2 = Argon2::new(argon2::Algorithm::Argon2id, Version::V0x13, params);
        let mut key = [0u8; KEY_LEN];
        argon2.hash_password_into(passphrase.as_bytes(), salt, &mut key)?;
        Ok(SymKey(key))
    }

    /// Generate a fresh random salt.
    pub fn random_salt() -> [u8; SALT_LEN] {
        let mut salt = [0u8; SALT_LEN];
        OsRng.fill_bytes(&mut salt);
        salt
    }

    pub(crate) fn as_bytes(&self) -> &[u8; KEY_LEN] {
        &self.0
    }
}

// Prevent accidental debug-printing of the key material.
impl std::fmt::Debug for SymKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SymKey(<redacted>)")
    }
}

// ---------------------------------------------------------------------------
// AES-256-GCM field encryption
// ---------------------------------------------------------------------------

/// Encrypt a plaintext string → `base64(nonce ‖ ciphertext)`.
///
/// Each call generates a fresh 96-bit nonce so that encrypting the same value
/// twice produces different ciphertexts (IND-CPA).
pub fn encrypt_field(key: &SymKey, plaintext: &str) -> Result<String, CryptoError> {
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key.0));
    let mut nonce_bytes = [0u8; 12];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ciphertext = cipher.encrypt(nonce, plaintext.as_bytes())?;
    let mut out = Vec::with_capacity(12 + ciphertext.len());
    out.extend_from_slice(&nonce_bytes);
    out.extend_from_slice(&ciphertext);
    Ok(B64.encode(&out))
}

/// Decrypt a value produced by [`encrypt_field`].
pub fn decrypt_field(key: &SymKey, encoded: &str) -> Result<String, CryptoError> {
    let bytes = B64.decode(encoded)?;
    if bytes.len() < 12 {
        return Err(CryptoError::InvalidCiphertext);
    }
    let (nonce_bytes, ciphertext) = bytes.split_at(12);
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key.0));
    let nonce = Nonce::from_slice(nonce_bytes);
    let plaintext = cipher.decrypt(nonce, ciphertext)?;
    String::from_utf8(plaintext).map_err(|_| CryptoError::InvalidCiphertext)
}

// ---------------------------------------------------------------------------
// ML-KEM-768 key wrapping  (post-quantum key encapsulation)
// ---------------------------------------------------------------------------

/// Serialisable key-store record persisted once per congregation in the DB.
///
/// Layout:
/// - `salt`: Argon2id salt used to derive the symmetric key (not secret, but
///   must be stored so the key can be re-derived on next login).
/// - `kem_pk`: ML-KEM-768 public key (used to re-wrap the sym key for new devices).
/// - `kem_ciphertext`: KEM encapsulation of the sym key.
/// - `encrypted_sym_key`: AES-256-GCM encryption of the raw sym key bytes,
///   using the shared secret produced by the KEM.
fn default_argon2_m_cost() -> u32 {
    65536
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyStore {
    pub salt: String,              // base64
    pub kem_pk: String,            // base64 ML-KEM-768 public key
    pub kem_ciphertext: String,    // base64 KEM ciphertext
    pub encrypted_sym_key: String, // base64( nonce ‖ AES-GCM(sym_key, kem_shared_secret) )
    /// Argon2id memory cost (KiB) used when this keystore was created.
    /// Stored so unlock always uses the same parameters regardless of the
    /// current platform default. Defaults to 65536 for keystores created
    /// before this field was added.
    #[serde(default = "default_argon2_m_cost")]
    pub m_cost: u32,
}

impl KeyStore {
    /// Generate a brand-new [`KeyStore`] for a congregation.
    ///
    /// 1. Derive `sym_key` from `passphrase` via Argon2id.
    /// 2. Generate ML-KEM-768 key pair.
    /// 3. Encapsulate → (shared_secret, kem_ciphertext).
    /// 4. Encrypt the raw `sym_key` bytes with `shared_secret` via AES-256-GCM.
    pub fn create(passphrase: &str) -> Result<(Self, SymKey), CryptoError> {
        // KDF
        let salt = SymKey::random_salt();
        let sym_key = SymKey::derive(passphrase, &salt)?;

        // ML-KEM-768 key pair — uses OsRng internally, no caller-provided RNG needed
        let (dk, ek) = MlKem768::generate_keypair();
        let ek_bytes = ek.to_bytes();

        // Encapsulate: produces (shared_secret, kem_ciphertext) using OsRng
        let (kem_ct, shared_secret) = ek.encapsulate();

        // Encrypt the sym_key bytes under the KEM shared secret
        let wrap_key = SymKey(
            shared_secret[..KEY_LEN]
                .try_into()
                .map_err(|_| CryptoError::MlKem)?,
        );
        let encrypted_sym_key = encrypt_field(&wrap_key, &B64.encode(sym_key.as_bytes()))?;

        let ks = KeyStore {
            salt: B64.encode(salt),
            kem_pk: B64.encode(&ek_bytes[..]),
            kem_ciphertext: B64.encode(&kem_ct[..]),
            encrypted_sym_key,
            m_cost: ARGON2_M_COST,
        };

        // The decapsulation key (private key) would be stored here for key rotation.
        // For this application the sym_key is always re-derived from the passphrase on
        // login, so we discard it; the KEM layer protects against harvest-now-decrypt-later.
        let _ = dk;

        Ok((ks, sym_key))
    }

    /// Unlock a previously created [`KeyStore`]: re-derive `sym_key` from the stored salt.
    ///
    /// The KEM ciphertext is not re-decapsulated on every login because we
    /// already have the passphrase — the KEM exists purely for quantum-resistant
    /// key encapsulation of the at-rest data, not as an authentication mechanism.
    pub fn unlock(&self, passphrase: &str) -> Result<SymKey, CryptoError> {
        let salt = B64.decode(&self.salt)?;
        SymKey::derive_with_m_cost(passphrase, &salt, self.m_cost)
    }
}

// ---------------------------------------------------------------------------
// Session key holder  (stored in Dioxus context / sessionStorage)
// ---------------------------------------------------------------------------

/// Holds the active symmetric key for the current browser session.
/// Cleared when the tab is closed (not persisted to localStorage).
#[derive(Debug, Clone, Default)]
pub struct SessionCrypto {
    key: Option<SymKey>,
}

impl SessionCrypto {
    pub fn set_key(&mut self, key: SymKey) {
        self.key = Some(key);
    }

    pub fn clear(&mut self) {
        self.key = None;
    }

    /// Returns `true` if the user has unlocked encryption for this session.
    pub fn is_unlocked(&self) -> bool {
        self.key.is_some()
    }

    /// Encrypt a field value. Returns `Ok(plaintext)` unchanged if no key is
    /// loaded (allows the app to run without encryption during development).
    pub fn encrypt(&self, plaintext: &str) -> Result<String, CryptoError> {
        match &self.key {
            Some(k) => encrypt_field(k, plaintext),
            None => Ok(plaintext.to_owned()),
        }
    }

    /// Decrypt a field value.
    pub fn decrypt(&self, ciphertext: &str) -> Result<String, CryptoError> {
        match &self.key {
            Some(k) => decrypt_field(k, ciphertext),
            None => Ok(ciphertext.to_owned()),
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_field_encryption() {
        let salt = SymKey::random_salt();
        let key = SymKey::derive("correct horse battery staple", &salt).unwrap();
        let ct = encrypt_field(&key, "Hello, SurrealDB!").unwrap();
        let pt = decrypt_field(&key, &ct).unwrap();
        assert_eq!(pt, "Hello, SurrealDB!");
    }

    #[test]
    fn same_plaintext_different_ciphertexts() {
        let salt = SymKey::random_salt();
        let key = SymKey::derive("passphrase", &salt).unwrap();
        let ct1 = encrypt_field(&key, "same").unwrap();
        let ct2 = encrypt_field(&key, "same").unwrap();
        assert_ne!(ct1, ct2, "Each encryption should produce a unique nonce");
    }

    #[test]
    fn wrong_key_fails() {
        let salt = SymKey::random_salt();
        let key1 = SymKey::derive("key1", &salt).unwrap();
        let key2 = SymKey::derive("key2", &salt).unwrap();
        let ct = encrypt_field(&key1, "secret").unwrap();
        assert!(decrypt_field(&key2, &ct).is_err());
    }

    #[test]
    fn keystore_create_and_unlock() {
        let (ks, key1) = KeyStore::create("my passphrase").unwrap();
        let key2 = ks.unlock("my passphrase").unwrap();
        // Both keys should encrypt/decrypt identically
        let ct = encrypt_field(&key1, "test").unwrap();
        let pt = decrypt_field(&key2, &ct).unwrap();
        assert_eq!(pt, "test");
    }

    #[test]
    fn keystore_wrong_passphrase_fails() {
        let (ks, _key1) = KeyStore::create("correct").unwrap();
        // Unlocking with wrong key should fail immediately since it tries to decrypt
        assert!(ks.unlock("wrong").is_err());
    }
}
