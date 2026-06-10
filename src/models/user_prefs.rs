use serde::{Deserialize, Serialize};
use surrealdb::types::SurrealValue;

use crate::database::Db;

pub const TABLE: &str = "user_prefs";
pub const RECORD_KEY: &str = "prefs";

/// Flat data stored in `user_prefs:prefs`.
/// Fields mirror `UserPrefs` in `pages/app/user_settings.rs`.
/// Prefs are not sensitive, so no encryption is applied.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, SurrealValue)]
#[surreal(crate = "surrealdb::types")]
pub struct UserPrefsData {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub theme: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub accent_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub name_format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub date_format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub time_format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub language: Option<String>,
}

/// Fetch the stored user prefs from the database.
/// Returns `None` if no record exists yet.
pub async fn get(db: &Db) -> Result<Option<UserPrefsData>, Box<dyn std::error::Error>> {
    let mut res = db
        .query(format!("SELECT * FROM {}:{}", TABLE, RECORD_KEY))
        .await?;
    let record: Option<UserPrefsData> = res.take(0)?;
    Ok(record)
}

/// Create or fully replace the user prefs record.
pub async fn upsert(db: &Db, prefs: &UserPrefsData) -> Result<(), Box<dyn std::error::Error>> {
    db.query(format!(
        "UPSERT {}:{} CONTENT $data",
        TABLE, RECORD_KEY
    ))
    .bind(("data", prefs.clone()))
    .await?;
    Ok(())
}
