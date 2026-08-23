//! Ported from: packages/core/src/v1/config/layout.ts

use serde::{Deserialize, Serialize};

/// Ported from: packages/core/src/v1/config/layout.ts:5 (Layout)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Layout {
    Auto,
    Stretch,
}
