//! What this device knows **before** anything is unlocked (stored with `platform::device_set`):
//! the accounts made on this device and the Local congregations stored on it, with their names.
//! Everything else about a congregation is encrypted in its own database (`theo-db`), one per
//! congregation: IndexedDB in the browser, a SQLite file on desktop and phone.
//!
//! A device account keeps no password: only its **auth key**, Argon2id of the password with a
//! salt made from the username (`theo_crypto::account_salt`), the same derivation a Cloud sign-in
//! uses (6.2). It opens the account's list; each congregation still needs its own password.

use serde::{Deserialize, Serialize};
use theo_crypto::{account_salt, AccountKeys, KdfParams};

const KEY: &str = "theo-manager.local";

/// An account made on this device.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LocalAccount {
    pub username: String,
    pub display_name: String,
    /// Hex of the auth key.
    pub auth: String,
}

/// A Local congregation stored on this device.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LocalCongregation {
    /// 32 hex digits (128-bit random).
    pub id: String,
    pub name: String,
    /// The username of the device account it belongs to.
    pub owner: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Registry {
    pub accounts: Vec<LocalAccount>,
    pub congregations: Vec<LocalCongregation>,
}

impl Registry {
    pub fn load() -> Self {
        crate::platform::device_get(KEY).and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default()
    }

    pub fn save(&self) {
        if let Ok(text) = serde_json::to_string(self) {
            crate::platform::device_set(KEY, Some(&text));
        }
    }

    /// Changes the registry and saves it.
    pub fn update(f: impl FnOnce(&mut Registry)) {
        let mut r = Registry::load();
        f(&mut r);
        r.save();
    }
}

/// The auth key of a device account, as hex (slow on purpose: Argon2id).
pub fn auth_hex(username: &str, password: &str) -> Option<String> {
    let salt = account_salt("device", "theo", "local", username);
    let keys = AccountKeys::derive(password, &salt, KdfParams::DEFAULT).ok()?;
    Some(to_hex(keys.auth_key()))
}

/// Bytes as lowercase hex.
pub fn to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Reads [`to_hex`].
pub fn from_hex(s: &str) -> Option<Vec<u8>> {
    if !s.len().is_multiple_of(2) || !s.is_ascii() {
        return None;
    }
    (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).ok()).collect()
}

/// The keys of a Cloud account on `server`: Argon2id of the password with the account salt of
/// that server (6.2). The auth key signs in; the other key encrypts the account's own labels.
pub fn cloud_keys(server: &theo_db::surreal::Server, username: &str, password: &str) -> Option<AccountKeys> {
    let salt = account_salt(&server.origin(), &server.namespace, &server.database, username);
    AccountKeys::derive(password, &salt, KdfParams::DEFAULT).ok()
}

/// Where a Local congregation's database is.
pub fn db_config(id: &str) -> theo_db::DbConfig {
    #[cfg(target_arch = "wasm32")]
    {
        theo_db::DbConfig::IndexedDb { name: format!("theo-manager-{id}") }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        theo_db::DbConfig::Sqlite { path: crate::platform::data_dir().join("congregations").join(format!("{id}.sqlite")) }
    }
}
