use serde::{Serialize, Deserialize};
use surrealdb::sql::Thing;
use surrealdb::Surreal;
use surrealdb::engine::any::Any;
use crate::database::db::get_db;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Absence {
    pub id: surrealdb::RecordId,
    pub start_date: chrono::NaiveDate,
    pub end_date: chrono::NaiveDate,
    pub notes: Option<String>,
    pub publisher: Option<Thing>, // Reference to a User
}

impl Absence {
    /// CREATE
    pub async fn create(absence: Absence) -> surrealdb::Result<Absence> {
        let db = get_db().await?;
        let created: Option<Absence> = db.create("absence").content(absence).await?;
        created.ok_or_else(|| surrealdb::Error::Api(surrealdb::error::Api::Query("Failed to create absence".to_string())))
    }

    /// FIND by ID
    pub async fn find(id: &str) -> surrealdb::Result<Option<Absence>> {
        let db = get_db().await?;
        let record: Option<Absence> = db.select(("absence", id)).await?;
        Ok(record)
    }

    /// LIST ALL
    pub async fn all() -> surrealdb::Result<Vec<Absence>> {
        let db: &Surreal<Any> = get_db().await?;
        let absences: Vec<Absence> = db.select("absence").await?;
        Ok(absences)
    }

    /// UPDATE
    pub async fn update(id: surrealdb::RecordId, update: Absence) -> surrealdb::Result<Absence> {
        let db: &Surreal<Any> = get_db().await?;
        let updated: Option<Absence> = db.update(id).content(update).await?;
        updated.ok_or_else(|| surrealdb::Error::Api(surrealdb::error::Api::Query("Failed to update absence".to_string())))
    }

    /// DELETE
    pub async fn delete(id: surrealdb::RecordId) -> surrealdb::Result<Absence> {
        let db: &Surreal<Any> = get_db().await?;
        let deleted: Option<Absence> = db.delete(id).await?;
        deleted.ok_or_else(|| surrealdb::Error::Api(surrealdb::error::Api::Query("Failed to delete absence".to_string())))
    }
}
