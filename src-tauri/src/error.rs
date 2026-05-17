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

impl From<crate::services::ChangeError> for CommandError {
    fn from(err: crate::services::ChangeError) -> Self {
        match err {
            crate::services::ChangeError::NotFound(id) => {
                CommandError::new("change_not_found", id).with_target("change")
            }
            crate::services::ChangeError::InvalidTransition { from, to } => {
                CommandError::new("invalid_transition", format!("{:?} -> {:?}", from, to))
            }
            crate::services::ChangeError::ValidationFailed => {
                CommandError::new("validation_failed", err.to_string())
            }
            crate::services::ChangeError::PathNotAllowed(path) => {
                CommandError::new("path_not_allowed", path).with_target("change")
            }
            crate::services::ChangeError::BackupFailed(msg) => {
                CommandError::new("backup_failed", msg).with_target("change")
            }
            crate::services::ChangeError::ApplyFailed(msg) => {
                CommandError::new("apply_failed", msg).with_target("change")
            }
            _ => CommandError::new("change_error", err.to_string()),
        }
    }
}

impl From<crate::services::BackupError> for CommandError {
    fn from(err: crate::services::BackupError) -> Self {
        match err {
            crate::services::BackupError::NotFound(id) => {
                CommandError::new("backup_not_found", id).with_target("backup")
            }
            crate::services::BackupError::TargetNotFound(path) => {
                CommandError::new("backup_target_not_found", path).with_target("backup")
            }
            _ => CommandError::new("backup_error", err.to_string()),
        }
    }
}

impl From<std::io::Error> for CommandError {
    fn from(err: std::io::Error) -> Self {
        CommandError::new("io_error", err.to_string())
    }
}

pub type CommandResult<T> = Result<T, CommandError>;
