pub mod absence;
pub mod congregation;
pub mod emergency_contact;
pub mod field_service_group;
pub mod migrate;
pub mod user;

// Re-export the database handle so models can be used without importing database directly.
pub use crate::database::Db;
