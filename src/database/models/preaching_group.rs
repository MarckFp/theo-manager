use serde::{Serialize, Deserialize};
use surrealdb::sql::Thing;
use surrealdb::Surreal;
use surrealdb::engine::any::Any;
use crate::database::db::get_db;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PreachingGroup {
    pub id: surrealdb::RecordId,
    pub name: String,
    pub supervisor: Option<Thing>, // Reference to a User
    pub auxiliar: Option<Thing>,   // Reference to a User
}

impl PreachingGroup {
    /// CREATE
    pub async fn create(preaching_group: PreachingGroup) -> surrealdb::Result<PreachingGroup> {
        let db = get_db().await?;
        let created: PreachingGroup = db.create("preaching_group").content(preaching_group).await?;
        Ok(created)
    }

    /// FIND by ID
    pub async fn find(id: &str) -> surrealdb::Result<Option<PreachingGroup>> {
        let db = get_db().await?;
        let record: Option<PreachingGroup> = db.select(id).await?;
        Ok(record)
    }

    /// LIST ALL
    pub async fn all() -> surrealdb::Result<Vec<PreachingGroup>> {
        let db: &Surreal<Any> = get_db().await?;
        let preaching_groups: Vec<PreachingGroup> = db.select("preaching_group").await?;
        Ok(preaching_groups)
    }

    /// UPDATE
    pub async fn update(id: surrealdb::RecordId, update: PreachingGroup) -> surrealdb::Result<PreachingGroup> {
        let db: &Surreal<Any> = get_db().await?;
        let updated: PreachingGroup = db.update(id).content(update).await?;
        Ok(updated)
    }

    /// DELETE
    pub async fn delete(id: surrealdb::RecordId) -> surrealdb::Result<PreachingGroup> {
        let db: &Surreal<Any> = get_db().await?;
        let deleted: PreachingGroup = db.delete(id).await?;
        Ok(deleted)
    }
}
