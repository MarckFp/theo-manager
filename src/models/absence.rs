use serde::{Deserialize, Serialize};
use surrealdb::types::{RecordId, SurrealValue};

use crate::crypto::{CryptoError, SessionCrypto};
use crate::database::Db;

pub const TABLE: &str = "absence";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, SurrealValue)]
#[surreal(crate = "surrealdb::types")]
pub struct Absence {
    pub id: Option<RecordId>,
    pub publisher: RecordId, // plaintext: used in DB-side queries
    /// ISO 8601 date string: `"2026-06-01"` — encrypted at rest
    pub start_date: String,
    pub end_date: Option<String>,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, SurrealValue)]
#[surreal(crate = "surrealdb::types")]
pub struct AbsenceData {
    pub publisher: RecordId,
    pub start_date: String,
    pub end_date: Option<String>,
    pub reason: Option<String>,
}

impl AbsenceData {
    pub fn encrypt(self, crypto: &SessionCrypto) -> Result<Self, CryptoError> {
        Ok(Self {
            publisher: self.publisher,
            start_date: crypto.encrypt(&self.start_date)?,
            end_date: self.end_date.map(|d| crypto.encrypt(&d)).transpose()?,
            reason: self.reason.map(|r| crypto.encrypt(&r)).transpose()?,
        })
    }
}

impl Absence {
    pub fn decrypt(self, crypto: &SessionCrypto) -> Result<Self, CryptoError> {
        Ok(Self {
            id: self.id,
            publisher: self.publisher,
            start_date: crypto.decrypt(&self.start_date)?,
            end_date: self.end_date.map(|d| crypto.decrypt(&d)).transpose()?,
            reason: self.reason.map(|r| crypto.decrypt(&r)).transpose()?,
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

    pub async fn by_publisher(
        db: &Db,
        crypto: &SessionCrypto,
        publisher_id: RecordId,
    ) -> Result<Vec<Self>, Box<dyn std::error::Error>> {
        let rows: Vec<Self> = db
            .query("SELECT * FROM absence WHERE publisher = $id ORDER BY start_date DESC")
            .bind(("id", publisher_id))
            .await?
            .take(0)?;
        // Note: ORDER BY start_date sorts on encrypted values (opaque order).
        // Sort in-memory by decrypted value when display order matters.
        let mut decrypted: Vec<Self> = rows
            .into_iter()
            .map(|r| r.decrypt(crypto).map_err(Into::into))
            .collect::<Result<_, Box<dyn std::error::Error>>>()?;
        decrypted.sort_by(|a, b| b.start_date.cmp(&a.start_date));
        Ok(decrypted)
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
        data: AbsenceData,
    ) -> Result<Option<Self>, Box<dyn std::error::Error>> {
        let row: Option<Self> = db.create(TABLE).content(data.encrypt(crypto)?).await?;
        row.map(|r| r.decrypt(crypto).map_err(Into::into))
            .transpose()
    }

    pub async fn update(
        db: &Db,
        crypto: &SessionCrypto,
        id: RecordId,
        data: AbsenceData,
    ) -> Result<Option<Self>, Box<dyn std::error::Error>> {
        let row: Option<Self> = db.update(id).content(data.encrypt(crypto)?).await?;
        row.map(|r| r.decrypt(crypto).map_err(Into::into))
            .transpose()
    }

    pub async fn delete(db: &Db, id: RecordId) -> surrealdb::Result<Option<Self>> {
        db.delete(id).await
    }
}
