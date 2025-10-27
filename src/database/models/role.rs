use serde::{Serialize, Deserialize};
use surrealdb::sql::Thing;
use surrealdb::Surreal;
use surrealdb::engine::any::Any;
use crate::database::db::get_db;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum RoleType {
    AbsenceViewer,
    AbsenceEditor,
    FieldServiceReportViewer,
    FieldServiceReportEditor,
    FieldServiceMeetingViewer,
    FieldServiceMeetingEditor,
    FieldServiceGroupViewer,
    FieldServiceGroupEditor,
    MeetingAttendanceViewer,
    MeetingAttendanceEditor,
    SpecialEventViewer,
    SpecialEventEditor,
    Owner,
    Editor,
    Viewer,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Role {
    pub id: surrealdb::RecordId,
    pub publisher: Option<Thing>, // Reference to a User
    pub type: RoleType,
    pub start_date: Option<chrono::NaiveDate>,
    pub end_date: Option<chrono::NaiveDate>,
    pub notes: Option<String>,
}

impl Role {
    /// CREATE
    pub async fn create(role: Role) -> surrealdb::Result<Role> {
        let db = get_db().await?;
        let created: Role = db.create("role").content(role).await?;
        Ok(created)
    }

    /// FIND by ID
    pub async fn find(id: &str) -> surrealdb::Result<Option<Role>> {
        let db = get_db().await?;
        let record: Option<Role> = db.select(id).await?;
        Ok(record)
    }

    /// LIST ALL
    pub async fn all() -> surrealdb::Result<Vec<Role>> {
        let db: &Surreal<Any> = get_db().await?;
        let roles: Vec<Role> = db.select("role").await?;
        Ok(roles)
    }

    /// UPDATE
    pub async fn update(id: surrealdb::RecordId, update: Role) -> surrealdb::Result<Role> {
        let db: &Surreal<Any> = get_db().await?;
        let updated: Role = db.update(id).content(update).await?;
        Ok(updated)
    }

    /// DELETE
    pub async fn delete(id: surrealdb::RecordId) -> surrealdb::Result<Role> {
        let db: &Surreal<Any> = get_db().await?;
        let deleted: Role = db.delete(id).await?;
        Ok(deleted)
    }
}
