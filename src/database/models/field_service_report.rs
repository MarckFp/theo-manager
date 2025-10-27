use serde::{Serialize, Deserialize};
use surrealdb::sql::Thing;
use surrealdb::Surreal;
use surrealdb::engine::any::Any;
use crate::database::db::get_db;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum FieldServiceReportStatus {
    Draft,
    Sent,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum FieldServiceReportCommitment {
    15,
    30,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FieldServiceReport {
    pub id: surrealdb::RecordId,
    pub date: chrono::NaiveDate,
    pub publisher: Option<Thing>, // Reference to a User
    pub preached: bool,
    pub status: FieldServiceReportStatus,
    pub hours: Option<i16>,
    pub credits: Option<i16>,
    pub commitment: Option<FieldServiceReportCommitment>,
    pub notes: Option<String>,
}

impl FieldServiceReport {
    /// CREATE
    pub async fn create(field_service_report: FieldServiceReport) -> surrealdb::Result<FieldServiceReport> {
        let db = get_db().await?;
        let created: FieldServiceReport = db.create("field_service_report").content(field_service_report).await?;
        Ok(created)
    }

    /// FIND by ID
    pub async fn find(id: &str) -> surrealdb::Result<Option<FieldServiceReport>> {
        let db = get_db().await?;
        let record: Option<FieldServiceReport> = db.select(id).await?;
        Ok(record)
    }

    /// LIST ALL
    pub async fn all() -> surrealdb::Result<Vec<FieldServiceReport>> {
        let db: &Surreal<Any> = get_db().await?;
        let field_service_reports: Vec<FieldServiceReport> = db.select("field_service_report").await?;
        Ok(field_service_reports)
    }

    /// UPDATE
    pub async fn update(id: surrealdb::RecordId, update: FieldServiceReport) -> surrealdb::Result<FieldServiceReport> {
        let db: &Surreal<Any> = get_db().await?;
        let updated: FieldServiceReport = db.update(id).content(update).await?;
        Ok(updated)
    }

    /// DELETE
    pub async fn delete(id: surrealdb::RecordId) -> surrealdb::Result<FieldServiceReport> {
        let db: &Surreal<Any> = get_db().await?;
        let deleted: FieldServiceReport = db.delete(id).await?;
        Ok(deleted)
    }
}
