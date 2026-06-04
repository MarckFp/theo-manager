use serde::{Deserialize, Serialize};
use surrealdb::types::{RecordId, SurrealValue};

use crate::crypto::{CryptoError, SessionCrypto};
use crate::database::Db;

pub const TABLE: &str = "user";

/// Service type. Defaults to [`UserType::Student`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, SurrealValue)]
#[surreal(crate = "surrealdb::types")]
pub enum UserType {
    Student,
    Publisher,
    BaptizedPublisher,
    ContinuousAuxiliaryPioneer,
    RegularPioneer,
    SpecialPioneer,
    Missionary,
}

impl Default for UserType {
    fn default() -> Self {
        Self::Student
    }
}

/// Congregation appointment — only applicable to [`Gender::Male`] publishers.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, SurrealValue)]
#[surreal(crate = "surrealdb::types")]
pub enum Appointment {
    Elder,
    MinisterialServant,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, SurrealValue)]
#[surreal(crate = "surrealdb::types")]
pub enum Gender {
    Male,
    Female,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, SurrealValue)]
#[surreal(crate = "surrealdb::types")]
pub struct User {
    pub id: Option<RecordId>,
    // ── Encrypted string fields ───────────────────────────────────────────
    pub first_name: String,
    pub last_name: String,
    pub birthday: Option<String>,     // ISO 8601 date, encrypted
    pub baptism_date: Option<String>, // ISO 8601 date, encrypted
    pub phone: Option<String>,
    pub address: Option<String>,
    pub email: Option<String>,
    pub password: Option<String>, // app-level PIN / passphrase
    // ── Plaintext fields ──────────────────────────────────────────────────
    pub user_type: UserType,
    pub gender: Gender,
    pub appointment: Option<Appointment>, // only valid for Gender::Male
    pub family_head: bool,
    pub congregations: Vec<RecordId>, // one or many linked congregations
    pub active: bool,
}

/// Payload for creating or updating a publisher.
#[derive(Debug, Clone, Serialize, Deserialize, SurrealValue)]
#[surreal(crate = "surrealdb::types")]
pub struct UserData {
    pub first_name: String,
    pub last_name: String,
    pub birthday: Option<String>,
    pub baptism_date: Option<String>,
    pub phone: Option<String>,
    pub address: Option<String>,
    pub email: Option<String>,
    pub password: Option<String>,
    #[serde(default)]
    pub user_type: UserType,
    pub gender: Gender,
    pub appointment: Option<Appointment>,
    #[serde(default)]
    pub family_head: bool,
    pub congregations: Vec<RecordId>,
    #[serde(default = "default_active")]
    pub active: bool,
}

fn default_active() -> bool {
    true
}

impl UserData {
    pub fn encrypt(self, crypto: &SessionCrypto) -> Result<Self, CryptoError> {
        Ok(Self {
            first_name: crypto.encrypt(&self.first_name)?,
            last_name: crypto.encrypt(&self.last_name)?,
            birthday: self.birthday.map(|s| crypto.encrypt(&s)).transpose()?,
            baptism_date: self.baptism_date.map(|s| crypto.encrypt(&s)).transpose()?,
            phone: self.phone.map(|s| crypto.encrypt(&s)).transpose()?,
            address: self.address.map(|s| crypto.encrypt(&s)).transpose()?,
            email: self.email.map(|s| crypto.encrypt(&s)).transpose()?,
            password: self.password.map(|s| crypto.encrypt(&s)).transpose()?,
            user_type: self.user_type,
            gender: self.gender,
            appointment: self.appointment,
            family_head: self.family_head,
            congregations: self.congregations,
            active: self.active,
        })
    }
}

impl User {
    pub fn decrypt(self, crypto: &SessionCrypto) -> Result<Self, CryptoError> {
        Ok(Self {
            id: self.id,
            first_name: crypto.decrypt(&self.first_name)?,
            last_name: crypto.decrypt(&self.last_name)?,
            birthday: self.birthday.map(|s| crypto.decrypt(&s)).transpose()?,
            baptism_date: self.baptism_date.map(|s| crypto.decrypt(&s)).transpose()?,
            phone: self.phone.map(|s| crypto.decrypt(&s)).transpose()?,
            address: self.address.map(|s| crypto.decrypt(&s)).transpose()?,
            email: self.email.map(|s| crypto.decrypt(&s)).transpose()?,
            password: self.password.map(|s| crypto.decrypt(&s)).transpose()?,
            user_type: self.user_type,
            gender: self.gender,
            appointment: self.appointment,
            family_head: self.family_head,
            congregations: self.congregations,
            active: self.active,
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

    pub async fn by_congregation(
        db: &Db,
        crypto: &SessionCrypto,
        congregation_id: RecordId,
    ) -> Result<Vec<Self>, Box<dyn std::error::Error>> {
        let rows: Vec<Self> = db
            .query("SELECT * FROM user WHERE congregations CONTAINS $id AND active = true")
            .bind(("id", congregation_id))
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
        mut data: UserData,
    ) -> Result<Option<Self>, Box<dyn std::error::Error>> {
        data.active = true;
        let row: Option<Self> = db.create(TABLE).content(data.encrypt(crypto)?).await?;
        row.map(|r| r.decrypt(crypto).map_err(Into::into))
            .transpose()
    }

    pub async fn update(
        db: &Db,
        crypto: &SessionCrypto,
        id: RecordId,
        data: UserData,
    ) -> Result<Option<Self>, Box<dyn std::error::Error>> {
        let row: Option<Self> = db.update(id).content(data.encrypt(crypto)?).await?;
        row.map(|r| r.decrypt(crypto).map_err(Into::into))
            .transpose()
    }

    /// Soft-delete: marks the publisher as inactive instead of removing the record.
    pub async fn deactivate(db: &Db, id: RecordId) -> surrealdb::Result<Option<Self>> {
        db.update(id)
            .merge(serde_json::json!({ "active": false }))
            .await
    }

    pub async fn delete(db: &Db, id: RecordId) -> surrealdb::Result<Option<Self>> {
        db.delete(id).await
    }
}
