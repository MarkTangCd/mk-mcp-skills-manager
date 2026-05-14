// Unified error model exposed to the frontend through Tauri commands.
//
// Frontend code consumes `{ code, message, target?, recoverable, details? }`
// and maps it into user-facing toasts or inline messages.

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandError {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
    pub recoverable: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

impl CommandError {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            target: None,
            recoverable: true,
            details: None,
        }
    }

    pub fn with_target(mut self, target: impl Into<String>) -> Self {
        self.target = Some(target.into());
        self
    }

    pub fn with_details(mut self, details: serde_json::Value) -> Self {
        self.details = Some(details);
        self
    }

    pub fn fatal(mut self) -> Self {
        self.recoverable = false;
        self
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::new("internal", message).fatal()
    }
}

impl From<crate::db::DbError> for CommandError {
    fn from(err: crate::db::DbError) -> Self {
        CommandError::new("db_error", err.to_string()).fatal()
    }
}

impl From<crate::services::app_data::AppDataError> for CommandError {
    fn from(err: crate::services::app_data::AppDataError) -> Self {
        CommandError::new("app_data_error", err.to_string()).fatal()
    }
}

impl From<std::io::Error> for CommandError {
    fn from(err: std::io::Error) -> Self {
        CommandError::new("io_error", err.to_string())
    }
}

pub type CommandResult<T> = Result<T, CommandError>;
