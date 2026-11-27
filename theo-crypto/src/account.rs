//! Accounts (Cloud mode, 6.2): one account password per user.
//!
//! ```text
//! account password ──Argon2id(salt_a)──► account_master (never stored)
//!    ├──HKDF "theo/v1/auth"────────► auth_key     → sign-in (the server stores only argon2(auth_key))
//!    └──HKDF "theo/v1/account-kek"─► account_kek  → unwraps the member's personal keys
//! salt_a = SHA-256("theo/v1/auth-salt" ‖ server_origin ‖ namespace ‖ database ‖ normalized_username)
//! ```
//!
//! HKDF is one-way: the server knows `auth_key` but cannot compute `account_kek` from it.
//!
//! The **personal keys** are three 32-byte seeds: X-Wing (grants and own sealed records),
//! ML-DSA-65 and Ed25519 (hybrid signatures). They are stored wrapped by `account_kek`:
//!
//! ```text
//! magic "THPK" (4) | format (1) = 1 | nonce (24) | ciphertext ‖ tag (96 + 16)
//! ```

use sha2::{Digest, Sha256};
use zeroize::Zeroizing;

use crate::kem::{KemPublicKey, KemSecretKey};
use crate::sign::{HybridSigner, HybridVerifier};
use crate::util::{self, Reader, Secret32};
use crate::{Error, KdfParams};

const MAGIC: &[u8; 4] = b"THPK";
const FORMAT: u8 = 1;

/// The deterministic account salt. The client needs it before sign-in, so it cannot be random.
/// The username is trimmed and lower-cased first.
pub fn account_salt(server_origin: &str, namespace: &str, database: &str, username: &str) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(b"theo/v1/auth-salt");
    for part in [server_origin, namespace, database, &username.trim().to_lowercase()] {
        h.update((part.len() as u32).to_le_bytes());
        h.update(part.as_bytes());
    }
    h.finalize().into()
}

/// The two keys derived from the account password.
pub struct AccountKeys {
    auth_key: Secret32,
    account_kek: Secret32,
}

impl std::fmt::Debug for AccountKeys {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("AccountKeys(..)")
    }
}

impl AccountKeys {
    /// Runs Argon2id on the password (slow on purpose) and derives both keys.
    pub fn derive(password: &str, salt: &[u8; 32], params: KdfParams) -> Result<Self, Error> {
        params.check()?;
        let master = crate::derive_key(password, salt, params)?;
        Ok(AccountKeys {
            auth_key: util::hkdf32(master.as_ref(), "theo/v1/auth"),
            account_kek: util::hkdf32(master.as_ref(), "theo/v1/account-kek"),
        })
    }

    /// What the client sends to sign in. It does not reveal the account kek.
    pub fn auth_key(&self) -> &[u8; 32] {
        &self.auth_key
    }

    /// Both keys (64 bytes), only to keep them sealed on this device (*Stay unlocked*,
    /// [`crate::device`]), so the app can sign in again without the password.
    pub fn to_bytes(&self) -> Zeroizing<Vec<u8>> {
        let mut out = Zeroizing::new(Vec::with_capacity(64));
        out.extend_from_slice(self.auth_key.as_ref());
        out.extend_from_slice(self.account_kek.as_ref());
        out
    }

    /// Reads [`AccountKeys::to_bytes`].
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        if bytes.len() != 64 {
            return Err(Error::InvalidKey);
        }
        Ok(AccountKeys { auth_key: util::secret_from(&bytes[..32])?, account_kek: util::secret_from(&bytes[32..])? })
    }

    /// Encrypts a short private text of this account (for example the name of a congregation it
    /// belongs to, which the server must not read). Only this account's password opens it.
    pub fn seal_label(&self, text: &str) -> Result<Vec<u8>, Error> {
        util::seal(&self.account_kek, b"theo/v1/label", text.as_bytes())
    }

    /// Opens what [`AccountKeys::seal_label`] made.
    pub fn open_label(&self, sealed: &[u8]) -> Result<String, Error> {
        let plain = util::open(&self.account_kek, b"theo/v1/label", sealed)?;
        String::from_utf8(plain.to_vec()).map_err(|_| Error::Damaged)
    }
}

/// A member's private keys. Wiped on drop.
pub struct PersonalKeys {
    kem_seed: Secret32,
    mldsa_seed: Secret32,
    ed_seed: Secret32,
}

impl std::fmt::Debug for PersonalKeys {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("PersonalKeys(..)")
    }
}

fn personal_aad(user_id: &str) -> Vec<u8> {
    let mut a = b"theo/v1/personal-keys".to_vec();
    util::put(&mut a, user_id.as_bytes());
    a
}

impl PersonalKeys {
    /// Fresh random keys for a new member.
    pub fn generate() -> Result<Self, Error> {
        Ok(PersonalKeys { kem_seed: util::random_secret()?, mldsa_seed: util::random_secret()?, ed_seed: util::random_secret()? })
    }

    /// The X-Wing private key (opens grants and the member's own sealed records).
    pub fn kem(&self) -> KemSecretKey {
        KemSecretKey::from_seed(&self.kem_seed)
    }

    /// The hybrid signing key.
    pub fn signer(&self) -> HybridSigner {
        HybridSigner::from_seeds(&self.mldsa_seed, &self.ed_seed)
    }

    /// The public keys to publish on the member's row.
    pub fn public(&self) -> PublicIdentity {
        PublicIdentity { kem: self.kem().public_key(), verifier: self.signer().verifier() }
    }

    /// The keys wrapped by the account kek, to store on the server.
    pub fn wrap(&self, account: &AccountKeys, user_id: &str) -> Result<Vec<u8>, Error> {
        let mut plain = Zeroizing::new(Vec::with_capacity(96));
        plain.extend_from_slice(self.kem_seed.as_ref());
        plain.extend_from_slice(self.mldsa_seed.as_ref());
        plain.extend_from_slice(self.ed_seed.as_ref());
        let mut out = MAGIC.to_vec();
        out.push(FORMAT);
        let mut aad = out.clone();
        aad.extend_from_slice(&personal_aad(user_id));
        out.extend_from_slice(&util::seal(&account.account_kek, &aad, &plain)?);
        Ok(out)
    }

    /// Opens what [`PersonalKeys::wrap`] made. A wrong password gives
    /// [`Error::WrongPasswordOrDamaged`].
    pub fn unwrap(account: &AccountKeys, user_id: &str, wrapped: &[u8]) -> Result<Self, Error> {
        let mut r = Reader::new(wrapped);
        if r.take(4)? != MAGIC {
            return Err(Error::NotAnEnvelope);
        }
        if r.u8()? != FORMAT {
            return Err(Error::UnknownFormat);
        }
        let mut aad = wrapped[..5].to_vec();
        aad.extend_from_slice(&personal_aad(user_id));
        let plain = util::open(&account.account_kek, &aad, r.rest()).map_err(|_| Error::WrongPasswordOrDamaged)?;
        if plain.len() != 96 {
            return Err(Error::Damaged);
        }
        Ok(PersonalKeys {
            kem_seed: util::secret_from(&plain[..32])?,
            mldsa_seed: util::secret_from(&plain[32..64])?,
            ed_seed: util::secret_from(&plain[64..])?,
        })
    }
}

/// A member's public keys.
#[derive(Clone, Debug, PartialEq)]
pub struct PublicIdentity {
    /// Grants and sealed records are encapsulated to it.
    pub kem: KemPublicKey,
    /// Checks the member's signatures.
    pub verifier: HybridVerifier,
}

impl PublicIdentity {
    /// The encoding: `X-Wing key (1216) | hybrid verifying key (1984)`.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = self.kem.to_bytes();
        out.extend_from_slice(&self.verifier.to_bytes());
        out
    }

    /// Reads [`PublicIdentity::to_bytes`].
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        if bytes.len() != crate::kem::PUBLIC_KEY_LEN + crate::sign::VERIFYING_KEY_LEN {
            return Err(Error::InvalidKey);
        }
        let (kem, verifier) = bytes.split_at(crate::kem::PUBLIC_KEY_LEN);
        Ok(PublicIdentity { kem: KemPublicKey::from_bytes(kem)?, verifier: HybridVerifier::from_bytes(verifier)? })
    }
}
