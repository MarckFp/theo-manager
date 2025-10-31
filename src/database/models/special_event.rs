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
    pub r#type: SpecialEventType,
    pub title: Option<String>,
}

impl SpecialEvent {
    /// CREATE
    pub async fn create(special_event: SpecialEvent) -> surrealdb::Result<SpecialEvent> {
        let db = get_db().await?;
        let created: Option<SpecialEvent> = db.create("special_event").content(special_event).await?;
        created.ok_or_else(|| surrealdb::Error::Api(surrealdb::error::Api::Query("Failed to create special event".to_string())))
    }

    /// FIND by ID
    pub async fn find(id: &str) -> surrealdb::Result<Option<SpecialEvent>> {
        let db = get_db().await?;
        let record: Option<SpecialEvent> = db.select(("special_event", id)).await?;
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
        let updated: Option<SpecialEvent> = db.update(id).content(update).await?;
        updated.ok_or_else(|| surrealdb::Error::Api(surrealdb::error::Api::Query("Failed to update special event".to_string())))
    }

    /// DELETE
    pub async fn delete(id: surrealdb::RecordId) -> surrealdb::Result<SpecialEvent> {
        let db: &Surreal<Any> = get_db().await?;
        let deleted: Option<SpecialEvent> = db.delete(id).await?;
        deleted.ok_or_else(|| surrealdb::Error::Api(surrealdb::error::Api::Query("Failed to delete special event".to_string())))
    }
}
