use serde::{Deserialize, Serialize};
use surrealdb::sql::Thing;
use surrealdb::Surreal;
use surrealdb::engine::any::Any;
use crate::database::db::get_db;
use chrono::NaiveDate;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EmergencyContact {
    pub firstname: String,
    pub lastname: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub address: Option<String>,
    pub notes: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum UserType {
    Student,
    UnbaptizedPublisher,
    BaptizedPublisher,
    RegularPioneer,
    SpecialPioneer,
    ContiniousAuxiliaryPioneer,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Appointment {
    Elder,
    MinisterialServant,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct User {
    pub id: surrealdb::RecordId,
    pub firstname: String,
    pub lastname: String,
    pub gender: bool,
    #[serde(default)]
    pub family_head: bool,
    pub email: Option<String>,
    pub password: Option<String>,
    pub birthday: Option<NaiveDate>,
    pub phone: Option<String>,
    pub address: Option<String>,
    pub city: Option<String>,
    pub country: Option<String>,
    pub zipcode: Option<String>,
    pub baptism_date: Option<NaiveDate>,
    pub anointed: Option<bool>,
    pub publisher_type: Option<UserType>,
    pub appointment: Option<Appointment>,
    pub preaching_group: Option<Thing>, // Reference to a Preaching Group
    #[serde(default)]
    pub emergency_contacts: Vec<EmergencyContact>,
}

// === DAO Implementation ===
impl User {
    /// CREATE
    pub async fn create(new_user: User) -> surrealdb::Result<User> {
        let db: &Surreal<Any> = get_db().await?;
        let inserted: User = db.create("user").content(new_user).await?;
        Ok(inserted)
    }

    /// FIND by ID
    pub async fn find(id: surrealdb::RecordId) -> surrealdb::Result<Option<User>> {
        let db: &Surreal<Any> = get_db().await?;
        let user: Option<User> = db.select(id).await?;
        Ok(user)
    }

    /// LIST ALL
    pub async fn all() -> surrealdb::Result<Vec<User>> {
        let db: &Surreal<Any> = get_db().await?;
        let users: Vec<User> = db.select("user").await?;
        Ok(users)
    }

    /// UPDATE
    pub async fn update(id: surrealdb::RecordId, update: User) -> surrealdb::Result<User> {
        let db: &Surreal<Any> = get_db().await?;
        let updated: User = db.update(id).content(update).await?;
        Ok(updated)
    }

    /// DELETE
    pub async fn delete(id: surrealdb::RecordId) -> surrealdb::Result<User> {
        let db: &Surreal<Any> = get_db().await?;
        let deleted: User = db.delete(id).await?;
        Ok(deleted)
    }
}
