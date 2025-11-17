use serde::{Serialize, Deserialize};
use surrealdb::sql::Thing;
use surrealdb::Surreal;
use surrealdb::engine::any::Any;
use crate::database::db::get_db;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct FieldServiceGroup {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<surrealdb::RecordId>,
    pub name: String,
    pub supervisor: Option<Thing>, // Reference to a User
    pub auxiliar: Option<Thing>,   // Reference to a User
    pub members: Vec<Thing>,       // List of User references
}

impl FieldServiceGroup {
    /// CREATE
    pub async fn create(field_service_group: FieldServiceGroup) -> surrealdb::Result<FieldServiceGroup> {
        let db = get_db().await?;
        let created: Option<FieldServiceGroup> = db.create("field_service_group").content(field_service_group).await?;
        created.ok_or_else(|| surrealdb::Error::Api(surrealdb::error::Api::Query("Failed to create field service group".to_string())))
    }

    /// FIND by ID
    pub async fn find(id: &str) -> surrealdb::Result<Option<FieldServiceGroup>> {
        let db = get_db().await?;
        let record: Option<FieldServiceGroup> = db.select((("field_service_group", id))).await?;
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
        let updated: Option<FieldServiceGroup> = db.update(id).content(update).await?;
        updated.ok_or_else(|| surrealdb::Error::Api(surrealdb::error::Api::Query("Failed to update field service group".to_string())))
    }

    /// DELETE
    pub async fn delete(id: surrealdb::RecordId) -> surrealdb::Result<FieldServiceGroup> {
        let db: &Surreal<Any> = get_db().await?;
        let deleted: Option<FieldServiceGroup> = db.delete(id).await?;
        deleted.ok_or_else(|| surrealdb::Error::Api(surrealdb::error::Api::Query("Failed to delete field service group".to_string())))
    }
}
