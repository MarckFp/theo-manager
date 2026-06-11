use serde::{Deserialize, Serialize};
use surrealdb::types::{RecordId, SurrealValue};

use crate::database::Db;

pub const TABLE: &str = "user_privilege";

/// All assignable privileges for a publisher. One record per user.
/// Every field defaults to `false` so old records without the field deserialise cleanly.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, SurrealValue)]
#[surreal(crate = "surrealdb::types")]
pub struct UserPrivileges {
    pub id: Option<RecordId>,
    pub publisher: RecordId,

    // ── Midweek Meeting ───────────────────────────────────────────────────────
    #[serde(default)] pub weekday_pray: bool,
    #[serde(default)] pub weekday_chairman: bool,
    #[serde(default)] pub aux_chairman: bool,
    #[serde(default)] pub treasures: bool,
    #[serde(default)] pub spiritual_gems: bool,
    #[serde(default)] pub bible_reading: bool,
    #[serde(default)] pub field_ministry_discussion: bool,
    #[serde(default)] pub starting_conversation: bool,
    #[serde(default)] pub following_up: bool,
    #[serde(default)] pub making_disciples: bool,
    #[serde(default)] pub assistant: bool,
    #[serde(default)] pub student_talk: bool,
    #[serde(default)] pub living_as_christians: bool,
    #[serde(default)] pub congregation_bible_study: bool,
    #[serde(default)] pub congregation_bible_study_reader: bool,

    // ── Weekend Meeting ───────────────────────────────────────────────────────
    #[serde(default)] pub weekend_pray: bool,
    #[serde(default)] pub weekend_chairman: bool,
    #[serde(default)] pub watchtower_conductor: bool,
    #[serde(default)] pub public_talks: bool,
    #[serde(default)] pub public_talks_away: bool,

    // ── Platform / Tech ───────────────────────────────────────────────────────
    #[serde(default)] pub stage: bool,
    #[serde(default)] pub audio: bool,
    #[serde(default)] pub video: bool,
    #[serde(default)] pub microphones: bool,
    #[serde(default)] pub attendant: bool,
    #[serde(default)] pub zoom_attendant: bool,

    // ── Other ─────────────────────────────────────────────────────────────────
    #[serde(default)] pub hospitality: bool,
    #[serde(default)] pub interpreter: bool,
    #[serde(default)] pub field_service_meeting: bool,
    #[serde(default)] pub public_witnessing: bool,
    #[serde(default)] pub cleaning: bool,
    #[serde(default)] pub maintenance: bool,
    #[serde(default)] pub territory: bool,
}

/// Payload for creating or updating a privilege record (no `id`).
#[derive(Debug, Clone, Serialize, Deserialize, SurrealValue)]
#[surreal(crate = "surrealdb::types")]
pub struct UserPrivilegesData {
    pub publisher: RecordId,
    pub weekday_pray: bool,
    pub weekday_chairman: bool,
    pub aux_chairman: bool,
    pub treasures: bool,
    pub spiritual_gems: bool,
    pub bible_reading: bool,
    pub field_ministry_discussion: bool,
    pub starting_conversation: bool,
    pub following_up: bool,
    pub making_disciples: bool,
    pub assistant: bool,
    pub student_talk: bool,
    pub living_as_christians: bool,
    pub congregation_bible_study: bool,
    pub congregation_bible_study_reader: bool,
    pub weekend_pray: bool,
    pub weekend_chairman: bool,
    pub watchtower_conductor: bool,
    pub public_talks: bool,
    pub public_talks_away: bool,
    pub stage: bool,
    pub audio: bool,
    pub video: bool,
    pub microphones: bool,
    pub attendant: bool,
    pub zoom_attendant: bool,
    pub hospitality: bool,
    pub interpreter: bool,
    pub field_service_meeting: bool,
    pub public_witnessing: bool,
    pub cleaning: bool,
    pub maintenance: bool,
    pub territory: bool,
}

pub const PRIV_TOTAL: usize = 33;

impl UserPrivileges {
    /// Count how many privilege flags are enabled.
    pub fn count_enabled(&self) -> usize {
        [
            self.weekday_pray, self.weekday_chairman, self.aux_chairman,
            self.treasures, self.spiritual_gems, self.bible_reading,
            self.field_ministry_discussion, self.starting_conversation,
            self.following_up, self.making_disciples, self.assistant,
            self.student_talk, self.living_as_christians,
            self.congregation_bible_study, self.congregation_bible_study_reader,
            self.weekend_pray, self.weekend_chairman, self.watchtower_conductor,
            self.public_talks, self.public_talks_away,
            self.stage, self.audio, self.video, self.microphones,
            self.attendant, self.zoom_attendant,
            self.hospitality, self.interpreter, self.field_service_meeting,
            self.public_witnessing, self.cleaning, self.maintenance, self.territory,
        ]
        .iter()
        .filter(|&&b| b)
        .count()
    }

    /// All privilege records for the congregation.
    pub async fn all(db: &Db) -> Result<Vec<Self>, Box<dyn std::error::Error>> {
        let rows: Vec<Self> = db.select(TABLE).await?;
        Ok(rows)
    }

    /// The privilege record for a specific publisher, if it exists.
    pub async fn by_publisher(
        db: &Db,
        publisher_id: RecordId,
    ) -> Result<Option<Self>, Box<dyn std::error::Error>> {
        let mut rows: Vec<Self> = db
            .query("SELECT * FROM user_privilege WHERE publisher = $id LIMIT 1")
            .bind(("id", publisher_id))
            .await?
            .take(0)?;
        Ok(rows.pop())
    }

    pub async fn create(
        db: &Db,
        data: UserPrivilegesData,
    ) -> Result<Option<Self>, Box<dyn std::error::Error>> {
        let created: Option<Self> = db.create(TABLE).content(data).await?;
        Ok(created)
    }

    pub async fn update(
        db: &Db,
        id: RecordId,
        data: UserPrivilegesData,
    ) -> Result<Option<Self>, Box<dyn std::error::Error>> {
        let updated: Option<Self> = db.update(id).content(data).await?;
        Ok(updated)
    }

    pub async fn delete(db: &Db, id: RecordId) -> surrealdb::Result<Option<Self>> {
        db.delete(id).await
    }
}
