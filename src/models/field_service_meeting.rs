use serde::{Deserialize, Serialize};
use surrealdb::types::{RecordId, SurrealValue};

use crate::database::Db;

pub const TABLE: &str = "field_service_meeting";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, SurrealValue)]
#[surreal(crate = "surrealdb::types")]
pub struct FieldServiceMeeting {
    pub id: Option<RecordId>,
    pub date: String,       // "YYYY-MM-DD"
    pub location: String,
    pub assignee: RecordId, // → user record
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, SurrealValue)]
#[surreal(crate = "surrealdb::types")]
pub struct FieldServiceMeetingData {
    pub date: String,
    pub location: String,
    pub assignee: RecordId,
    pub notes: Option<String>,
}

impl FieldServiceMeeting {
    /// All meetings in a given year/month, sorted by date.
    pub async fn all_for_month(
        db: &Db,
        year: i32,
        month: u8,
    ) -> Result<Vec<Self>, Box<dyn std::error::Error>> {
        let prefix = format!("{:04}-{:02}", year, month);
        let mut rows: Vec<Self> = db
            .query("SELECT * FROM field_service_meeting WHERE string::starts_with(date, $prefix)")
            .bind(("prefix", prefix))
            .await?
            .take(0)?;
        rows.sort_by(|a, b| a.date.cmp(&b.date));
        Ok(rows)
    }

    pub async fn create(
        db: &Db,
        data: FieldServiceMeetingData,
    ) -> Result<Option<Self>, Box<dyn std::error::Error>> {
        let created: Option<Self> = db.create(TABLE).content(data).await?;
        Ok(created)
    }

    pub async fn update(
        db: &Db,
        id: RecordId,
        data: FieldServiceMeetingData,
    ) -> Result<Option<Self>, Box<dyn std::error::Error>> {
        let updated: Option<Self> = db.update(id).content(data).await?;
        Ok(updated)
    }

    pub async fn delete(db: &Db, id: RecordId) -> surrealdb::Result<Option<Self>> {
        db.delete(id).await
    }
}
