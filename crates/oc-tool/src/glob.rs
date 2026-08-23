//! Ported from: packages/opencode/src/tool/glob.ts

use std::path::{Path, PathBuf};

use serde_json::json;

use crate::path_safety::assert_external_directory;
use crate::{relative, resolve, Context, ExecuteResult, ToolDef, ToolError};

pub fn execute(params: &serde_json::Value, ctx: &Context) -> Result<ExecuteResult, ToolError> {
    let pattern = params["pattern"]
        .as_str()
        .ok_or_else(|| ToolError::Message("pattern is required".to_string()))?;
    let path_param = params.get("path").and_then(serde_json::Value::as_str);

    ctx.ask("glob", vec![pattern.to_string()], vec!["*".to_string()], {
        let mut metadata = oc_config::v1::OrderedMap::new();
        metadata.insert("pattern".to_string(), json!(pattern));
        metadata.insert("path".to_string(), json!(path_param.map(str::to_string)));
        metadata
    })?;

    let search = resolve(&ctx.directory, path_param.unwrap_or(""));
    if search.is_file() {
        return Err(ToolError::Message(format!(
            "glob path must be a directory: {}",
            search.display()
        )));
    }
    assert_external_directory(ctx, Some(&search), false, true)?;

    let limit = 100usize;
    let files = match crate::ripgrep::glob(&search, pattern, limit) {
        Ok(files) => files,
        Err(crate::ripgrep::RipgrepError::BinaryNotFound) => {
            fallback_glob(&search, pattern, limit)?
        }
        Err(other) => return Err(ToolError::Message(other.to_string())),
    };
    let truncated = files.len() == limit;

    let mut output: Vec<String> = Vec::new();
    if files.is_empty() {
        output.push("No files found".to_string());
    } else {
        for file in &files {
            output.push(resolve(&search, file).to_string_lossy().into_owned());
        }
        if truncated {
            output.push(String::new());
            output.push(format!(
                "(Results are truncated: showing first {limit} results. Consider using a more specific path or pattern.)"
            ));
        }
    }

    Ok(ExecuteResult {
        title: relative(&ctx.worktree, &search),
        output: output.join("\n"),
        metadata: json!({
            "count": files.len(),
            "truncated": truncated,
        }),
    })
}

// --- fallback glob walker (dipakai hanya bila binary rg tidak ada) ---

fn is_sep(c: char) -> bool {
    c == '/' || c == '\\'
}

fn segment_matches(pattern: &str, segment: &str) -> bool {
    let parts: Vec<&str> = pattern.split('*').collect();
    if parts.len() == 1 {
        return pattern == segment;
    }
    if !segment.starts_with(parts[0]) {
        return false;
    }
    let mut cursor = parts[0].len();
    for part in &parts[1..parts.len() - 1] {
        match segment[cursor..].find(part) {
            Some(index) => cursor += index + part.len(),
            None => return false,
        }
    }
    segment[cursor..].ends_with(*parts.last().unwrap())
}

fn match_parts(pattern: &[String], segments: &[String]) -> bool {
    match pattern.split_first() {
        None => segments.is_empty(),
        Some((first, rest)) => {
            if first == "**" {
                (0..=segments.len()).any(|skip| match_parts(rest, &segments[skip..]))
            } else {
                match segments.split_first() {
                    Some((segment, tail)) => {
                        segment_matches(first, segment) && match_parts(rest, tail)
                    }
                    None => false,
                }
            }
        }
    }
}

fn to_segments(path: &Path) -> Vec<String> {
    path.components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect()
}

pub fn collect_files(
    root: &Path,
    relative: &Path,
    out: &mut Vec<PathBuf>,
) -> Result<(), ToolError> {
    for entry in std::fs::read_dir(root.join(relative))? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if name == ".git" {
            continue;
        }
        let child = relative.join(&name);
        if entry.path().is_dir() {
            collect_files(root, &child, out)?;
        } else {
            out.push(child);
        }
    }
    Ok(())
}

fn fallback_glob(root: &Path, pattern: &str, limit: usize) -> Result<Vec<String>, ToolError> {
    let parts: Vec<String> = pattern
        .split(is_sep)
        .filter(|s| !s.is_empty() && *s != ".")
        .map(str::to_string)
        .collect();
    let mut all_files = Vec::new();
    collect_files(root, Path::new(""), &mut all_files)?;
    let mut matched: Vec<PathBuf> = all_files
        .into_iter()
        .filter(|file| match_parts(&parts, &to_segments(file)))
        .collect();
    matched.sort();
    matched.truncate(limit);
    Ok(matched
        .into_iter()
        .map(|p| p.to_string_lossy().replace('\\', "/"))
        .collect())
}

/// Ported from: tool/glob.ts + DESCRIPTION glob.txt (verbatim).
pub const GLOB_TOOL: ToolDef = ToolDef {
    id: "glob",
    description: include_str!("../assets/glob.txt"),
    execute,
};
