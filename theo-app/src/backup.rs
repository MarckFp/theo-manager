//! Congregation backups: one file with all the data of a congregation, encrypted with the
//! congregation password (`theo-crypto`). Restoring replaces everything on this device.
//! See `docs/design/local-sharing.md`, "Backup".
//!
//! ⚠️ The payload is the in-memory scaffold data (`Directory`). When `theo-core` and `theo-db`
//! arrive, the payload becomes the encrypted records; the envelope stays the same.

use crate::people::Directory;
use crate::prefs::Preferences;
use crate::state::MeetingSlot;
use serde::{Deserialize, Serialize};
use theo_crypto::KdfParams;

/// File name extension of backups.
pub const EXTENSION: &str = "theobackup";
const FORMAT: u32 = 1;

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

/// Serialises and encrypts a backup.
pub fn seal(backup: &Backup, password: &str, params: KdfParams) -> Result<Vec<u8>, String> {
    let json = serde_json::to_vec(backup).map_err(|_| "The backup could not be written.".to_string())?;
    theo_crypto::seal_with_password(password, &json, params).map_err(|e| e.to_string())
}

/// Decrypts and reads a backup file.
pub fn open(bytes: &[u8], password: &str) -> Result<Backup, String> {
    let json = theo_crypto::open_with_password(password, bytes).map_err(|e| e.to_string())?;
    let backup: Backup = serde_json::from_slice(&json).map_err(|_| "This backup could not be read.".to_string())?;
    if backup.format != FORMAT {
        return Err("This backup was made by another version of Theo Manager.".into());
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
    fn a_backup_restores_everything_with_the_right_password() {
        let dir = crate::mock::demo_directory(40);
        let sealed = seal(&Backup::new(details(), dir.clone()), "demo", FAST).unwrap_or_default();
        let back = open(&sealed, "demo");
        assert!(back.as_ref().is_ok_and(|b| b.directory == dir && b.details == details()));
        assert!(open(&sealed, "wrong password").is_err());
        assert!(open(b"not a backup", "demo").is_err());
    }
}
