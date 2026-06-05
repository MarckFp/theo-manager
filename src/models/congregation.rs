use serde::{Deserialize, Serialize};
use surrealdb::types::{RecordId, SurrealValue};

use crate::crypto::{CryptoError, SessionCrypto};
use crate::database::Db;

pub const TABLE: &str = "congregation";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, SurrealValue)]
#[surreal(crate = "surrealdb::types")]
pub enum TimeFormat {
    #[serde(rename = "12h")]
    H12,
    #[serde(rename = "24h")]
    H24,
}

impl Default for TimeFormat {
    fn default() -> Self {
        Self::H24
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, SurrealValue)]
#[surreal(crate = "surrealdb::types")]
pub enum DateFormat {
    #[serde(rename = "YYYY-MM-DD")]
    YMD,
    #[serde(rename = "DD-MM-YYYY")]
    DMY,
    #[serde(rename = "MM-DD-YYYY")]
    MDY,
}

impl Default for DateFormat {
    fn default() -> Self {
        Self::YMD
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, SurrealValue)]
#[surreal(crate = "surrealdb::types")]
pub enum NameFormat {
    #[serde(rename = "FirstLast")]
    FirstLast,
    #[serde(rename = "LastFirst")]
    LastFirst,
}

impl Default for NameFormat {
    fn default() -> Self {
        Self::FirstLast
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, SurrealValue)]
#[surreal(crate = "surrealdb::types")]
pub enum Theme {
    Light,
    Dark,
}

impl Default for Theme {
    fn default() -> Self {
        Self::Light
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, SurrealValue)]
#[surreal(crate = "surrealdb::types")]
pub struct Congregation {
    pub id: Option<RecordId>,
    /// Unique identifier for this congregation (UUID).
    /// Used as the SurrealDB namespace; stored in plaintext.
    pub uid: String,
    pub name: String,
    pub address: Option<String>,
    pub circuit: Option<String>,
    pub language: String,
    pub time_format: TimeFormat,
    pub date_format: DateFormat,
    pub name_format: NameFormat,
    pub theme: Theme,
}

/// Data required to create or update a congregation (no id).
#[derive(Debug, Clone, Serialize, Deserialize, SurrealValue)]
#[surreal(crate = "surrealdb::types")]
pub struct CongregationData {
    /// Unique identifier for this congregation (UUID). Plaintext.
    pub uid: String,
    pub name: String,
    pub address: Option<String>,
    pub circuit: Option<String>,
    pub language: String,
    #[serde(default)]
    pub time_format: TimeFormat,
    #[serde(default)]
    pub date_format: DateFormat,
    #[serde(default)]
    pub name_format: NameFormat,
    #[serde(default)]
    pub theme: Theme,
}

impl CongregationData {
    /// Encrypt all string fields before persisting to the database.
    pub fn encrypt(self, crypto: &SessionCrypto) -> Result<Self, CryptoError> {
        Ok(Self {
            uid: self.uid, // uid is plaintext — it's the namespace key
            name: crypto.encrypt(&self.name)?,
            address: self.address.map(|s| crypto.encrypt(&s)).transpose()?,
            circuit: self.circuit.map(|s| crypto.encrypt(&s)).transpose()?,
            language: crypto.encrypt(&self.language)?,
            time_format: self.time_format,
            date_format: self.date_format,
            name_format: self.name_format,
            theme: self.theme,
        })
    }
}

impl Congregation {
    /// Decrypt all string fields after reading from the database.
    pub fn decrypt(self, crypto: &SessionCrypto) -> Result<Self, CryptoError> {
        Ok(Self {
            id: self.id,
            uid: self.uid, // uid is plaintext
            name: crypto.decrypt(&self.name)?,
            address: self.address.map(|s| crypto.decrypt(&s)).transpose()?,
            circuit: self.circuit.map(|s| crypto.decrypt(&s)).transpose()?,
            language: crypto.decrypt(&self.language)?,
            time_format: self.time_format,
            date_format: self.date_format,
            name_format: self.name_format,
            theme: self.theme,
        })
    }

    pub async fn all(
        db: &Db,
        crypto: &SessionCrypto,
    ) -> Result<Vec<Self>, Box<dyn std::error::Error>> {
        let rows: Vec<Self> = db.select(TABLE).await?;
        rows.into_iter()
            .map(|r| r.decrypt(crypto).map_err(Into::into))
            .collect()
    }

    pub async fn get(
        db: &Db,
        crypto: &SessionCrypto,
        id: RecordId,
    ) -> Result<Option<Self>, Box<dyn std::error::Error>> {
        let row: Option<Self> = db.select(id).await?;
        row.map(|r| r.decrypt(crypto).map_err(Into::into))
            .transpose()
    }

    pub async fn create(
        db: &Db,
        crypto: &SessionCrypto,
        data: CongregationData,
    ) -> Result<Option<Self>, Box<dyn std::error::Error>> {
        let row: Option<Self> = db.create(TABLE).content(data.encrypt(crypto)?).await?;
        row.map(|r| r.decrypt(crypto).map_err(Into::into))
            .transpose()
    }

    pub async fn update(
        db: &Db,
        crypto: &SessionCrypto,
        id: RecordId,
        data: CongregationData,
    ) -> Result<Option<Self>, Box<dyn std::error::Error>> {
        let row: Option<Self> = db.update(id).content(data.encrypt(crypto)?).await?;
        row.map(|r| r.decrypt(crypto).map_err(Into::into))
            .transpose()
    }

    pub async fn delete(db: &Db, id: RecordId) -> surrealdb::Result<Option<Self>> {
        db.delete(id).await
    }
}
