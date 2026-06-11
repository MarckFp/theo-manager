use serde::{Deserialize, Serialize};
use surrealdb::types::{RecordId, SurrealValue};

use crate::database::Db;

pub const TABLE: &str = "congregation_event";

// ── Date helpers ──────────────────────────────────────────────────────────────

/// Convert Unix seconds to an ISO 8601 date string (`"YYYY-MM-DD"`).
/// Uses the Howard Hinnant civil-calendar algorithm.
fn unix_secs_to_date(secs: u64) -> String {
    let z = secs / 86400 + 719468;
    let era = z / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + if m <= 2 { 1 } else { 0 };
    format!("{:04}-{:02}-{:02}", y, m, d)
}

#[cfg(target_arch = "wasm32")]
pub fn today_str() -> String {
    let d = js_sys::Date::new_0();
    format!(
        "{:04}-{:02}-{:02}",
        d.get_full_year() as u32,
        d.get_month() + 1,
        d.get_date()
    )
}

#[cfg(not(target_arch = "wasm32"))]
pub fn today_str() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    unix_secs_to_date(secs)
}

#[cfg(target_arch = "wasm32")]
pub fn add_days_str(days: u32) -> String {
    let ms = js_sys::Date::now() + days as f64 * 86_400_000.0;
    let secs = (ms / 1000.0) as u64;
    unix_secs_to_date(secs)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn add_days_str(days: u32) -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        + days as u64 * 86_400;
    unix_secs_to_date(secs)
}

// ── Event type enum ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, SurrealValue)]
#[surreal(crate = "surrealdb::types")]
pub enum EventType {
    #[default]
    CircuitAssembly,
    Memorial,
    CircuitOverseerVisit,
    RegionalConvention,
    Other,
}

// ── DB structs ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, SurrealValue)]
#[surreal(crate = "surrealdb::types")]
pub struct CongregationEvent {
    pub id: Option<RecordId>,
    pub start_date: String,
    pub end_date: String,
    pub event_type: EventType,
    pub title: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, SurrealValue)]
#[surreal(crate = "surrealdb::types")]
pub struct CongregationEventData {
    pub start_date: String,
    pub end_date: String,
    pub event_type: EventType,
    pub title: Option<String>,
    pub description: Option<String>,
}

// ── CRUD ──────────────────────────────────────────────────────────────────────

impl CongregationEvent {
    /// Load all events, automatically pruning those whose `end_date` is before today.
    pub async fn all_prune(db: &Db) -> Result<Vec<Self>, Box<dyn std::error::Error>> {
        let today = today_str();
        db.query("DELETE congregation_event WHERE end_date < $today")
            .bind(("today", today))
            .await?;
        let mut rows: Vec<Self> = db.select(TABLE).await?;
        rows.sort_by(|a, b| a.start_date.cmp(&b.start_date));
        Ok(rows)
    }

    /// Events that are ongoing or start within `days` days from today (for dashboard).
    pub async fn upcoming(db: &Db, days: u32) -> Result<Vec<Self>, Box<dyn std::error::Error>> {
        let today = today_str();
        let until = add_days_str(days);
        let mut rows: Vec<Self> = db
            .query(
                "SELECT * FROM congregation_event \
                 WHERE end_date >= $today AND start_date <= $until",
            )
            .bind(("today", today))
            .bind(("until", until))
            .await?
            .take(0)?;
        rows.sort_by(|a, b| a.start_date.cmp(&b.start_date));
        Ok(rows)
    }

    pub async fn create(
        db: &Db,
        data: CongregationEventData,
    ) -> Result<Option<Self>, Box<dyn std::error::Error>> {
        let created: Option<Self> = db.create(TABLE).content(data).await?;
        Ok(created)
    }

    pub async fn update(
        db: &Db,
        id: RecordId,
        data: CongregationEventData,
    ) -> Result<Option<Self>, Box<dyn std::error::Error>> {
        let updated: Option<Self> = db.update(id).content(data).await?;
        Ok(updated)
    }

    pub async fn delete(db: &Db, id: RecordId) -> surrealdb::Result<Option<Self>> {
        db.delete(id).await
    }
}
