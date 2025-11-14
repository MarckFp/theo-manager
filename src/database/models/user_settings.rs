use serde::{Serialize, Deserialize};
use surrealdb::Surreal;
use surrealdb::engine::any::Any;
use crate::database::db::get_db;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct UserSettings {
    pub id: surrealdb::RecordId,
    pub theme: String,
    pub language: String,
}

impl UserSettings {
    /// Get or create settings (singleton pattern - only one settings record)
    pub async fn get_or_create() -> surrealdb::Result<UserSettings> {
        let db: &Surreal<Any> = get_db().await?;
        
        // Try to get existing settings
        let settings: Vec<UserSettings> = db.select("user_settings").await?;
        
        if let Some(existing) = settings.into_iter().next() {
            Ok(existing)
        } else {
            // Create default settings
            let default_settings = UserSettings {
                id: "user_settings:default".parse().unwrap(),
                theme: "dark".to_string(),
                language: "en".to_string(),
            };
            
            let created: Option<UserSettings> = db
                .create("user_settings")
                .content(default_settings)
                .await?;
            
            created.ok_or_else(|| {
                surrealdb::Error::Api(surrealdb::error::Api::Query(
                    "Failed to create settings".to_string()
                ))
            })
        }
    }
    
    /// Update settings
    pub async fn update(theme: String, language: String) -> surrealdb::Result<UserSettings> {
        let db: &Surreal<Any> = get_db().await?;
        
        // Get existing settings to get the ID
        let existing = Self::get_or_create().await?;
        
        let updated_settings = UserSettings {
            id: existing.id.clone(),
            theme,
            language,
        };
        
        let result: Option<UserSettings> = db
            .update(existing.id)
            .content(updated_settings)
            .await?;
        
        result.ok_or_else(|| {
            surrealdb::Error::Api(surrealdb::error::Api::Query(
                "Failed to update settings".to_string()
            ))
        })
    }
}
