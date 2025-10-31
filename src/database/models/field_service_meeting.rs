use serde::{Serialize, Deserialize};
use surrealdb::sql::Thing;
use surrealdb::Surreal;
use surrealdb::engine::any::Any;
use crate::database::db::get_db;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FieldServiceMeeting {
    pub id: surrealdb::RecordId,
    pub weekday: chrono::Weekday,
    pub time: chrono::NaiveTime,
    pub location: String,
    pub notes: Option<String>,
}

impl FieldServiceMeeting {
    /// CREATE
    pub async fn create(field_service_meeting: FieldServiceMeeting) -> surrealdb::Result<FieldServiceMeeting> {
        let db = get_db().await?;
        let created: Option<FieldServiceMeeting> = db.create("field_service_meeting").content(field_service_meeting).await?;
        created.ok_or_else(|| surrealdb::Error::Api(surrealdb::error::Api::Query("Failed to create field service meeting".to_string())))
    }

    /// FIND by ID
    pub async fn find(id: &str) -> surrealdb::Result<Option<FieldServiceMeeting>> {
        let db = get_db().await?;
        let record: Option<FieldServiceMeeting> = db.select(("field_service_meeting", id)).await?;
        Ok(record)
    }

    /// LIST ALL
    pub async fn all() -> surrealdb::Result<Vec<FieldServiceMeeting>> {
        let db: &Surreal<Any> = get_db().await?;
        let field_service_meetings: Vec<FieldServiceMeeting> = db.select("field_service_meeting").await?;
        Ok(field_service_meetings)
    }

    /// UPDATE
    pub async fn update(id: surrealdb::RecordId, update: FieldServiceMeeting) -> surrealdb::Result<FieldServiceMeeting> {
        let db: &Surreal<Any> = get_db().await?;
        let updated: Option<FieldServiceMeeting> = db.update(id).content(update).await?;
        updated.ok_or_else(|| surrealdb::Error::Api(surrealdb::error::Api::Query("Failed to update field service meeting".to_string())))
    }

    /// DELETE
    pub async fn delete(id: surrealdb::RecordId) -> surrealdb::Result<FieldServiceMeeting> {
        let db: &Surreal<Any> = get_db().await?;
        let deleted: Option<FieldServiceMeeting> = db.delete(id).await?;
        deleted.ok_or_else(|| surrealdb::Error::Api(surrealdb::error::Api::Query("Failed to delete field service meeting".to_string())))
    }
}
