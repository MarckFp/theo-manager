use serde::{Serialize, Deserialize};
use surrealdb::sql::Thing;
use surrealdb::Surreal;
use surrealdb::engine::any::Any;
use crate::database::db::get_db;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FieldServiceGroup {
    pub id: surrealdb::RecordId,
    pub name: String,
    pub supervisor: Option<Thing>, // Reference to a User
    pub auxiliar: Option<Thing>,   // Reference to a User
}

impl FieldServiceGroup {
    /// CREATE
    pub async fn create(field_service_group: FieldServiceGroup) -> surrealdb::Result<FieldServiceGroup> {
        let db = get_db().await?;
        let created: FieldServiceGroup = db.create("field_service_group").content(field_service_group).await?;
        Ok(created)
    }

    /// FIND by ID
    pub async fn find(id: &str) -> surrealdb::Result<Option<FieldServiceGroup>> {
        let db = get_db().await?;
        let record: Option<FieldServiceGroup> = db.select(id).await?;
        Ok(record)
    }

    /// LIST ALL
    pub async fn all() -> surrealdb::Result<Vec<FieldServiceGroup>> {
        let db: &Surreal<Any> = get_db().await?;
        let field_service_groups: Vec<FieldServiceGroup> = db.select("field_service_group").await?;
        Ok(field_service_groups)
    }

    /// UPDATE
    pub async fn update(id: surrealdb::RecordId, update: FieldServiceGroup) -> surrealdb::Result<FieldServiceGroup> {
        let db: &Surreal<Any> = get_db().await?;
        let updated: FieldServiceGroup = db.update(id).content(update).await?;
        Ok(updated)
    }

    /// DELETE
    pub async fn delete(id: surrealdb::RecordId) -> surrealdb::Result<FieldServiceGroup> {
        let db: &Surreal<Any> = get_db().await?;
        let deleted: FieldServiceGroup = db.delete(id).await?;
        Ok(deleted)
    }
}
