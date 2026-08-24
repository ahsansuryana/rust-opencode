//! Ported dari packages/opencode/src/provider/provider.ts (tipe data publik,
//! error, sort/parse helpers) dan provider/error.ts.

pub mod auth;
pub mod error;

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Ported dari schema ModelStatus (v1/config/provider.rs ModelStatus).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelStatus {
    Alpha,
    Beta,
    Deprecated,
    Active,
}

/// Ported from: provider.ts:1053-1068 (Model & sub-structs).
/// Field `api` dipertahankan sebagai JSON mentah karena nilainya berisi
/// metadata npm-package AI-SDK yang tidak direplikasi di Rust (deviasi
/// tercatat di DEVIATIONS.md § sprint 7).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Cost {
    #[serde(default)]
    pub input: f64,
    #[serde(default)]
    pub output: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache_read: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache_write: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_over_200k: Option<Value>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Limit {
    pub context: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input: Option<u64>,
    pub output: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Capabilities {
    #[serde(default)]
    pub tool_call: bool,
    #[serde(default)]
    pub attachment: bool,
    #[serde(default)]
    pub reasoning: bool,
    #[serde(default)]
    pub temperature: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Model {
    pub id: String,
    pub provider_id: String,
    /// Metadata API mentah (npm package + options) — passthrough.
    pub api: Value,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub family: Option<String>,
    pub capabilities: Capabilities,
    pub cost: Cost,
    pub limit: Limit,
    pub status: ModelStatus,
    #[serde(default)]
    pub options: BTreeMap<String, Value>,
    #[serde(default)]
    pub headers: BTreeMap<String, String>,
    #[serde(default)]
    pub release_date: String,
}

/// Ported from: provider.ts:1070-1079 (Provider Info)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Info {
    pub id: String,
    /// Literal ["env","config","custom","api"]
    pub source: Source,
    pub name: String,
    #[serde(default)]
    pub env: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    #[serde(default)]
    pub options: BTreeMap<String, Value>,
    #[serde(default)]
    pub models: BTreeMap<String, Model>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Source {
    Env,
    Config,
    Custom,
    Api,
}

/// Ported from: provider.ts:1083-1094 (ListResult)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListResult {
    pub all: Vec<Info>,
    pub default: BTreeMap<String, String>,
    pub connected: Vec<String>,
}

pub trait ModelsHolder {
    fn model_ids(&self) -> Vec<String>;
}

impl ModelsHolder for Info {
    fn model_ids(&self) -> Vec<String> {
        self.models.keys().cloned().collect()
    }
}

/// Ported from: provider.ts:1112-1115 (defaultModelIDs)
pub fn default_model_ids<T: ModelsHolder>(
    providers: &BTreeMap<String, T>,
) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    for (id, provider) in providers {
        if let Some(first) = sort_models(&provider.model_ids()).first() {
            out.insert(id.clone(), first.clone());
        }
    }
    out
}

/// Ported from: provider.ts:2018-2022 (priority)
const PRIORITY: &[&str] = &["gpt-5", "claude-sonnet-4", "big-pickle", "gemini-3-pro"];

fn priority_index(id: &str) -> i32 {
    PRIORITY
        .iter()
        .position(|p| id.contains(p))
        .map(|i| i as i32)
        .unwrap_or(-1)
}

/// Ported from: provider.ts:2024-2029 (sort)
/// sortBy: [priorityIndex desc, latest asc (0<1), id desc]
pub fn sort_models(models: &[String]) -> Vec<String> {
    let mut models: Vec<String> = models.to_vec();
    models.sort_by(|a, b| {
        let pa = priority_index(a);
        let pb = priority_index(b);
        pb.cmp(&pa)
            .then_with(|| usize::from(a.contains("latest")).cmp(&usize::from(b.contains("latest"))))
            .then_with(|| b.cmp(a))
    });
    models
}

/// Ported from: provider.ts:2033-2040 (parseModel)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedModel {
    pub provider_id: String,
    pub model_id: String,
}

pub fn parse_model(model: &str) -> ParsedModel {
    let mut parts = model.splitn(2, '/');
    let provider_id = parts.next().unwrap_or_default().to_string();
    let model_id = parts.next().unwrap_or_default().to_string();
    ParsedModel {
        provider_id,
        model_id,
    }
}
