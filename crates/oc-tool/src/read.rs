//! Ported from: packages/opencode/src/tool/read.ts

use std::path::Path;
use std::sync::OnceLock;

use serde_json::json;

use crate::path_safety::assert_external_directory;
use crate::{relative, resolve, Context, ExecuteResult, ToolDef, ToolError};

const DEFAULT_READ_LIMIT: usize = 2000;
const MAX_LINE_LENGTH: usize = 2000;
const MAX_LINE_SUFFIX: &str = "... (line truncated to 2000 chars)";
const MAX_BYTES: usize = 50 * 1024;
const SAMPLE_BYTES: usize = 4096;

fn max_bytes_label() -> &'static str {
    static LABEL: OnceLock<String> = OnceLock::new();
    LABEL.get_or_init(|| format!("{} KB", MAX_BYTES / 1024))
}

/// Ported from: read.ts:182-227 (isBinaryFile)
fn is_binary_file(filepath: &Path, bytes: &[u8]) -> bool {
    let ext = filepath
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    if matches!(
        ext.as_str(),
        "zip"
            | "tar"
            | "gz"
            | "exe"
            | "dll"
            | "so"
            | "class"
            | "jar"
            | "war"
            | "7z"
            | "doc"
            | "docx"
            | "xls"
            | "xlsx"
            | "ppt"
            | "pptx"
            | "odt"
            | "ods"
            | "odp"
            | "bin"
            | "dat"
            | "obj"
            | "o"
            | "a"
            | "lib"
            | "wasm"
            | "pyc"
            | "pyo"
    ) {
        return true;
    }
    if bytes.is_empty() {
        return false;
    }
    let mut non_printable = 0usize;
    for &byte in bytes {
        if byte == 0 {
            return true;
        }
        if byte < 9 || (byte > 13 && byte < 32) {
            non_printable += 1;
        }
    }
    non_printable as f64 / bytes.len() as f64 > 0.3
}

/// Ported from: read.ts:101-115 (list) — entri direktori + "/" utk folder,
/// sorted localeCompare (di sini sort biasa).
fn list_entries(filepath: &Path) -> Result<Vec<String>, ToolError> {
    let mut items = Vec::new();
    for entry in std::fs::read_dir(filepath)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        let label = if entry.path().is_dir() {
            format!("{name}/")
        } else {
            name
        };
        items.push(label);
    }
    items.sort();
    Ok(items)
}

/// Ported from: read.ts:137-180 (lines) — window baris dengan cap 50 KB.
struct LinesResult {
    raw: Vec<String>,
    count: usize,
    cut: bool,
    more: bool,
    offset: usize,
}

fn read_lines(filepath: &Path, limit: usize, offset: usize) -> Result<LinesResult, ToolError> {
    let text = std::fs::read_to_string(filepath)?;
    // splitLines JS mempertahankan line terakhir tanpa newline
    let mut all_lines: Vec<&str> = text.split('\n').collect();
    if let Some(last) = all_lines.last() {
        if last.is_empty() {
            all_lines.pop();
        }
    }

    let start = offset.saturating_sub(1);
    let mut raw = Vec::new();
    let mut bytes = 0usize;
    let mut cut = false;
    let mut more = false;

    for (index, original) in all_lines.iter().enumerate() {
        if raw.len() >= limit && index >= start {
            more = index >= start;
            break;
        }
        if index < start {
            continue;
        }
        let line = if original.chars().count() > MAX_LINE_LENGTH {
            let truncated: String = original.chars().take(MAX_LINE_LENGTH).collect();
            format!("{truncated}{MAX_LINE_SUFFIX}")
        } else {
            (*original).to_string()
        };
        let size = line.len() + if raw.is_empty() { 0 } else { 1 };
        if bytes + size <= MAX_BYTES {
            raw.push(line);
            bytes += size;
        } else {
            cut = true;
            more = true;
            break;
        }
    }

    Ok(LinesResult {
        count: all_lines.len(),
        raw,
        cut,
        more,
        offset,
    })
}

/// Ported from: read.ts:76-99 (miss)
fn miss(filepath: &Path) -> ToolError {
    let dir = filepath.parent().unwrap_or(Path::new("/"));
    let base = filepath
        .file_name()
        .map(|b| b.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    let mut items: Vec<String> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            let lower = name.to_lowercase();
            if lower.contains(&base) || base.contains(&lower) {
                items.push(dir.join(name).to_string_lossy().into_owned());
            }
        }
    }
    items.truncate(3);
    if !items.is_empty() {
        return ToolError::Message(format!(
            "File not found: {}\n\nDid you mean one of these?\n{}",
            filepath.display(),
            items.join("\n")
        ));
    }
    ToolError::Message(format!("File not found: {}", filepath.display()))
}

pub fn execute(params: &serde_json::Value, ctx: &Context) -> Result<ExecuteResult, ToolError> {
    let file_path = params["filePath"]
        .as_str()
        .ok_or_else(|| ToolError::Message("filePath is required".to_string()))?;
    let limit_param = params.get("limit").and_then(serde_json::Value::as_u64);
    let offset_param = params.get("offset").and_then(serde_json::Value::as_u64);

    let filepath = resolve(&ctx.directory, file_path);
    let title = relative(&ctx.worktree, &filepath);

    let metadata_exists = filepath.exists();
    assert_external_directory(
        ctx,
        Some(&filepath),
        ctx.bypass_cwd_check,
        filepath.is_dir(),
    )?;

    ctx.ask(
        "read",
        vec![relative(&ctx.worktree, &filepath)],
        vec!["*".to_string()],
        Default::default(),
    )?;

    if !metadata_exists {
        return Err(miss(&filepath));
    }

    if filepath.is_dir() {
        let items = list_entries(&filepath)?;
        let limit = limit_param
            .map(|v| v as usize)
            .unwrap_or(DEFAULT_READ_LIMIT);
        let offset = offset_param.unwrap_or(1) as usize;
        let start = offset.saturating_sub(1);
        let sliced: Vec<String> = items.iter().skip(start).take(limit).cloned().collect();
        let truncated = start + sliced.len() < items.len();

        let tail = if truncated {
            format!(
                "\n(Showing {} of {} entries. Use 'offset' parameter to read beyond entry {})",
                sliced.len(),
                items.len(),
                offset + sliced.len()
            )
        } else {
            format!("\n({} entries)", items.len())
        };
        let output = [
            format!("<path>{}</path>", filepath.display()),
            "<type>directory</type>".to_string(),
            "<entries>".to_string(),
            sliced.join("\n"),
            tail,
            "</entries>".to_string(),
        ]
        .join("\n");
        let preview: Vec<String> = sliced.iter().take(20).cloned().collect();
        return Ok(ExecuteResult {
            title,
            output,
            metadata: json!({
                "preview": preview.join("\n"),
                "truncated": truncated,
                "loaded": [],
            }),
        });
    }

    // file branch
    let sample_len = SAMPLE_BYTES.min(std::fs::metadata(&filepath)?.len() as usize);
    let sample = {
        use std::io::Read;
        let mut buffer = vec![0u8; sample_len];
        let mut file = std::fs::File::open(&filepath)?;
        file.read_exact(&mut buffer).unwrap_or_default();
        buffer
    };

    if is_binary_file(&filepath, &sample) {
        return Err(ToolError::Message(format!(
            "Cannot read binary file: {}",
            filepath.display()
        )));
    }

    let file = read_lines(
        &filepath,
        limit_param
            .map(|v| v as usize)
            .unwrap_or(DEFAULT_READ_LIMIT),
        offset_param.unwrap_or(1) as usize,
    )?;
    if file.count < file.offset && !(file.count == 0 && file.offset == 1) {
        return Err(ToolError::Message(format!(
            "Offset {} is out of range for this file ({} lines)",
            file.offset, file.count
        )));
    }

    let mut output = [
        format!("<path>{}</path>", filepath.display()),
        "<type>file</type>".to_string(),
        "<content>\n".to_string(),
    ]
    .join("\n");
    output.push_str(
        &file
            .raw
            .iter()
            .enumerate()
            .map(|(i, line)| format!("{}: {line}", i + file.offset))
            .collect::<Vec<_>>()
            .join("\n"),
    );

    let last = file.offset + file.raw.len().saturating_sub(1);
    let next = last + 1;
    let truncated = file.more || file.cut;
    if file.cut {
        output.push_str(&format!(
            "\n\n(Output capped at {}. Showing lines {}-{}. Use offset={} to continue.)",
            max_bytes_label(),
            file.offset,
            last,
            next
        ));
    } else if file.more {
        output.push_str(&format!(
            "\n\n(Showing lines {}-{} of {}. Use offset={} to continue.)",
            file.offset, last, file.count, next
        ));
    } else {
        output.push_str(&format!("\n\n(End of file - total {} lines)", file.count));
    }
    output.push_str("\n</content>");

    let preview: Vec<String> = file.raw.iter().take(20).cloned().collect();
    Ok(ExecuteResult {
        title,
        output,
        metadata: json!({
            "preview": preview.join("\n"),
            "truncated": truncated,
            "loaded": [],
        }),
    })
}

/// Ported from: tool/read.ts:64-86 + DESCRIPTION read.txt (verbatim).
pub const READ_TOOL: ToolDef = ToolDef {
    id: "read",
    description: include_str!("../assets/read.txt"),
    execute,
};
