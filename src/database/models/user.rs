use serde::{Serialize, Deserialize};
use surrealdb::sql::Thing;
use surrealdb::Surreal;
use surrealdb::engine::any::Any;
use crate::database::db::get_db;
use chrono::NaiveDate;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct UserEmergencyContact {
    pub firstname: String,
    pub lastname: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub address: Option<String>,
    pub notes: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum UserType {
    Student,
    UnbaptizedPublisher,
    BaptizedPublisher,
    RegularPioneer,
    SpecialPioneer,
    ContiniousAuxiliaryPioneer,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum UserAppointment {
    Elder,
    MinisterialServant,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
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
    pub appointment: Option<UserAppointment>,
    pub preaching_group: Option<Thing>, // Reference to a Preaching Group
    #[serde(default)]
    pub emergency_contacts: Vec<UserEmergencyContact>,
}

// === DAO Implementation ===
impl User {
    /// Hash a password using bcrypt
    pub fn hash_password(password: &str) -> Result<String, bcrypt::BcryptError> {
        bcrypt::hash(password, bcrypt::DEFAULT_COST)
    }
    
    /// Verify a password against a hash
    pub fn verify_password(password: &str, hash: &str) -> Result<bool, bcrypt::BcryptError> {
        bcrypt::verify(password, hash)
    }
    
    /// CREATE
    pub async fn create(mut new_user: User) -> surrealdb::Result<User> {
        // Hash password if present
        if let Some(ref password) = new_user.password {
            let hashed = Self::hash_password(password)
                .map_err(|e| surrealdb::Error::Api(surrealdb::error::Api::Query(format!("Password hashing failed: {}", e))))?;
            new_user.password = Some(hashed);
        }
        
        let db: &Surreal<Any> = get_db().await?;
        let inserted: Option<User> = db.create("user").content(new_user).await?;
        inserted.ok_or_else(|| surrealdb::Error::Api(surrealdb::error::Api::Query("Failed to create user".to_string())))
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
        let updated: Option<User> = db.update(id).content(update).await?;
        updated.ok_or_else(|| surrealdb::Error::Api(surrealdb::error::Api::Query("Failed to update user".to_string())))
    }

    /// DELETE
    pub async fn delete(id: surrealdb::RecordId) -> surrealdb::Result<User> {
        let db: &Surreal<Any> = get_db().await?;
        let deleted: Option<User> = db.delete(id).await?;
        deleted.ok_or_else(|| surrealdb::Error::Api(surrealdb::error::Api::Query("Failed to delete user".to_string())))
    }
}
