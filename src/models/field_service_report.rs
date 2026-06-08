use serde::{Deserialize, Serialize};
use surrealdb::types::{RecordId, SurrealValue};

use crate::crypto::{CryptoError, SessionCrypto};
use crate::database::Db;

pub const TABLE: &str = "field_service_report";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, SurrealValue)]
#[surreal(crate = "surrealdb::types")]
pub struct FieldServiceReport {
    pub id: Option<RecordId>,
    pub publisher: RecordId,   // plaintext FK
    pub year: i32,             // plaintext (calendar year)
    pub month: u8,             // 1-12, plaintext
    pub placements: Option<u32>,
    pub videos: Option<u32>,
    pub return_visits: Option<u32>,
    pub bible_studies: Option<u32>,
    pub hours: Option<f64>,
    pub auxiliary_pioneer: bool,
    pub notes: Option<String>, // encrypted
}

/// Payload for creating or updating a field service report.
#[derive(Debug, Clone, Serialize, Deserialize, SurrealValue)]
#[surreal(crate = "surrealdb::types")]
pub struct FieldServiceReportData {
    pub publisher: RecordId,
    pub year: i32,
    pub month: u8,
    pub placements: Option<u32>,
    pub videos: Option<u32>,
    pub return_visits: Option<u32>,
    pub bible_studies: Option<u32>,
    pub hours: Option<f64>,
    pub auxiliary_pioneer: bool,
    pub notes: Option<String>,
}

impl FieldServiceReportData {
    pub fn encrypt(mut self, crypto: &SessionCrypto) -> Result<Self, CryptoError> {
        self.notes = self.notes.map(|n| crypto.encrypt(&n)).transpose()?;
        Ok(self)
    }
}

impl FieldServiceReport {
    pub fn decrypt(mut self, crypto: &SessionCrypto) -> Result<Self, CryptoError> {
        self.notes = self.notes.map(|n| crypto.decrypt(&n)).transpose()?;
        Ok(self)
    }

    /// All reports for a given publisher, sorted newest first.
    pub async fn by_publisher(
        db: &Db,
        crypto: &SessionCrypto,
        publisher_id: RecordId,
    ) -> Result<Vec<Self>, Box<dyn std::error::Error>> {
        let rows: Vec<Self> = db
            .query("SELECT * FROM field_service_report WHERE publisher = $id")
            .bind(("id", publisher_id))
            .await?
            .take(0)?;
        let mut decrypted: Vec<Self> = rows
            .into_iter()
            .map(|r| r.decrypt(crypto).map_err(Into::into))
            .collect::<Result<_, Box<dyn std::error::Error>>>()?;
        decrypted.sort_by(|a, b| b.year.cmp(&a.year).then(b.month.cmp(&a.month)));
        Ok(decrypted)
    }

    pub async fn create(
        db: &Db,
        crypto: &SessionCrypto,
        data: FieldServiceReportData,
    ) -> Result<Option<Self>, Box<dyn std::error::Error>> {
        let row: Option<Self> = db.create(TABLE).content(data.encrypt(crypto)?).await?;
        row.map(|r| r.decrypt(crypto).map_err(Into::into)).transpose()
    }

    pub async fn update(
        db: &Db,
        crypto: &SessionCrypto,
        id: RecordId,
        data: FieldServiceReportData,
    ) -> Result<Option<Self>, Box<dyn std::error::Error>> {
        let row: Option<Self> = db.update(id).content(data.encrypt(crypto)?).await?;
        row.map(|r| r.decrypt(crypto).map_err(Into::into)).transpose()
    }

    pub async fn delete(db: &Db, id: RecordId) -> surrealdb::Result<Option<Self>> {
        db.delete(id).await
    }

    /// Delete all reports belonging to a publisher (e.g. when deleting the publisher).
    pub async fn delete_by_publisher(
        db: &Db,
        publisher_id: RecordId,
    ) -> Result<(), Box<dyn std::error::Error>> {
        db.query("DELETE field_service_report WHERE publisher = $id")
            .bind(("id", publisher_id))
            .await?;
        Ok(())
    }
}
