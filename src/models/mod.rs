pub mod absence;
pub mod congregation;
pub mod emergency_contact;
pub mod event;
pub mod field_service_group;
pub mod field_service_meeting;
pub mod field_service_report;
pub mod migrate;
pub mod privilege;
pub mod territory;
pub mod user;
pub mod user_prefs;

// Re-export the database handle so models can be used without importing database directly.
pub use crate::database::Db;
