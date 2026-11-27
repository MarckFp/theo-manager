//! Congregation export files (`.theobackup`): one file with all the data of a congregation,
//! always encrypted, with a password chosen when exporting (D-51). Importing replaces everything.
//! See `docs/design/local-sharing.md`, "Backup".
//!
//! ```text
//! file  = password envelope (theo-crypto: Argon2id + XChaCha20-Poly1305, "THBK")
//! inner = "THB2" | deflate(CBOR(Backup))        format 2 (small)
//!       | JSON(Backup)                          format 1 (older files, still read)
//! ```
//!
//! The file never holds the congregation password or keys: whoever imports it uses their own
//! congregation's keys. Decompression is limited, so a crafted file cannot exhaust memory.

use crate::people::Directory;
use crate::prefs::Preferences;
use crate::state::MeetingSlot;
use serde::{Deserialize, Serialize};
use theo_crypto::KdfParams;

/// File name extension of backups.
pub const EXTENSION: &str = "theobackup";
const FORMAT: u32 = 2;
/// The start of a format-2 payload (after decryption).
const MAGIC: &[u8; 4] = b"THB2";
/// The most a payload may decompress to (a congregation is a few MB at most).
const MAX_SIZE: usize = 256 * 1024 * 1024;

/// The congregation details that a backup restores (not the storage mode or the password).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Details {
    pub name: String,
    pub number: String,
    pub language: String,
    pub time_zone: String,
    pub midweek: MeetingSlot,
    pub weekend: MeetingSlot,
    pub defaults: Preferences,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Backup {
    pub format: u32,
    /// `YYYY-MM-DD HH:MM`, UTC.
    pub created: String,
    pub details: Details,
    pub directory: Directory,
}

impl Backup {
    pub fn new(details: Details, directory: Directory) -> Self {
        Backup { format: FORMAT, created: crate::prefs::now_stamp(), details, directory }
    }
}

/// Packs, compresses and encrypts a backup.
pub fn seal(backup: &Backup, password: &str, params: KdfParams) -> Result<Vec<u8>, String> {
    let mut cbor = Vec::new();
    ciborium::into_writer(backup, &mut cbor).map_err(|_| "The backup could not be written.".to_string())?;
    let mut payload = MAGIC.to_vec();
    payload.extend_from_slice(&miniz_oxide::deflate::compress_to_vec(&cbor, 9));
    theo_crypto::seal_with_password(password, &payload, params).map_err(|e| e.to_string())
}

/// Decrypts and reads a backup file (format 2, or an older format-1 file).
pub fn open(bytes: &[u8], password: &str) -> Result<Backup, String> {
    let plain = theo_crypto::open_with_password(password, bytes).map_err(|e| e.to_string())?;
    let unreadable = || "This backup could not be read.".to_string();
    let backup: Backup = match plain.strip_prefix(MAGIC.as_slice()) {
        Some(packed) => {
            let cbor = miniz_oxide::inflate::decompress_to_vec_with_limit(packed, MAX_SIZE).map_err(|_| unreadable())?;
            ciborium::from_reader(cbor.as_slice()).map_err(|_| unreadable())?
        }
        None => serde_json::from_slice(&plain).map_err(|_| unreadable())?,
    };
    if backup.format != FORMAT && backup.format != 1 {
        return Err("This backup was made by a newer version of Theo Manager. Update the app.".into());
    }
    Ok(backup)
}

#[cfg(test)]
mod tests {
    use super::*;

    const FAST: KdfParams = KdfParams { m_kib: 19 * 1024, t: 2, p: 1 };

    fn details() -> Details {
        Details {
            name: "Riverside".into(),
            number: String::new(),
            language: "en".into(),
            time_zone: "Europe/London".into(),
            midweek: MeetingSlot { day: "Thursday".into(), time: "19:00".into() },
            weekend: MeetingSlot { day: "Sunday".into(), time: "10:00".into() },
            defaults: Preferences::default(),
        }
    }

    #[test]
    fn exports_are_small_and_older_files_still_open() {
        let dir = crate::mock::demo_directory(36);
        let backup = Backup::new(details(), dir.clone());
        let sealed = seal(&backup, "pw", FAST).unwrap_or_default();
        let json = serde_json::to_vec(&backup).unwrap_or_default();
        assert!(sealed.len() * 3 < json.len(), "{} vs {}", sealed.len(), json.len());
        // A format-1 file (JSON) still opens.
        let mut old = backup.clone();
        old.format = 1;
        let old_file = theo_crypto::seal_with_password("pw", &serde_json::to_vec(&old).unwrap_or_default(), FAST).unwrap_or_default();
        assert!(open(&old_file, "pw").is_ok_and(|b| b.directory == dir));
        // A crafted payload that would inflate past the limit is refused, not unpacked.
        let mut bomb = MAGIC.to_vec();
        bomb.extend_from_slice(&miniz_oxide::deflate::compress_to_vec(&vec![0u8; MAX_SIZE + 1], 9));
        let bomb_file = theo_crypto::seal_with_password("pw", &bomb, FAST).unwrap_or_default();
        assert!(open(&bomb_file, "pw").is_err());
    }

    #[test]
    fn a_backup_restores_everything_with_the_right_password() {
        let dir = crate::mock::demo_directory(40);
        let sealed = seal(&Backup::new(details(), dir.clone()), "demo", FAST).unwrap_or_default();
        let back = open(&sealed, "demo");
        assert!(back.as_ref().is_ok_and(|b| b.directory == dir && b.details == details()));
        assert!(open(&sealed, "wrong password").is_err());
        assert!(open(b"not a backup", "demo").is_err());
    }
}
