use std::sync::Arc;

use crate::db::Database;
use crate::services::AppDataService;

/// Application state shared across Tauri commands.
#[derive(Clone)]
pub struct AppState {
    pub app_data: AppDataService,
    pub db: Arc<Database>,
}

impl AppState {
    pub fn new(app_data: AppDataService, db: Database) -> Self {
        Self {
            app_data,
            db: Arc::new(db),
        }
    }
}
