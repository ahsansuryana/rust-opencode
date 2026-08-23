//! Ported from: packages/core/src/v1/config/lsp.ts

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::OrderedMap;

/// Ported from: packages/core/src/v1/config/lsp.ts:5-7 (Disabled)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspDisabled {
    pub disabled: bool,
}

/// Ported from: packages/core/src/v1/config/lsp.ts:9-18 (Entry)
#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum LspEntry {
    Disabled(LspDisabled),
    Full(LspFull),
}

impl<'de> Deserialize<'de> for LspEntry {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = Value::deserialize(deserializer)?;
        if let Ok(disabled) = serde_json::from_value::<LspDisabled>(value.clone()) {
            if disabled.disabled {
                return Ok(LspEntry::Disabled(disabled));
            }
        }
        serde_json::from_value::<LspFull>(value)
            .map(LspEntry::Full)
            .map_err(serde::de::Error::custom)
    }
}

/// Sisi kedua union Entry (tanpa `disabled: true` literal).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LspFull {
    pub command: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extensions: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disabled: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub env: Option<OrderedMap<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub initialization: Option<OrderedMap<Value>>,
}

/// Ported from: packages/core/src/v1/config/lsp.ts:22-61 (builtinServerIds)
pub const BUILTIN_SERVER_IDS: &[&str] = &[
    "deno",
    "typescript",
    "vue",
    "eslint",
    "oxlint",
    "biome",
    "gopls",
    "ruby-lsp",
    "ty",
    "pyright",
    "elixir-ls",
    "zls",
    "csharp",
    "razor",
    "fsharp",
    "sourcekit-lsp",
    "rust",
    "clangd",
    "svelte",
    "astro",
    "jdtls",
    "kotlin-ls",
    "yaml-ls",
    "lua-ls",
    "php intelephense",
    "prisma",
    "dart",
    "ocaml-lsp",
    "bash",
    "terraform",
    "texlab",
    "dockerfile",
    "gleam",
    "clojure-lsp",
    "nixd",
    "tinymist",
    "haskell-language-server",
    "julials",
];

/// Ported from: packages/core/src/v1/config/lsp.ts:63-78
/// (Info + check requiresExtensionsForCustomServers)
#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum LspInfo {
    Bool(bool),
    Servers(OrderedMap<LspEntry>),
}

impl<'de> Deserialize<'de> for LspInfo {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = Value::deserialize(deserializer)?;
        match value {
            Value::Bool(b) => Ok(LspInfo::Bool(b)),
            Value::Object(map) => {
                for (id, config) in &map {
                    let disabled_truthy = config
                        .get("disabled")
                        .and_then(Value::as_bool)
                        .unwrap_or(false);
                    if disabled_truthy {
                        continue;
                    }
                    if BUILTIN_SERVER_IDS.contains(&id.as_str()) {
                        continue;
                    }
                    // "extensions" in config && Boolean(config.extensions)
                    let has_extensions = config.get("extensions").is_some_and(|v| !v.is_null());
                    if !has_extensions {
                        return Err(serde::de::Error::custom(
                            "For custom LSP servers, 'extensions' array is required.",
                        ));
                    }
                }
                let mut servers = OrderedMap::new();
                for (id, entry) in map {
                    let parsed: LspEntry =
                        serde_json::from_value(entry).map_err(serde::de::Error::custom)?;
                    servers.insert(id, parsed);
                }
                Ok(LspInfo::Servers(servers))
            }
            _ => Err(serde::de::Error::custom(
                "Expected boolean or LSP server record",
            )),
        }
    }
}
