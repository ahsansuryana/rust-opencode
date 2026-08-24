//! Ported dari packages/opencode/src/tool/websearch.ts (helper murni) dan
//! core/util/encode.ts checksum.
//!
//! CATATAN: pemanggilan provider eksternal (exa/parallel via MCP) butuh
//! subsystem MCP client (sprint 13) — execute saat ini mengembalikan fallback
//! message persis seperti hasil kosong di TS; tercatat di DEVIATIONS.

use serde_json::json;

use crate::{Context, ExecuteResult, ToolDef, ToolError};

/// Ported from: core/util/encode.ts:22-30 (checksum — FNV-1a 32-bit, base36)
pub fn checksum(content: &str) -> Option<String> {
    if content.is_empty() {
        return None;
    }
    let mut hash: u32 = 0x811c_9dc5;
    for byte in content.chars() {
        // charCodeAt = UTF-16 code unit
        hash ^= (byte as u16) as u32;
        hash = hash.wrapping_mul(0x0100_0193);
    }
    Some(to_base36(hash))
}

fn hash_to_base36(hash: u32) -> String {
    const ALPHABET: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyz";
    let mut value = hash;
    if value == 0 {
        return "0".to_string();
    }
    let mut buffer = Vec::new();
    while value > 0 {
        buffer.push(ALPHABET[(value % 36) as usize]);
        value /= 36;
    }
    buffer.reverse();
    String::from_utf8(buffer).unwrap()
}

fn to_base36(value: u32) -> String {
    hash_to_base36(value)
}

/// Padanan parseInt(x, 36).
fn parse_base36(text: &str) -> u64 {
    let trimmed = text.trim_start_matches('0');
    if trimmed.is_empty() {
        return 0;
    }
    let mut result: u64 = 0;
    for c in trimmed.chars() {
        let digit = match c {
            '0'..='9' => c as u64 - '0' as u64,
            'a'..='z' => c as u64 - 'a' as u64 + 10,
            _ => return result,
        };
        result = result.saturating_mul(36).saturating_add(digit);
    }
    result
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WebSearchProvider {
    Exa,
    Parallel,
}

impl WebSearchProvider {
    pub fn as_str(self) -> &'static str {
        match self {
            WebSearchProvider::Exa => "exa",
            WebSearchProvider::Parallel => "parallel",
        }
    }
}

pub struct ProviderFlags {
    pub exa: bool,
    pub parallel: bool,
}

/// Ported from: websearch.ts:30-37 (selectWebSearchProvider)
pub fn select_web_search_provider(session_id: &str, flags: &ProviderFlags) -> WebSearchProvider {
    if let Ok(override_value) = std::env::var("OPENCODE_WEBSEARCH_PROVIDER") {
        if override_value == "exa" {
            return WebSearchProvider::Exa;
        }
        if override_value == "parallel" {
            return WebSearchProvider::Parallel;
        }
    }
    if flags.parallel {
        return WebSearchProvider::Parallel;
    }
    if flags.exa {
        return WebSearchProvider::Exa;
    }
    let checksum_text = checksum(session_id).unwrap_or_else(|| "0".to_string());
    if parse_base36(&checksum_text).is_multiple_of(2) {
        WebSearchProvider::Exa
    } else {
        WebSearchProvider::Parallel
    }
}

/// Ported from: websearch.ts:39-43 (webSearchProviderLabel)
pub fn web_search_provider_label(provider: Option<WebSearchProvider>) -> &'static str {
    match provider {
        Some(WebSearchProvider::Parallel) => "Parallel Web Search",
        Some(WebSearchProvider::Exa) => "Exa Web Search",
        None => "Web Search",
    }
}

pub fn execute(params: &serde_json::Value, ctx: &Context) -> Result<ExecuteResult, ToolError> {
    let query = params["query"]
        .as_str()
        .ok_or_else(|| ToolError::Message("query is required".to_string()))?;
    let provider = select_web_search_provider(
        &ctx.session_id,
        &ProviderFlags {
            exa: false,
            parallel: false,
        },
    );
    let title = web_search_provider_label(Some(provider));

    ctx.ask(
        "websearch",
        vec![query.to_string()],
        vec!["*".to_string()],
        {
            let mut metadata = oc_config::v1::OrderedMap::new();
            metadata.insert("query".to_string(), json!(query));
            metadata.insert(
                "numResults".to_string(),
                json!(params
                    .get("numResults")
                    .cloned()
                    .unwrap_or(serde_json::Value::Null)),
            );
            metadata.insert("provider".to_string(), json!(provider.as_str()));
            metadata
        },
    )?;

    // callProvider → MCP client (exa/parallel endpoint) menunggu sprint 13.
    Ok(ExecuteResult {
        output: "No search results found. Please try a different query.".to_string(),
        title: format!("{title}: {query}"),
        metadata: json!({ "provider": provider.as_str() }),
    })
}

/// Ported from: tool/websearch.ts + DESCRIPTION websearch.txt.
/// Deskripsi TS mengganti {{year}} dengan tahun berjalan; di sini template
/// statis + helper `description_rendered`.
pub const WEBSEARCH_TOOL: ToolDef = ToolDef {
    id: "websearch",
    description: include_str!("../assets/websearch.txt"),
    execute,
};

/// Deskripsi dengan {{year}} diganti tahun berjalan (dipakai registry nanti).
pub fn description_rendered() -> String {
    let year = chrono_year();
    include_str!("../assets/websearch.txt").replace("{{year}}", &year)
}

fn chrono_year() -> String {
    let since_epoch = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let days = since_epoch.as_secs() / 86_400;
    // civil-from-days algorithm (Howard Hinnant)
    let z = days as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let year = y + if mp < 10 { 0 } else { 1 };
    year.to_string()
}
