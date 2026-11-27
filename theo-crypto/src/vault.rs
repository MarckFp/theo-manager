//! The congregation vault (6.2): the congregation password unlocks the **root key**, and the root
//! key unlocks every key domain.
//!
//! ```text
//! congregation password ──Argon2id(salt_c)──► congregation_kek ──unwraps──► root_key (32 B random)
//! ```
//!
//! The vault header is what the server stores (`vault_header`). It never contains the password or
//! the root key in clear. All integers are little-endian:
//!
//! ```text
//! magic "THVH" (4) | format (1) = 1 | argon2 m_kib (4) | t (4) | p (4) | salt_c (16)
//! | wrapped root: nonce (24) | ciphertext ‖ tag (48)
//! | recovery (1) = 0 or 1 | [1: root wrapped by the recovery key: nonce (24) | ciphertext ‖ tag (48)]
//! ```
//!
//! The wrapped root key is bound to the congregation id and to the Argon2 parameters and salt, so
//! the server cannot swap headers between congregations or lower the cost (R-302).
//!
//! - **Changing the password** re-wraps the root key only; every domain and record stays as it is.
//! - **Recovery kit** (R-311): an optional 32-byte random key that wraps the root key a second time.

use crate::util::{self, Reader, Secret32};
use zeroize::Zeroizing;
use crate::{Error, KdfParams};

const MAGIC: &[u8; 4] = b"THVH";
const FORMAT: u8 = 1;
const SALT_LEN: usize = 16;
const WRAPPED_LEN: usize = util::NONCE_LEN + 32 + util::TAG_LEN;

/// The congregation root key. It is held in memory only while the congregation is unlocked, and is
/// wiped on drop (R-309).
pub struct RootKey(pub(crate) Secret32);

impl std::fmt::Debug for RootKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("RootKey(..)")
    }
}

/// The optional recovery key (R-311). Shown to the admin once; only they keep it.
pub struct RecoveryKey(Secret32);

impl std::fmt::Debug for RecoveryKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("RecoveryKey(..)")
    }
}

impl RecoveryKey {
    /// The 32 bytes to show to the admin (for example as words or a QR code).
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// A recovery key the admin typed or scanned.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        util::secret_from(bytes).map(RecoveryKey).map_err(|_| Error::InvalidKey)
    }
}

/// The stored vault header.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Vault {
    params: KdfParams,
    salt: [u8; SALT_LEN],
    wrapped_root: Vec<u8>,
    recovery: Option<Vec<u8>>,
}

fn aad(congregation_id: &str, params: KdfParams, salt: &[u8; SALT_LEN]) -> Vec<u8> {
    let mut a = b"theo/v1/vault".to_vec();
    util::put(&mut a, congregation_id.as_bytes());
    a.extend_from_slice(&params.m_kib.to_le_bytes());
    a.extend_from_slice(&params.t.to_le_bytes());
    a.extend_from_slice(&params.p.to_le_bytes());
    a.extend_from_slice(salt);
    a
}

fn recovery_kek(recovery: &RecoveryKey) -> Secret32 {
    util::hkdf32(recovery.0.as_ref(), "theo/v1/recovery-kek")
}

impl Vault {
    /// A new congregation: a fresh random root key, wrapped with the password.
    pub fn create(congregation_id: &str, password: &str, params: KdfParams) -> Result<(Vault, RootKey), Error> {
        let root = RootKey(util::random_secret()?);
        let vault = Vault::wrap(congregation_id, &root, password, params, None)?;
        Ok((vault, root))
    }

    fn wrap(congregation_id: &str, root: &RootKey, password: &str, params: KdfParams, recovery: Option<Vec<u8>>) -> Result<Vault, Error> {
        params.check()?;
        let salt = util::random::<SALT_LEN>()?;
        let kek = crate::derive_key(password, &salt, params)?;
        let wrapped_root = util::seal(&kek, &aad(congregation_id, params, &salt), root.0.as_ref())?;
        Ok(Vault { params, salt, wrapped_root, recovery })
    }

    /// Unlocks the root key with the congregation password.
    pub fn unlock(&self, congregation_id: &str, password: &str) -> Result<RootKey, Error> {
        self.params.check()?;
        let kek = crate::derive_key(password, &self.salt, self.params)?;
        let root = util::open(&kek, &aad(congregation_id, self.params, &self.salt), &self.wrapped_root)
            .map_err(|_| Error::WrongPasswordOrDamaged)?;
        util::secret_from(&root).map(RootKey)
    }

    /// A new header for a new password (and maybe new Argon2 parameters). Needs the unlocked root
    /// key; the root key itself does not change, so no domain or record changes. A recovery key
    /// keeps working.
    pub fn change_password(&self, congregation_id: &str, root: &RootKey, new_password: &str, params: KdfParams) -> Result<Vault, Error> {
        // The recovery entry is bound only to the congregation (not to the salt), so it is kept.
        Vault::wrap(congregation_id, root, new_password, params, self.recovery.clone())
    }

    /// Turns on the recovery kit: returns the recovery key to show once.
    pub fn add_recovery(&mut self, congregation_id: &str, root: &RootKey) -> Result<RecoveryKey, Error> {
        let recovery = RecoveryKey(util::random_secret()?);
        self.recovery = Some(util::seal(&recovery_kek(&recovery), &recovery_aad(congregation_id), root.0.as_ref())?);
        Ok(recovery)
    }

    /// Turns the recovery kit off. The old recovery key no longer works with this header.
    pub fn remove_recovery(&mut self) {
        self.recovery = None;
    }

    /// Is a recovery kit set?
    pub fn has_recovery(&self) -> bool {
        self.recovery.is_some()
    }

    /// Unlocks the root key with the recovery key (when the password is lost).
    pub fn unlock_with_recovery(&self, congregation_id: &str, recovery: &RecoveryKey) -> Result<RootKey, Error> {
        let wrapped = self.recovery.as_ref().ok_or(Error::NoKey)?;
        let root = util::open(&recovery_kek(recovery), &recovery_aad(congregation_id), wrapped).map_err(|_| Error::WrongPasswordOrDamaged)?;
        util::secret_from(&root).map(RootKey)
    }

    /// The Argon2 parameters in use.
    pub fn params(&self) -> KdfParams {
        self.params
    }

    /// The header bytes to store.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = MAGIC.to_vec();
        out.push(FORMAT);
        out.extend_from_slice(&self.params.m_kib.to_le_bytes());
        out.extend_from_slice(&self.params.t.to_le_bytes());
        out.extend_from_slice(&self.params.p.to_le_bytes());
        out.extend_from_slice(&self.salt);
        out.extend_from_slice(&self.wrapped_root);
        match &self.recovery {
            Some(r) => {
                out.push(1);
                out.extend_from_slice(r);
            }
            None => out.push(0),
        }
        out
    }

    /// Reads [`Vault::to_bytes`]. Unknown formats and too-weak parameters are refused.
    pub fn from_bytes(bytes: &[u8]) -> Result<Vault, Error> {
        let mut r = Reader::new(bytes);
        if r.take(4)? != MAGIC {
            return Err(Error::NotAnEnvelope);
        }
        if r.u8()? != FORMAT {
            return Err(Error::UnknownFormat);
        }
        let params = KdfParams { m_kib: r.u32()?, t: r.u32()?, p: r.u32()? };
        params.check()?;
        let mut salt = [0u8; SALT_LEN];
        salt.copy_from_slice(r.take(SALT_LEN)?);
        let wrapped_root = r.take(WRAPPED_LEN)?.to_vec();
        let recovery = match r.u8()? {
            0 => None,
            1 => Some(r.take(WRAPPED_LEN)?.to_vec()),
            _ => return Err(Error::NotAnEnvelope),
        };
        if !r.rest().is_empty() {
            return Err(Error::NotAnEnvelope);
        }
        Ok(Vault { params, salt, wrapped_root, recovery })
    }
}

fn recovery_aad(congregation_id: &str) -> Vec<u8> {
    let mut a = b"theo/v1/recovery".to_vec();
    util::put(&mut a, congregation_id.as_bytes());
    a
}

impl RootKey {
    /// The key's bytes, only to keep it sealed on this device (*Stay unlocked*, passkey unlock:
    /// [`crate::device`]). Never send or store them unsealed.
    pub fn to_bytes(&self) -> Zeroizing<[u8; 32]> {
        Zeroizing::new(*self.0)
    }

    /// Reads [`RootKey::to_bytes`].
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        util::secret_from(bytes).map(RootKey).map_err(|_| Error::InvalidKey)
    }

    /// A key for one purpose, derived from the root key.
    pub(crate) fn derive(&self, label: &str) -> Secret32 {
        util::hkdf32(self.0.as_ref(), label)
    }

    /// The stored id of an item: the first 16 bytes of HMAC-SHA-256 under a key derived from the
    /// root key, over the collection and the item's own key. It looks random (it shows neither
    /// the order nor the number of items), stays the same on every device and every save, and
    /// does not change with the password, because the root key does not.
    pub fn record_id(&self, collection: &str, key: &str) -> [u8; 16] {
        let mut msg = Vec::with_capacity(8 + collection.len() + key.len());
        util::put(&mut msg, collection.as_bytes());
        msg.extend_from_slice(key.as_bytes());
        let k = self.derive("theo/v1/record-id");
        // HKDF-Extract(salt = key, ikm = msg) is exactly HMAC-SHA-256(key, msg).
        let (mac, _) = hkdf::Hkdf::<sha2::Sha256>::extract(Some(k.as_ref()), &msg);
        let mut id = [0u8; 16];
        id.copy_from_slice(&mac[..16]);
        id
    }
}
