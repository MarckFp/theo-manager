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
    /// When true the publisher submitted a "did not preach" report.
    /// Old records without this field deserialize as false (serde default).
    #[serde(default)]
    pub not_preached: bool,
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
    #[serde(default)]
    pub not_preached: bool,
    pub notes: Option<String>,
}

/// Minimal row for active-publisher queries (no decryption needed).
#[derive(Debug, serde::Deserialize, SurrealValue)]
#[surreal(crate = "surrealdb::types")]
struct PublisherOnlyRow {
    publisher: RecordId,
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

    /// Returns the set of publisher record-ID strings (e.g. `"user:abc123"`)
    /// that have at least one report since `(since_year, since_month)` inclusive
    /// where `not_preached` is not true (i.e. they actually preached).
    ///
    /// Existing records that pre-date the `not_preached` field are treated as
    /// preached (SurrealDB stores them without the field → != true).
    pub async fn active_publisher_ids(
        db: &Db,
        since_year: i32,
        since_month: u8,
    ) -> Result<std::collections::HashSet<String>, Box<dyn std::error::Error>> {
        let rows: Vec<PublisherOnlyRow> = db
            .query(
                "SELECT publisher FROM field_service_report \
                 WHERE (year > $sy OR (year = $sy AND month >= $sm)) \
                 AND not_preached != true",
            )
            .bind(("sy", since_year))
            .bind(("sm", since_month))
            .await?
            .take(0)?;
        let ids = rows
            .into_iter()
            .map(|r| {
                format!(
                    "{}:{}",
                    r.publisher.table,
                    match &r.publisher.key {
                        surrealdb::types::RecordIdKey::String(k) => k.clone(),
                        surrealdb::types::RecordIdKey::Number(n) => n.to_string(),
                        _ => String::new(),
                    }
                )
            })
            .collect();
        Ok(ids)
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
