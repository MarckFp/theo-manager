use serde::{Deserialize, Serialize};
use surrealdb::types::{RecordId, SurrealValue};

use crate::database::Db;

pub const TERRITORY_TABLE: &str = "territory";
pub const TERRITORY_ADDRESS_TABLE: &str = "territory_address";
pub const TERRITORY_ASSIGNMENT_TABLE: &str = "territory_assignment";

// ── Territory ─────────────────────────────────────────────────────────────────

/// A named territory with an optional map boundary (list of [lat, lng] pairs).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, SurrealValue)]
#[surreal(crate = "surrealdb::types")]
pub struct Territory {
    pub id: Option<RecordId>,
    pub number: String,
    pub name: String,
    pub description: Option<String>,
    /// Polygon boundary as ordered `[lat, lng]` pairs.
    pub boundary: Vec<Vec<f64>>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, SurrealValue)]
#[surreal(crate = "surrealdb::types")]
pub struct TerritoryData {
    pub number: String,
    pub name: String,
    pub description: Option<String>,
    pub boundary: Vec<Vec<f64>>,
    pub notes: Option<String>,
}

impl Territory {
    pub async fn all(db: &Db) -> Result<Vec<Self>, Box<dyn std::error::Error>> {
        let mut rows: Vec<Self> = db
            .query("SELECT * FROM territory ORDER BY number")
            .await?
            .take(0)?;
        Ok(rows)
    }

    pub async fn get(
        db: &Db,
        id: RecordId,
    ) -> Result<Option<Self>, Box<dyn std::error::Error>> {
        Ok(db
            .query("SELECT * FROM $id")
            .bind(("id", id))
            .await?
            .take(0)?)
    }

    pub async fn create(
        db: &Db,
        data: TerritoryData,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let mut r: Vec<Self> = db
            .query("CREATE territory CONTENT $data")
            .bind(("data", data))
            .await?
            .take(0)?;
        r.into_iter().next().ok_or_else(|| "create returned no record".into())
    }

    pub async fn update(
        db: &Db,
        id: RecordId,
        data: TerritoryData,
    ) -> Result<Option<Self>, Box<dyn std::error::Error>> {
        Ok(db
            .query("UPDATE $id CONTENT $data")
            .bind(("id", id))
            .bind(("data", data))
            .await?
            .take(0)?)
    }

    pub async fn delete(
        db: &Db,
        id: RecordId,
    ) -> Result<(), Box<dyn std::error::Error>> {
        db.query("DELETE $id").bind(("id", id)).await?;
        Ok(())
    }
}

// ── TerritoryAddress ──────────────────────────────────────────────────────────

/// A point of interest (house, flat entrance, etc.) within a territory.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, SurrealValue)]
#[surreal(crate = "surrealdb::types")]
pub struct TerritoryAddress {
    pub id: Option<RecordId>,
    pub territory: RecordId,
    pub lat: f64,
    pub lng: f64,
    pub description: String,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, SurrealValue)]
#[surreal(crate = "surrealdb::types")]
pub struct TerritoryAddressData {
    pub territory: RecordId,
    pub lat: f64,
    pub lng: f64,
    pub description: String,
    pub notes: Option<String>,
}

impl TerritoryAddress {
    pub async fn for_territory(
        db: &Db,
        territory: &RecordId,
    ) -> Result<Vec<Self>, Box<dyn std::error::Error>> {
        let rows: Vec<Self> = db
            .query("SELECT * FROM territory_address WHERE territory = $t ORDER BY description")
            .bind(("t", territory.clone()))
            .await?
            .take(0)?;
        Ok(rows)
    }

    pub async fn create(
        db: &Db,
        data: TerritoryAddressData,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let mut r: Vec<Self> = db
            .query("CREATE territory_address CONTENT $data")
            .bind(("data", data))
            .await?
            .take(0)?;
        r.into_iter().next().ok_or_else(|| "create returned no record".into())
    }

    pub async fn delete(
        db: &Db,
        id: RecordId,
    ) -> Result<(), Box<dyn std::error::Error>> {
        db.query("DELETE $id").bind(("id", id)).await?;
        Ok(())
    }
}

// ── TerritoryAssignment ───────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, SurrealValue)]
#[surreal(crate = "surrealdb::types")]
pub struct TerritoryAssignment {
    pub id: Option<RecordId>,
    pub territory: RecordId,
    pub user: RecordId,
    pub assigned_date: String,          // YYYY-MM-DD
    pub returned_date: Option<String>,  // YYYY-MM-DD; None = still out
}

#[derive(Debug, Clone, Serialize, Deserialize, SurrealValue)]
#[surreal(crate = "surrealdb::types")]
pub struct TerritoryAssignmentData {
    pub territory: RecordId,
    pub user: RecordId,
    pub assigned_date: String,
    pub returned_date: Option<String>,
}

impl TerritoryAssignment {
    /// All assignments where assigned_date OR returned_date falls within `year`.
    pub async fn all_for_year(
        db: &Db,
        year: i32,
    ) -> Result<Vec<Self>, Box<dyn std::error::Error>> {
        let prefix = format!("{:04}", year);
        let rows: Vec<Self> = db
            .query(
                "SELECT * FROM territory_assignment \
                 WHERE string::starts_with(assigned_date, $prefix) \
                    OR (returned_date != NONE AND string::starts_with(returned_date, $prefix)) \
                 ORDER BY assigned_date DESC",
            )
            .bind(("prefix", prefix))
            .await?
            .take(0)?;
        Ok(rows)
    }

    /// All currently active (not returned) assignments.
    pub async fn active(db: &Db) -> Result<Vec<Self>, Box<dyn std::error::Error>> {
        let rows: Vec<Self> = db
            .query(
                "SELECT * FROM territory_assignment \
                 WHERE returned_date = NONE ORDER BY assigned_date",
            )
            .await?
            .take(0)?;
        Ok(rows)
    }

    /// Active assignments for a specific user.
    pub async fn active_for_user(
        db: &Db,
        user: &RecordId,
    ) -> Result<Vec<Self>, Box<dyn std::error::Error>> {
        let rows: Vec<Self> = db
            .query(
                "SELECT * FROM territory_assignment \
                 WHERE user = $user AND returned_date = NONE ORDER BY assigned_date",
            )
            .bind(("user", user.clone()))
            .await?
            .take(0)?;
        Ok(rows)
    }

    pub async fn create(
        db: &Db,
        data: TerritoryAssignmentData,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let mut r: Vec<Self> = db
            .query("CREATE territory_assignment CONTENT $data")
            .bind(("data", data))
            .await?
            .take(0)?;
        r.into_iter().next().ok_or_else(|| "create returned no record".into())
    }

    pub async fn return_territory(
        db: &Db,
        id: RecordId,
        returned_date: String,
    ) -> Result<(), Box<dyn std::error::Error>> {
        db.query("UPDATE $id SET returned_date = $date")
            .bind(("id", id))
            .bind(("date", returned_date))
            .await?;
        Ok(())
    }

    pub async fn delete(
        db: &Db,
        id: RecordId,
    ) -> Result<(), Box<dyn std::error::Error>> {
        db.query("DELETE $id").bind(("id", id)).await?;
        Ok(())
    }
}

// ── TerritoryRequest ──────────────────────────────────────────────────────────

pub const TERRITORY_REQUEST_TABLE: &str = "territory_request";

/// A publisher's request for a territory. Fulfilled by the territory overseer.
/// Expires after 30 days if not acted upon.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, SurrealValue)]
#[surreal(crate = "surrealdb::types")]
pub struct TerritoryRequest {
    pub id: Option<RecordId>,
    pub user: RecordId,
    pub notes: Option<String>,
    pub requested_date: String,   // YYYY-MM-DD
    pub status: String,           // "pending" | "fulfilled" | "expired"
}

#[derive(Debug, Clone, Serialize, Deserialize, SurrealValue)]
#[surreal(crate = "surrealdb::types")]
pub struct TerritoryRequestData {
    pub user: RecordId,
    pub notes: Option<String>,
    pub requested_date: String,
    pub status: String,
}

impl TerritoryRequest {
    /// Expire requests with `requested_date <= cutoff_date`, then return pending ones.
    pub async fn expire_and_get_pending(
        db: &Db,
        cutoff_date: &str,
    ) -> Result<Vec<Self>, Box<dyn std::error::Error>> {
        db.query(
            "UPDATE territory_request SET status = 'expired' \
             WHERE status = 'pending' AND requested_date <= $cutoff",
        )
        .bind(("cutoff", cutoff_date.to_string()))
        .await?;
        let rows: Vec<Self> = db
            .query(
                "SELECT * FROM territory_request WHERE status = 'pending' \
                 ORDER BY requested_date DESC",
            )
            .await?
            .take(0)?;
        Ok(rows)
    }

    pub async fn create(
        db: &Db,
        data: TerritoryRequestData,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let mut r: Vec<Self> = db
            .query("CREATE territory_request CONTENT $data")
            .bind(("data", data))
            .await?
            .take(0)?;
        r.into_iter().next().ok_or_else(|| "create returned no record".into())
    }

    pub async fn fulfill(
        db: &Db,
        id: RecordId,
    ) -> Result<(), Box<dyn std::error::Error>> {
        db.query("UPDATE $id SET status = 'fulfilled'")
            .bind(("id", id))
            .await?;
        Ok(())
    }

    pub async fn delete(
        db: &Db,
        id: RecordId,
    ) -> Result<(), Box<dyn std::error::Error>> {
        db.query("DELETE $id").bind(("id", id)).await?;
        Ok(())
    }
}
