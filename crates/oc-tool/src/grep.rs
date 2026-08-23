//! Ported from: packages/opencode/src/tool/grep.ts

use std::path::Path;

use serde_json::json;

use crate::path_safety::assert_external_directory;
use crate::{resolve, Context, ExecuteResult, ToolDef, ToolError};

pub fn execute(params: &serde_json::Value, ctx: &Context) -> Result<ExecuteResult, ToolError> {
    let empty = ExecuteResult {
        title: String::new(),
        metadata: json!({"matches": 0, "truncated": false}),
        output: "No files found".to_string(),
    };
    let pattern = params["pattern"]
        .as_str()
        .ok_or_else(|| ToolError::Message("pattern is required".to_string()))?;
    if pattern.is_empty() {
        return Err(ToolError::Message("pattern is required".to_string()));
    }
    let path_param = params.get("path").and_then(serde_json::Value::as_str);
    let include = params.get("include").and_then(serde_json::Value::as_str);

    ctx.ask("grep", vec![pattern.to_string()], vec!["*".to_string()], {
        let mut metadata = oc_config::v1::OrderedMap::new();
        metadata.insert("pattern".to_string(), json!(pattern));
        metadata.insert("path".to_string(), json!(path_param.map(str::to_string)));
        metadata.insert("include".to_string(), json!(include.map(str::to_string)));
        metadata
    })?;

    // requested = absolute(params.path ?? directory)
    let requested = match path_param {
        Some(path_param) => resolve(&ctx.directory, path_param),
        None => ctx.directory.clone(),
    };
    assert_external_directory(ctx, Some(&requested), false, requested.is_dir())?;

    // search = FSUtil.resolve(requested); cwd = dir bila file
    let search = requested.clone();
    let cwd = if search.is_dir() {
        search.clone()
    } else {
        search
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."))
    };

    let limit = 100usize;
    let result = match crate::ripgrep::grep(&cwd, pattern, include, limit) {
        Ok(result) => result,
        // rg tidak tersedia → pencarian literal sederhana (subset; regex butuh
        // binary rg — dicatat di DEVIATIONS § technical notes).
        Err(crate::ripgrep::RipgrepError::BinaryNotFound) => {
            fallback_grep(&cwd, include, pattern, limit)?
        }
        Err(other) => return Err(ToolError::Message(other.to_string())),
    };
    if result.is_empty() {
        return Ok(empty);
    }

    let rows: Vec<(String, u64, String)> = result
        .into_iter()
        .map(|item| {
            (
                resolve(
                    if requested.is_dir() {
                        &requested
                    } else {
                        requested.parent().unwrap_or(Path::new("."))
                    },
                    &item.entry_path,
                )
                .to_string_lossy()
                .replace('\\', "/"),
                item.line_number,
                item.text,
            )
        })
        .collect();

    let truncated = rows.len() == limit;
    let total = rows.len();
    let has_more = truncated;
    let mut output = vec![format!(
        "Found {total} matches{}",
        if has_more {
            " (more matches available)"
        } else {
            ""
        }
    )];

    let mut current = String::new();
    for (path, line, text) in &rows {
        if *current != *path {
            if !current.is_empty() {
                output.push(String::new());
            }
            current = path.clone();
            output.push(format!("{path}:"));
        }
        output.push(format!("  Line {line}: {text}"));
    }

    if truncated {
        output.push(String::new());
        output.push(
            "(Results truncated. Consider using a more specific path or pattern.)".to_string(),
        );
    }

    Ok(ExecuteResult {
        title: pattern.to_string(),
        metadata: json!({
            "matches": total,
            "truncated": truncated,
        }),
        output: output.join("\n"),
    })
}

/// Padanan PathBuf import helper.
use std::path::PathBuf;

/// Pencarian literal per-baris tanpa rg (fallback terbatas: pattern harus
/// bebas karakter regex khusus).
fn fallback_grep(
    cwd: &Path,
    include: Option<&str>,
    pattern: &str,
    limit: usize,
) -> Result<Vec<crate::ripgrep::GrepMatch>, ToolError> {
    const REGEX_SPECIALS: &str = "\\^$.|?*+()[]{}";
    if pattern.chars().any(|c| REGEX_SPECIALS.contains(c)) {
        return Ok(Vec::new());
    }
    let mut matches = Vec::new();
    let mut files = Vec::new();
    crate::glob::collect_files(cwd, Path::new(""), &mut files)?;
    files.sort();
    for file in files {
        if let Some(include) = include {
            let name = file.to_string_lossy().replace('\\', "/");
            if !glob_match_simple(include, &name) {
                continue;
            }
        }
        let Ok(text) = std::fs::read_to_string(cwd.join(&file)) else {
            continue;
        };
        for (index, line) in text.lines().enumerate() {
            if line.contains(pattern) {
                matches.push(crate::ripgrep::GrepMatch {
                    entry_path: file.to_string_lossy().replace('\\', "/"),
                    line_number: (index + 1) as u64,
                    text: line.to_string(),
                });
                if matches.len() >= limit {
                    return Ok(matches);
                }
            }
        }
    }
    Ok(matches)
}

fn glob_match_simple(pattern: &str, path: &str) -> bool {
    // dukung *.ext sederhana dan **/ prefix
    let pattern = pattern.strip_prefix("**/").unwrap_or(pattern);
    let name_only = path.rsplit('/').next().unwrap_or(path);
    if pattern.contains('/') {
        return false;
    }
    match pattern.split_once('*') {
        Some((prefix, suffix)) => {
            name_only.starts_with(prefix)
                && name_only.ends_with(suffix)
                && name_only.len() >= prefix.len() + suffix.len()
        }
        None => name_only == pattern,
    }
}

/// Ported from: tool/grep.ts + DESCRIPTION grep.txt (verbatim).
pub const GREP_TOOL: ToolDef = ToolDef {
    id: "grep",
    description: include_str!("../assets/grep.txt"),
    execute,
};
