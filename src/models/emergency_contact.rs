use serde::{Deserialize, Serialize};
use surrealdb::types::{RecordId, SurrealValue};

use crate::crypto::{CryptoError, SessionCrypto};
use crate::database::Db;

pub const TABLE: &str = "emergency_contact";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, SurrealValue)]
#[surreal(crate = "surrealdb::types")]
pub struct EmergencyContact {
    pub id: Option<RecordId>,
    pub publisher: RecordId, // plaintext — foreign key to publisher table
    // ── Encrypted fields ─────────────────────────────────────────────────
    pub first_name: String,
    pub last_name: String,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub address: Option<String>,
    pub relationship: Option<String>,
}

/// Payload for creating or updating an emergency contact.
#[derive(Debug, Clone, Serialize, Deserialize, SurrealValue)]
#[surreal(crate = "surrealdb::types")]
pub struct EmergencyContactData {
    pub publisher: RecordId,
    pub first_name: String,
    pub last_name: String,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub address: Option<String>,
    pub relationship: Option<String>,
}

impl EmergencyContactData {
    pub fn encrypt(self, crypto: &SessionCrypto) -> Result<Self, CryptoError> {
        Ok(Self {
            publisher: self.publisher,
            first_name: crypto.encrypt(&self.first_name)?,
            last_name: crypto.encrypt(&self.last_name)?,
            phone: self.phone.map(|s| crypto.encrypt(&s)).transpose()?,
            email: self.email.map(|s| crypto.encrypt(&s)).transpose()?,
            address: self.address.map(|s| crypto.encrypt(&s)).transpose()?,
            relationship: self.relationship.map(|s| crypto.encrypt(&s)).transpose()?,
        })
    }
}

impl EmergencyContact {
    pub fn decrypt(self, crypto: &SessionCrypto) -> Result<Self, CryptoError> {
        Ok(Self {
            id: self.id,
            publisher: self.publisher,
            first_name: crypto.decrypt(&self.first_name)?,
            last_name: crypto.decrypt(&self.last_name)?,
            phone: self.phone.map(|s| crypto.decrypt(&s)).transpose()?,
            email: self.email.map(|s| crypto.decrypt(&s)).transpose()?,
            address: self.address.map(|s| crypto.decrypt(&s)).transpose()?,
            relationship: self.relationship.map(|s| crypto.decrypt(&s)).transpose()?,
        })
    }

    /// All emergency contacts for a given publisher.
    pub async fn by_publisher(
        db: &Db,
        crypto: &SessionCrypto,
        publisher_id: RecordId,
    ) -> Result<Vec<Self>, Box<dyn std::error::Error>> {
        let rows: Vec<Self> = db
            .query("SELECT * FROM emergency_contact WHERE publisher = $id")
            .bind(("id", publisher_id))
            .await?
            .take(0)?;
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
        data: EmergencyContactData,
    ) -> Result<Option<Self>, Box<dyn std::error::Error>> {
        let row: Option<Self> = db.create(TABLE).content(data.encrypt(crypto)?).await?;
        row.map(|r| r.decrypt(crypto).map_err(Into::into))
            .transpose()
    }

    pub async fn update(
        db: &Db,
        crypto: &SessionCrypto,
        id: RecordId,
        data: EmergencyContactData,
    ) -> Result<Option<Self>, Box<dyn std::error::Error>> {
        let row: Option<Self> = db.update(id).content(data.encrypt(crypto)?).await?;
        row.map(|r| r.decrypt(crypto).map_err(Into::into))
            .transpose()
    }

    pub async fn delete(db: &Db, id: RecordId) -> surrealdb::Result<Option<Self>> {
        db.delete(id).await
    }

    /// Delete all emergency contacts belonging to a publisher (e.g. when deleting the publisher).
    pub async fn delete_by_publisher(
        db: &Db,
        publisher_id: RecordId,
    ) -> Result<(), Box<dyn std::error::Error>> {
        db.query("DELETE emergency_contact WHERE publisher = $id")
            .bind(("id", publisher_id))
            .await?;
        Ok(())
    }
}
