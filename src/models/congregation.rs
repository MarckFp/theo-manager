use serde::{Deserialize, Serialize};
use surrealdb::types::{RecordId, SurrealValue};

use crate::crypto::{CryptoError, SessionCrypto};
use crate::database::Db;

pub const TABLE: &str = "congregation";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, SurrealValue)]
#[surreal(crate = "surrealdb::types")]
pub struct Congregation {
    pub id: Option<RecordId>,
    /// Unique identifier for this congregation (UUID).
    /// Used as the SurrealDB namespace; stored in plaintext.
    pub uid: String,
    pub name: String,
    pub city: String,
    pub circuit: String,
    pub language: String,
}

/// Data required to create or update a congregation (no id).
#[derive(Debug, Clone, Serialize, Deserialize, SurrealValue)]
#[surreal(crate = "surrealdb::types")]
pub struct CongregationData {
    /// Unique identifier for this congregation (UUID). Plaintext.
    pub uid: String,
    pub name: String,
    pub city: String,
    pub circuit: String,
    pub language: String,
}

impl CongregationData {
    /// Encrypt all string fields before persisting to the database.
    pub fn encrypt(self, crypto: &SessionCrypto) -> Result<Self, CryptoError> {
        Ok(Self {
            uid: self.uid, // uid is plaintext — it's the namespace key
            name: crypto.encrypt(&self.name)?,
            city: crypto.encrypt(&self.city)?,
            circuit: crypto.encrypt(&self.circuit)?,
            language: crypto.encrypt(&self.language)?,
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
            city: crypto.decrypt(&self.city)?,
            circuit: crypto.decrypt(&self.circuit)?,
            language: crypto.decrypt(&self.language)?,
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
