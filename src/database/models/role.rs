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
    pub r#type: RoleType,
    pub start_date: Option<chrono::NaiveDate>,
    pub end_date: Option<chrono::NaiveDate>,
    pub notes: Option<String>,
}

impl Role {
    /// CREATE
    pub async fn create(role: Role) -> surrealdb::Result<Role> {
        let db = get_db().await?;
        let created: Option<Role> = db.create("role").content(role).await?;
        created.ok_or_else(|| surrealdb::Error::Api(surrealdb::error::Api::Query("Failed to create role".to_string())))
    }

    /// FIND by ID
    pub async fn find(id: &str) -> surrealdb::Result<Option<Role>> {
        let db = get_db().await?;
        let record: Option<Role> = db.select(("role", id)).await?;
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
        let updated: Option<Role> = db.update(id).content(update).await?;
        updated.ok_or_else(|| surrealdb::Error::Api(surrealdb::error::Api::Query("Failed to update role".to_string())))
    }

    /// DELETE
    pub async fn delete(id: surrealdb::RecordId) -> surrealdb::Result<Role> {
        let db: &Surreal<Any> = get_db().await?;
        let deleted: Option<Role> = db.delete(id).await?;
        deleted.ok_or_else(|| surrealdb::Error::Api(surrealdb::error::Api::Query("Failed to delete role".to_string())))
    }
}
