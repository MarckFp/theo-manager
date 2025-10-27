use serde::{Serialize, Deserialize};
use surrealdb::sql::Thing;
use surrealdb::Surreal;
use surrealdb::engine::any::Any;
use crate::database::db::get_db;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum SpecialEventType {
    CircuitAssembly,
    RegionalConvention,
    CircuitOverseerVisit,
    Memorial,
    CustomEvent
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SpecialEvent {
    pub id: surrealdb::RecordId,
    pub date: chrono::NaiveDate,
    pub type: SpecialEventType,
    pub title: Option<String>,
}

impl SpecialEvent {
    /// CREATE
    pub async fn create(special_event: SpecialEvent) -> surrealdb::Result<SpecialEvent> {
        let db = get_db().await?;
        let created: SpecialEvent = db.create("special_event").content(special_event).await?;
        Ok(created)
    }

    /// FIND by ID
    pub async fn find(id: &str) -> surrealdb::Result<Option<SpecialEvent>> {
        let db = get_db().await?;
        let record: Option<SpecialEvent> = db.select(id).await?;
        Ok(record)
    }

    /// LIST ALL
    pub async fn all() -> surrealdb::Result<Vec<SpecialEvent>> {
        let db: &Surreal<Any> = get_db().await?;
        let special_events: Vec<SpecialEvent> = db.select("special_event").await?;
        Ok(special_events)
    }

    /// UPDATE
    pub async fn update(id: surrealdb::RecordId, update: SpecialEvent) -> surrealdb::Result<SpecialEvent> {
        let db: &Surreal<Any> = get_db().await?;
        let updated: SpecialEvent = db.update(id).content(update).await?;
        Ok(updated)
    }

    /// DELETE
    pub async fn delete(id: surrealdb::RecordId) -> surrealdb::Result<SpecialEvent> {
        let db: &Surreal<Any> = get_db().await?;
        let deleted: SpecialEvent = db.delete(id).await?;
        Ok(deleted)
    }
}
