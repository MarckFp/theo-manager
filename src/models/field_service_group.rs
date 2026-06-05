use serde::{Deserialize, Serialize};
use surrealdb::types::{RecordId, SurrealValue};

use crate::crypto::{CryptoError, SessionCrypto};
use crate::database::Db;

pub const TABLE: &str = "field_service_group";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, SurrealValue)]
#[surreal(crate = "surrealdb::types")]
pub struct FieldServiceGroup {
    pub id: Option<RecordId>,
    pub congregation: RecordId, // plaintext FK
    // ── Encrypted ────────────────────────────────────────────────────────
    pub name: String,
    // ── Plaintext FKs ────────────────────────────────────────────────────
    /// Must be a `Gender::Male` publisher — enforced at the application layer.
    pub overseer: Option<RecordId>,
    /// Must be a `Gender::Male` publisher — enforced at the application layer.
    pub assistant: Option<RecordId>,
    /// Each publisher may belong to at most one group — enforced at the application layer.
    pub members: Vec<RecordId>,
}

/// Payload for creating or updating a field service group.
#[derive(Debug, Clone, Serialize, Deserialize, SurrealValue)]
#[surreal(crate = "surrealdb::types")]
pub struct FieldServiceGroupData {
    pub congregation: RecordId,
    pub name: String,
    pub overseer: Option<RecordId>,
    pub assistant: Option<RecordId>,
    pub members: Vec<RecordId>,
}

impl FieldServiceGroupData {
    pub fn encrypt(self, crypto: &SessionCrypto) -> Result<Self, CryptoError> {
        Ok(Self {
            congregation: self.congregation,
            name: crypto.encrypt(&self.name)?,
            overseer: self.overseer,
            assistant: self.assistant,
            members: self.members,
        })
    }
}

impl FieldServiceGroup {
    pub fn decrypt(self, crypto: &SessionCrypto) -> Result<Self, CryptoError> {
        Ok(Self {
            id: self.id,
            congregation: self.congregation,
            name: crypto.decrypt(&self.name)?,
            overseer: self.overseer,
            assistant: self.assistant,
            members: self.members,
        })
    }

    /// All groups for a congregation.
    pub async fn all(
        db: &Db,
        crypto: &SessionCrypto,
    ) -> Result<Vec<Self>, Box<dyn std::error::Error>> {
        let rows: Vec<Self> = db.select(TABLE).await?;
        rows.into_iter()
            .map(|r| r.decrypt(crypto).map_err(Into::into))
            .collect()
    }

    pub async fn by_congregation(
        db: &Db,
        crypto: &SessionCrypto,
        congregation_id: RecordId,
    ) -> Result<Vec<Self>, Box<dyn std::error::Error>> {
        let rows: Vec<Self> = db
            .query("SELECT * FROM field_service_group WHERE congregation = $id")
            .bind(("id", congregation_id))
            .await?
            .take(0)?;
        rows.into_iter()
            .map(|r| r.decrypt(crypto).map_err(Into::into))
            .collect()
    }

    /// The group a specific publisher belongs to (at most one).
    pub async fn of_publisher(
        db: &Db,
        crypto: &SessionCrypto,
        publisher_id: RecordId,
    ) -> Result<Option<Self>, Box<dyn std::error::Error>> {
        let mut rows: Vec<Self> = db
            .query("SELECT * FROM field_service_group WHERE members CONTAINS $id LIMIT 1")
            .bind(("id", publisher_id))
            .await?
            .take(0)?;
        rows.pop()
            .map(|r| r.decrypt(crypto).map_err(Into::into))
            .transpose()
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
        data: FieldServiceGroupData,
    ) -> Result<Option<Self>, Box<dyn std::error::Error>> {
        let row: Option<Self> = db.create(TABLE).content(data.encrypt(crypto)?).await?;
        row.map(|r| r.decrypt(crypto).map_err(Into::into))
            .transpose()
    }

    pub async fn update(
        db: &Db,
        crypto: &SessionCrypto,
        id: RecordId,
        data: FieldServiceGroupData,
    ) -> Result<Option<Self>, Box<dyn std::error::Error>> {
        let row: Option<Self> = db.update(id).content(data.encrypt(crypto)?).await?;
        row.map(|r| r.decrypt(crypto).map_err(Into::into))
            .transpose()
    }

    /// Add a publisher to this group.
    /// Call `of_publisher` first to verify they are not already in another group.
    pub async fn add_member(
        db: &Db,
        group_id: RecordId,
        publisher_id: RecordId,
    ) -> Result<(), Box<dyn std::error::Error>> {
        db.query(
            "UPDATE $group_id SET members += [$publisher_id] WHERE NOT members CONTAINS $publisher_id",
        )
        .bind(("group_id", group_id))
        .bind(("publisher_id", publisher_id))
        .await?;
        Ok(())
    }

    /// Remove a publisher from this group.
    pub async fn remove_member(
        db: &Db,
        group_id: RecordId,
        publisher_id: RecordId,
    ) -> Result<(), Box<dyn std::error::Error>> {
        db.query("UPDATE $group_id SET members -= [$publisher_id]")
            .bind(("group_id", group_id))
            .bind(("publisher_id", publisher_id))
            .await?;
        Ok(())
    }

    pub async fn delete(db: &Db, id: RecordId) -> surrealdb::Result<Option<Self>> {
        db.delete(id).await
    }
}
