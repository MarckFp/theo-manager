use serde::{Serialize, Deserialize};
use surrealdb::sql::Thing;
use surrealdb::Surreal;
use surrealdb::engine::any::Any;
use crate::database::db::get_db;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum NameOrder {
    FirstnameLastname,
    LastnameFirstname,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum FirstWeekday {
    Sunday,
    Monday,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MeetingTime {
    pub day: chrono::Weekday,
    pub time: chrono::NaiveTime,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Congregation {
    pub id: surrealdb::RecordId,
    pub name: String,
    pub jw_code: Option<String>,
    pub name_order: NameOrder,
    pub first_weekday: FirstWeekday,
    pub weekday_meeting: MeetingTime,
    pub weekend_meeting: MeetingTime,
}

impl Congregation {
    /// CREATE
    pub async fn create(congregation: Congregation) -> surrealdb::Result<Congregation> {
        let db = get_db().await?;
        let created: Option<Congregation> = db.create("congregation").content(congregation).await?;
        created.ok_or_else(|| surrealdb::Error::Api(surrealdb::error::Api::Query("Failed to create congregation".to_string())))
    }

    /// FIND by ID
    pub async fn find(id: &str) -> surrealdb::Result<Option<Congregation>> {
        let db = get_db().await?;
        let record: Option<Congregation> = db.select(("congregation", id)).await?;
        Ok(record)
    }

    /// LIST ALL
    pub async fn all() -> surrealdb::Result<Vec<Congregation>> {
        let db: &Surreal<Any> = get_db().await?;
        let congregations: Vec<Congregation> = db.select("congregation").await?;
        Ok(congregations)
    }

    /// UPDATE
    pub async fn update(id: surrealdb::RecordId, update: Congregation) -> surrealdb::Result<Congregation> {
        let db: &Surreal<Any> = get_db().await?;
        let updated: Option<Congregation> = db.update(id).content(update).await?;
        updated.ok_or_else(|| surrealdb::Error::Api(surrealdb::error::Api::Query("Failed to update congregation".to_string())))
    }

    /// DELETE
    pub async fn delete(id: surrealdb::RecordId) -> surrealdb::Result<Congregation> {
        let db: &Surreal<Any> = get_db().await?;
        let deleted: Option<Congregation> = db.delete(id).await?;
        deleted.ok_or_else(|| surrealdb::Error::Api(surrealdb::error::Api::Query("Failed to delete congregation".to_string())))
    }
}
