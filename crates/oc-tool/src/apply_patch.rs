//! Ported from: packages/opencode/src/tool/apply_patch.ts

use std::path::PathBuf;

use serde_json::json;

use crate::edit;
use crate::patch::{self, Hunk};
use crate::path_safety::assert_external_directory;
use crate::{relative, resolve, Context, ExecuteResult, ToolDef, ToolError};

fn bom_split(content: &str) -> (bool, String) {
    match content.strip_prefix('\u{feff}') {
        Some(rest) => (true, rest.to_string()),
        None => (false, content.to_string()),
    }
}

fn bom_join(text: &str, bom: bool) -> String {
    if bom {
        format!("\u{feff}{text}")
    } else {
        text.to_string()
    }
}

enum ChangeType {
    Add,
    Update,
    Move,
    Delete,
}

impl ChangeType {
    fn as_str(&self) -> &'static str {
        match self {
            ChangeType::Add => "add",
            ChangeType::Update => "update",
            ChangeType::Move => "move",
            ChangeType::Delete => "delete",
        }
    }
}

struct FileChange {
    file_path: PathBuf,
    #[allow(dead_code)] // dipertahankan meniru struktur TS (dipakai LSP/format nanti)
    old_content: String,
    new_content: String,
    kind: ChangeType,
    move_path: Option<PathBuf>,
    diff: String,
    additions: u64,
    deletions: u64,
    bom: bool,
}

pub fn execute(params: &serde_json::Value, ctx: &Context) -> Result<ExecuteResult, ToolError> {
    let patch_text = params["patchText"]
        .as_str()
        .ok_or_else(|| ToolError::Message("patchText is required".to_string()))?;
    if patch_text.is_empty() {
        return Err(ToolError::Message("patchText is required".to_string()));
    }

    let hunks = patch::parse_patch(patch_text)
        .map_err(|error| ToolError::Message(format!("apply_patch verification failed: {error}")))?;

    if hunks.is_empty() {
        let normalized = patch_text
            .replace("\r\n", "\n")
            .replace('\r', "\n")
            .trim()
            .to_string();
        if normalized == "*** Begin Patch\n*** End Patch" {
            return Err(ToolError::Message(
                "patch rejected: empty patch".to_string(),
            ));
        }
        return Err(ToolError::Message(
            "apply_patch verification failed: no hunks found".to_string(),
        ));
    }

    let mut file_changes: Vec<FileChange> = Vec::new();
    let mut total_diff = String::new();

    for hunk in &hunks {
        let file_path = resolve(&ctx.directory, hunk.path());
        assert_external_directory(ctx, Some(&file_path), false, false)?;

        match hunk {
            Hunk::Add { contents, .. } => {
                let old_content = String::new();
                let new_content = if contents.is_empty() || contents.ends_with('\n') {
                    contents.clone()
                } else {
                    format!("{contents}\n")
                };
                let (next_bom, next_text) = bom_split(&new_content);
                let diff = edit::two_file_patch(&old_content, &next_text);
                let (additions, deletions) = edit_count_changes(&old_content, &next_text);
                total_diff.push_str(&diff);
                total_diff.push('\n');
                file_changes.push(FileChange {
                    file_path,
                    old_content,
                    new_content: next_text,
                    kind: ChangeType::Add,
                    move_path: None,
                    diff,
                    additions,
                    deletions,
                    bom: next_bom,
                });
            }
            Hunk::Update {
                chunks, move_path, ..
            } => {
                if !file_path.exists() || file_path.is_dir() {
                    return Err(ToolError::Message(format!(
                        "apply_patch verification failed: Failed to read file to update: {}",
                        file_path.display()
                    )));
                }
                let raw = std::fs::read_to_string(&file_path)?;
                let (source_bom, source_text) = bom_split(&raw);

                let original_with_bom = bom_join(&source_text, source_bom);
                let update =
                    patch::derive_new_contents_from_chunks(&file_path, chunks, &original_with_bom)
                        .map_err(|error| {
                            ToolError::Message(format!("apply_patch verification failed: {error}"))
                        })?;

                let old_content = source_text;
                let new_content = update.content;
                let bom = update.bom;
                let diff = edit::two_file_patch(&old_content, &new_content);
                let (additions, deletions) = edit_count_changes(&old_content, &new_content);

                let resolved_move = move_path.as_ref().map(|p| resolve(&ctx.directory, p));
                if let Some(move_target) = &resolved_move {
                    assert_external_directory(ctx, Some(move_target), false, false)?;
                }

                total_diff.push_str(&diff);
                total_diff.push('\n');
                file_changes.push(FileChange {
                    file_path,
                    old_content,
                    new_content,
                    kind: if resolved_move.is_some() {
                        ChangeType::Move
                    } else {
                        ChangeType::Update
                    },
                    move_path: resolved_move,
                    diff,
                    additions,
                    deletions,
                    bom,
                });
            }
            Hunk::Delete { .. } => {
                let raw = std::fs::read_to_string(&file_path).map_err(|error| {
                    ToolError::Message(format!("apply_patch verification failed: {error}"))
                })?;
                let (bom, content_to_delete) = bom_split(&raw);
                let delete_diff = edit::two_file_patch(&content_to_delete, "");
                let deletions = content_to_delete.split('\n').count() as u64;
                total_diff.push_str(&delete_diff);
                total_diff.push('\n');
                file_changes.push(FileChange {
                    file_path,
                    old_content: content_to_delete,
                    new_content: String::new(),
                    kind: ChangeType::Delete,
                    move_path: None,
                    diff: delete_diff,
                    additions: 0,
                    deletions,
                    bom,
                });
            }
        }
    }

    // permission ask dengan metadata files array
    let relative_paths: Vec<String> = file_changes
        .iter()
        .map(|change| relative(&ctx.worktree, &change.file_path).replace('\\', "/"))
        .collect();
    let files_meta: Vec<serde_json::Value> = file_changes
        .iter()
        .map(|change| {
            let display_path = change.move_path.as_ref().unwrap_or(&change.file_path);
            json!({
                "filePath": change.file_path.to_string_lossy(),
                "relativePath": relative(&ctx.worktree, display_path).replace('\\', "/"),
                "type": change.kind.as_str(),
                "patch": change.diff,
                "additions": change.additions,
                "deletions": change.deletions,
                "movePath": change.move_path.as_ref().map(|p| p.to_string_lossy()),
            })
        })
        .collect();

    ctx.ask("edit", relative_paths.clone(), vec!["*".to_string()], {
        let mut metadata = oc_config::v1::OrderedMap::new();
        metadata.insert("filepath".to_string(), json!(relative_paths.join(", ")));
        metadata.insert("diff".to_string(), json!(total_diff));
        metadata.insert("files".to_string(), json!(files_meta));
        metadata
    })?;

    // apply perubahan
    for change in &file_changes {
        match change.kind {
            ChangeType::Add | ChangeType::Update => {
                if let Some(parent) = change.file_path.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::write(&change.file_path, bom_join(&change.new_content, change.bom))?;
            }
            ChangeType::Move => {
                if let Some(move_path) = &change.move_path {
                    if let Some(parent) = move_path.parent() {
                        std::fs::create_dir_all(parent)?;
                    }
                    std::fs::write(move_path, bom_join(&change.new_content, change.bom))?;
                    std::fs::remove_file(&change.file_path)?;
                }
            }
            ChangeType::Delete => {
                std::fs::remove_file(&change.file_path)?;
            }
        }
    }

    // ringkasan output
    let summary_lines: Vec<String> = file_changes
        .iter()
        .map(|change| {
            let rel = relative(&ctx.worktree, &change.file_path).replace('\\', "/");
            match change.kind {
                ChangeType::Add => format!("A {rel}"),
                ChangeType::Delete => format!("D {rel}"),
                _ => {
                    let target = change.move_path.as_ref().unwrap_or(&change.file_path);
                    format!("M {}", relative(&ctx.worktree, target).replace('\\', "/"))
                }
            }
        })
        .collect();
    let output = format!(
        "Success. Updated the following files:\n{}",
        summary_lines.join("\n")
    );

    Ok(ExecuteResult {
        title: output.clone(),
        metadata: json!({
            "diff": total_diff,
            "files": files_meta,
            "diagnostics": {},
        }),
        output,
    })
}

/// Padanan loop diffLines utk hitung additions/deletions.
fn edit_count_changes(old_text: &str, new_text: &str) -> (u64, u64) {
    // reuse LCS counter dari modul edit
    count_via_lcs(old_text, new_text)
}

fn count_via_lcs(old_text: &str, new_text: &str) -> (u64, u64) {
    let a: Vec<&str> = old_text.split('\n').collect();
    let b: Vec<&str> = new_text.split('\n').collect();
    let n = a.len();
    let m = b.len();
    let mut dp = vec![vec![0usize; m + 1]; n + 1];
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            dp[i][j] = if a[i] == b[j] {
                dp[i + 1][j + 1] + 1
            } else {
                dp[i + 1][j].max(dp[i][j + 1])
            };
        }
    }
    let (mut additions, mut deletions) = (0u64, 0u64);
    let (mut i, mut j) = (0usize, 0usize);
    while i < n && j < m {
        if a[i] == b[j] {
            i += 1;
            j += 1;
        } else if dp[i + 1][j] >= dp[i][j + 1] {
            deletions += 1;
            i += 1;
        } else {
            additions += 1;
            j += 1;
        }
    }
    deletions += (n - i) as u64;
    additions += (m - j) as u64;
    (additions, deletions)
}

/// Ported from: tool/apply_patch.ts + DESCRIPTION apply_patch.txt (verbatim).
pub const APPLY_PATCH_TOOL: ToolDef = ToolDef {
    id: "apply_patch",
    description: include_str!("../assets/apply_patch.txt"),
    execute,
};
