//! Ported from: packages/core/src/v1/config/attachment.ts

use serde::{Deserialize, Serialize};

use super::server::positive_u64_opt;

/// Ported from: packages/core/src/v1/config/attachment.ts:6-20 (Image)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AttachmentImage {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auto_resize: Option<bool>,
    #[serde(
        default,
        deserialize_with = "positive_u64_opt",
        skip_serializing_if = "Option::is_none"
    )]
    pub max_width: Option<u64>,
    #[serde(
        default,
        deserialize_with = "positive_u64_opt",
        skip_serializing_if = "Option::is_none"
    )]
    pub max_height: Option<u64>,
    #[serde(
        default,
        deserialize_with = "positive_u64_opt",
        skip_serializing_if = "Option::is_none"
    )]
    pub max_base64_bytes: Option<u64>,
}

/// Ported from: packages/core/src/v1/config/attachment.ts:22-25 (Info)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AttachmentInfo {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image: Option<AttachmentImage>,
}
