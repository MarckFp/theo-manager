//! Key domains and the keyring (6.2, R-316). Each domain (schedule, people, reports, territories,
//! …) has a 32-byte random secret per **key version**. Rotating a domain adds a new version; old
//! versions stay so old records can still be read.
//!
//! ```text
//! domain secret ──HKDF "theo/v1/record-key"───► record_key        (suite 0x01 envelopes)
//!               ──HKDF "theo/v1/blind-index"──► blind_index_key   (searchable equality tags)
//!               ──HKDF "theo/v1/domain-kem"───► X-Wing keypair    (sealed records, suite 0x02)
//! ```
//!
//! Every keyring entry is stored wrapped by the root key, one row per (domain, version):
//! `nonce (24) | ciphertext ‖ tag (48)`, bound to the congregation, domain and version.

use std::collections::BTreeMap;

use hkdf::Hkdf;
use sha2::Sha256;

use crate::kem::{KemPublicKey, KemSecretKey};
use crate::util::{self, Secret32};
use crate::vault::RootKey;
use crate::Error;

/// A key domain. The final list comes from the M-02 modeling session; ids are stored in every
/// envelope, so an id must never be reused for another domain.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DomainId(pub u8);

impl DomainId {
    /// Meetings, duties, public talks, public witnessing, calendar.
    pub const SCHEDULE: DomainId = DomainId(1);
    /// Contact details, addresses, privileges, notes.
    pub const PEOPLE: DomainId = DomainId(2);
    /// Field service reports, attendance.
    pub const REPORTS: DomainId = DomainId(3);
    /// Territory cards, boundaries, assignments.
    pub const TERRITORIES: DomainId = DomainId(4);
}

/// The secret of one domain at one key version. Wiped on drop.
#[derive(Clone)]
pub struct DomainSecret(pub(crate) Secret32);

impl std::fmt::Debug for DomainSecret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("DomainSecret(..)")
    }
}

impl DomainSecret {
    /// A fresh random secret.
    pub fn generate() -> Result<Self, Error> {
        util::random_secret().map(DomainSecret)
    }

    /// The keys derived from the secret.
    pub fn keys(&self) -> DomainKeys {
        let seed = util::hkdf32(self.0.as_ref(), "theo/v1/domain-kem");
        DomainKeys {
            record_key: util::hkdf32(self.0.as_ref(), "theo/v1/record-key"),
            blind_index_key: util::hkdf32(self.0.as_ref(), "theo/v1/blind-index"),
            kem: KemSecretKey::from_seed(&seed),
        }
    }
}

/// The working keys of one domain version.
#[derive(Debug)]
pub struct DomainKeys {
    pub(crate) record_key: Secret32,
    blind_index_key: Secret32,
    pub(crate) kem: KemSecretKey,
}

impl DomainKeys {
    /// The domain's public key: members who cannot read the domain seal records to it (R-327).
    pub fn public_key(&self) -> KemPublicKey {
        self.kem.public_key()
    }

    /// A blind index: a keyed hash (HMAC-SHA-256) of a value, so the server can match equal
    /// values (for example a normalized e-mail address) without learning them. `field` keeps the
    /// tags of different fields apart.
    pub fn blind_index(&self, field: &str, value: &[u8]) -> [u8; 32] {
        let mut msg = Vec::with_capacity(8 + field.len() + value.len());
        util::put(&mut msg, field.as_bytes());
        msg.extend_from_slice(value);
        // HKDF-Extract(salt = key, ikm = msg) is exactly HMAC-SHA-256(key, msg).
        let (prk, _) = Hkdf::<Sha256>::extract(Some(self.blind_index_key.as_ref()), &msg);
        let mut out = [0u8; 32];
        out.copy_from_slice(&prk);
        out
    }
}

/// Every domain secret this device has unlocked, by domain and version.
#[derive(Clone, Debug, Default)]
pub struct Keyring {
    entries: BTreeMap<(DomainId, u32), DomainSecret>,
}

fn entry_aad(congregation_id: &str, domain: DomainId, version: u32) -> Vec<u8> {
    let mut a = b"theo/v1/keyring".to_vec();
    util::put(&mut a, congregation_id.as_bytes());
    a.push(domain.0);
    a.extend_from_slice(&version.to_le_bytes());
    a
}

impl Keyring {
    /// An empty keyring.
    pub fn new() -> Self {
        Keyring::default()
    }

    /// Adds (or replaces) one secret.
    pub fn insert(&mut self, domain: DomainId, version: u32, secret: DomainSecret) {
        self.entries.insert((domain, version), secret);
    }

    /// The secret of one version.
    pub fn get(&self, domain: DomainId, version: u32) -> Option<&DomainSecret> {
        self.entries.get(&(domain, version))
    }

    /// The newest version of a domain: new records use it.
    pub fn current(&self, domain: DomainId) -> Option<(u32, &DomainSecret)> {
        self.entries.range((domain, 0)..=(domain, u32::MAX)).next_back().map(|((_, v), s)| (*v, s))
    }

    /// Adds a new random version of a domain (version 1 for a new domain) and returns it. Old
    /// versions stay (R-316).
    pub fn rotate(&mut self, domain: DomainId) -> Result<u32, Error> {
        let version = match self.current(domain) {
            Some((v, _)) => v.checked_add(1).ok_or(Error::InvalidKey)?,
            None => 1,
        };
        self.insert(domain, version, DomainSecret::generate()?);
        Ok(version)
    }

    /// Every (domain, version) held.
    pub fn versions(&self) -> impl Iterator<Item = (DomainId, u32)> + '_ {
        self.entries.keys().copied()
    }

    /// Drops every secret (lock, R-310).
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// One entry wrapped by the root key, to store as a keyring row.
    pub fn wrap_entry(&self, root: &RootKey, congregation_id: &str, domain: DomainId, version: u32) -> Result<Vec<u8>, Error> {
        let secret = self.get(domain, version).ok_or(Error::NoKey)?;
        util::seal(&root.derive("theo/v1/keyring"), &entry_aad(congregation_id, domain, version), secret.0.as_ref())
    }

    /// Unwraps a keyring row made by [`Keyring::wrap_entry`] and adds it.
    pub fn unwrap_entry(&mut self, root: &RootKey, congregation_id: &str, domain: DomainId, version: u32, wrapped: &[u8]) -> Result<(), Error> {
        let secret = util::open(&root.derive("theo/v1/keyring"), &entry_aad(congregation_id, domain, version), wrapped)?;
        self.insert(domain, version, DomainSecret(util::secret_from(&secret)?));
        Ok(())
    }
}
