use serde::{Serialize, Deserialize};
use surrealdb::sql::Thing;
use surrealdb::Surreal;
use surrealdb::engine::any::Any;
use crate::database::db::get_db;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MeetingAttendance {
    pub id: surrealdb::RecordId,
    pub date: chrono::NaiveDate,
    pub attendance: i16,
    pub notes: Option<String>,
}

impl MeetingAttendance {
    /// CREATE
    pub async fn create(meeting_attendance: MeetingAttendance) -> surrealdb::Result<MeetingAttendance> {
        let db = get_db().await?;
        let created: MeetingAttendance = db.create("meeting_attendance").content(meeting_attendance).await?;
        Ok(created)
    }

    /// FIND by ID
    pub async fn find(id: &str) -> surrealdb::Result<Option<MeetingAttendance>> {
        let db = get_db().await?;
        let record: Option<MeetingAttendance> = db.select(id).await?;
        Ok(record)
    }

    /// LIST ALL
    pub async fn all() -> surrealdb::Result<Vec<MeetingAttendance>> {
        let db: &Surreal<Any> = get_db().await?;
        let meeting_attendances: Vec<MeetingAttendance> = db.select("meeting_attendance").await?;
        Ok(meeting_attendances)
    }

    /// UPDATE
    pub async fn update(id: surrealdb::RecordId, update: MeetingAttendance) -> surrealdb::Result<MeetingAttendance> {
        let db: &Surreal<Any> = get_db().await?;
        let updated: MeetingAttendance = db.update(id).content(update).await?;
        Ok(updated)
    }

    /// DELETE
    pub async fn delete(id: surrealdb::RecordId) -> surrealdb::Result<MeetingAttendance> {
        let db: &Surreal<Any> = get_db().await?;
        let deleted: MeetingAttendance = db.delete(id).await?;
        Ok(deleted)
    }
}
