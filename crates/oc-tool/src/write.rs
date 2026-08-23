//! Ported from: packages/opencode/src/tool/write.ts (subset deterministik:
//! permission + tulis file; LSP diagnostics / formatter / watcher events
//! ditunda — lihat NAMING_MAP).

use std::path::Path;

use serde_json::json;

use crate::path_safety::assert_external_directory;
use crate::{relative, resolve, Context, ExecuteResult, ToolDef, ToolError};

pub fn execute(params: &serde_json::Value, ctx: &Context) -> Result<ExecuteResult, ToolError> {
    let content = params["content"]
        .as_str()
        .ok_or_else(|| ToolError::Message("content is required".to_string()))?;
    let file_path = params["filePath"]
        .as_str()
        .ok_or_else(|| ToolError::Message("filePath is required".to_string()))?;

    let filepath = resolve(&ctx.directory, file_path);
    assert_external_directory(ctx, Some(&filepath), false, false)?;

    let exists = filepath.exists();
    // metadata.diff dipakai ctx.ask; generator unified diff ditunda → string kosong
    let diff = "";
    ctx.ask(
        "edit",
        vec![relative(&ctx.worktree, &filepath)],
        vec!["*".to_string()],
        {
            let mut metadata = oc_config::v1::OrderedMap::new();
            metadata.insert("filepath".to_string(), json!(filepath.to_string_lossy()));
            metadata.insert("diff".to_string(), json!(diff));
            metadata
        },
    )?;

    if let Some(parent) = filepath.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&filepath, content)?;

    Ok(ExecuteResult {
        title: relative(&ctx.worktree, &filepath),
        output: "Wrote file successfully.".to_string(),
        metadata: json!({
            "diagnostics": {},
            "filepath": filepath.to_string_lossy(),
            "exists": exists,
        }),
    })
}

/// Ported from: tool/write.ts + DESCRIPTION write.txt (verbatim).
pub const WRITE_TOOL: ToolDef = ToolDef {
    id: "write",
    description: include_str!("../assets/write.txt"),
    execute,
};

#[allow(dead_code)]
fn unused(_: &Path) {}
