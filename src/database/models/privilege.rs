use serde::{Serialize, Deserialize};
use surrealdb::sql::Thing;
use surrealdb::Surreal;
use surrealdb::engine::any::Any;
use crate::database::db::get_db;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum PrivilegeType {
    Audio,
    Video,
    Stage,
    Microphones,
    FieldServiceMeeting,
    Cleaning,
    Prayer,
    WeekdayChairman,
    WeekendChairman,
    Treasures,
    Gems,
    BibleReading,
    StartingConversation,
    FollowingUp,
    MakingDisciples,
    StudentTalk,
    LivingAsChristians,
    CongregationBibleStudy,
    CongregationBibleStudyReader,
    Attendant,
    EntranceAttendant,
    ZoomAttendant,
    PublicTalk,
    WatchtowerConductor,
    WatchtowerReader,
    FieldServiceMeeting,
    PublicWitnessing,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Privilege {
    pub id: surrealdb::RecordId,
    pub publisher: Option<Thing>, // Reference to a User
    pub type: PrivilegeType,
    pub start_date: Option<chrono::NaiveDate>,
    pub end_date: Option<chrono::NaiveDate>,
    pub notes: Option<String>,
}

impl Privilege {
    /// CREATE
    pub async fn create(privilege: Privilege) -> surrealdb::Result<Privilege> {
        let db = get_db().await?;
        let created: Privilege = db.create("privilege").content(privilege).await?;
        Ok(created)
    }

    /// FIND by ID
    pub async fn find(id: &str) -> surrealdb::Result<Option<Privilege>> {
        let db = get_db().await?;
        let record: Option<Privilege> = db.select(id).await?;
        Ok(record)
    }

    /// LIST ALL
    pub async fn all() -> surrealdb::Result<Vec<Privilege>> {
        let db: &Surreal<Any> = get_db().await?;
        let privileges: Vec<Privilege> = db.select("privilege").await?;
        Ok(privileges)
    }

    /// UPDATE
    pub async fn update(id: surrealdb::RecordId, update: Privilege) -> surrealdb::Result<Privilege> {
        let db: &Surreal<Any> = get_db().await?;
        let updated: Privilege = db.update(id).content(update).await?;
        Ok(updated)
    }

    /// DELETE
    pub async fn delete(id: surrealdb::RecordId) -> surrealdb::Result<Privilege> {
        let db: &Surreal<Any> = get_db().await?;
        let deleted: Privilege = db.delete(id).await?;
        Ok(deleted)
    }
}
