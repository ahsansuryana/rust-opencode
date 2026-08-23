//! Ported from: packages/core/src/v1/config/error.ts (NamedError → struct bernama sama)

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// Ported from: packages/core/src/v1/config/error.ts:6-13 (Issue)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Issue {
    pub message: String,
    pub path: Vec<String>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

/// Ported from: packages/core/src/v1/config/error.ts:14-17 (JsonError / ConfigJsonError)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonError {
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl std::fmt::Display for JsonError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "[ConfigJsonError] {}: {}",
            self.path,
            self.message.as_deref().unwrap_or("")
        )
    }
}

/// Ported from: packages/core/src/v1/config/error.ts:19-23 (InvalidError / ConfigInvalidError)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvalidError {
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub issues: Option<Vec<Issue>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl std::fmt::Display for InvalidError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[ConfigInvalidError] {}", self.path)?;
        if let Some(message) = &self.message {
            write!(f, ": {message}")?;
        }
        if let Some(issues) = &self.issues {
            for issue in issues {
                write!(f, "\n  - {}", issue.message)?;
            }
        }
        Ok(())
    }
}
