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
    pub day: Weekday,
    pub time: NaiveTime,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Congregation {
    pub id: surrealdb::RecordId,
    pub name: String,
    pub jw_code: Option<String>,
    pub name_order: NameOrder,
    pub first_weekday: FirstWeekday,
    pub weekday_meeting: Option<MeetingTime>,
    pub weekend_meeting: Option<MeetingTime>,
}

impl Congregation {
    /// CREATE
    pub async fn create(congregation: Congregation) -> surrealdb::Result<Congregation> {
        let db = get_db().await?;
        let created: Congregation = db.create("congregation").content(congregation).await?;
        Ok(created)
    }

    /// FIND by ID
    pub async fn find(id: &str) -> surrealdb::Result<Option<Congregation>> {
        let db = get_db().await?;
        let record: Option<Congregation> = db.select(id).await?;
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
        let updated: Congregation = db.update(id).content(update).await?;
        Ok(updated)
    }

    /// DELETE
    pub async fn delete(id: surrealdb::RecordId) -> surrealdb::Result<Congregation> {
        let db: &Surreal<Any> = get_db().await?;
        let deleted: Congregation = db.delete(id).await?;
        Ok(deleted)
    }
}
