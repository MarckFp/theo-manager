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
        let created: Absence = db.create("absence").content(absence).await?;
        Ok(created)
    }

    /// FIND by ID
    pub async fn find(id: &str) -> surrealdb::Result<Option<Absence>> {
        let db = get_db().await?;
        let record: Option<Absence> = db.select(id).await?;
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
        let updated: Absence = db.update(id).content(update).await?;
        Ok(updated)
    }

    /// DELETE
    pub async fn delete(id: surrealdb::RecordId) -> surrealdb::Result<Absence> {
        let db: &Surreal<Any> = get_db().await?;
        let deleted: Absence = db.delete(id).await?;
        Ok(deleted)
    }
}
