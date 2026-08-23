//! Ported from: packages/opencode/src/storage/storage.ts:81-211 (MIGRATIONS)
//! beserta helper `git.run(["rev-list", ...])` minimal (subset dari @/git —
//! crate git penuh menyusul; folder src/git belum termap di 17 sprint).

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::fs_util;

/// Ported dari pemakaian `git.run` di storage.ts:109-117.
/// Mengembalikan root commit pertama (sorted) atau None bila gagal/kosong.
fn git_rev_list_first_root(cwd: &Path) -> Option<String> {
    let output = Command::new("git")
        .args(["rev-list", "--max-parents=0", "--all"])
        .current_dir(cwd)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let mut lines: Vec<String> = text
        .split('\n')
        .filter(|line| !line.is_empty())
        .map(|line| line.trim().to_string())
        .collect();
    lines.sort();
    lines.into_iter().next()
}

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[derive(Debug, Deserialize)]
struct RootFile {
    #[serde(default)]
    path: Option<RootPath>,
}

#[derive(Debug, Deserialize)]
struct RootPath {
    #[serde(default)]
    root: Option<String>,
}

fn decode_root(value: &Value) -> Option<RootFile> {
    serde_json::from_value(value.clone()).ok()
}

#[derive(Debug, Deserialize)]
struct SessionFile {
    id: String,
}

fn decode_session(value: &Value) -> Option<SessionFile> {
    serde_json::from_value(value.clone()).ok()
}

#[derive(Debug, Deserialize)]
struct MessageFile {
    id: String,
}

fn decode_message(value: &Value) -> Option<MessageFile> {
    serde_json::from_value(value.clone()).ok()
}

#[derive(Debug, Deserialize)]
struct SummaryFile {
    id: String,
    #[serde(rename = "projectID")]
    project_id: String,
    summary: SummaryDiffs,
}

#[derive(Debug, Deserialize)]
struct SummaryDiffs {
    diffs: Vec<DiffEntry>,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub struct DiffEntry {
    pub additions: u64,
    pub deletions: u64,
}

fn decode_summary(value: &Value) -> Option<SummaryFile> {
    serde_json::from_value(value.clone()).ok()
}

fn read_json(path: &Path) -> std::io::Result<Value> {
    let text = std::fs::read_to_string(path)?;
    serde_json::from_str(&text)
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error.to_string()))
}

fn write_json_pretty(path: &Path, content: &Value) -> std::io::Result<()> {
    let text = serde_json::to_string_pretty(content)
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error.to_string()))?;
    fs_util::write_with_dirs(path, &text)
}

fn absolute(base: &Path, relative: &std::path::Path) -> PathBuf {
    base.join(relative)
}

/// Ported from: storage.ts:82-181 (Storage.migration.1)
pub fn migration_1(dir: &Path) -> std::io::Result<()> {
    let project = dir.parent().unwrap_or(dir).join("project");
    if !fs_util::is_dir(&project) {
        return Ok(());
    }
    let project_dirs = fs_util::glob_scan(&project, "*", false)?;
    for project_dir in project_dirs {
        let full = project.join(&project_dir);
        if !fs_util::is_dir(&full) {
            continue;
        }
        tracing::info!("migrating project {}", project_dir.display());
        let mut project_id = project_dir.to_string_lossy().into_owned();
        let mut worktree = "/".to_string();

        if project_id != "global" {
            let msg_files = fs_util::glob_scan(&full, "storage/session/message/*/*.json", true)?;
            for msg_file in msg_files {
                let json = read_json(&absolute(&full, &msg_file))?;
                if let Some(root_file) = decode_root(&json) {
                    if let Some(root_path) = root_file.path.and_then(|p| p.root) {
                        worktree = root_path;
                        break;
                    }
                }
            }
            // TS: `if (!worktree) continue` — "/" selalu truthy, replikasi harfiah
            if worktree.is_empty() {
                continue;
            }
            let worktree_path = PathBuf::from(&worktree);
            if !fs_util::is_dir(&worktree_path) {
                continue;
            }
            let Some(id) = git_rev_list_first_root(&worktree_path) else {
                continue;
            };
            project_id = id;

            let created = now_millis();
            write_json_pretty(
                &dir.join("project").join(format!("{project_id}.json")),
                &serde_json::json!({
                    "id": project_id,
                    "vcs": "git",
                    "worktree": worktree,
                    "time": { "created": created, "initialized": created },
                }),
            )?;

            tracing::info!("migrating sessions for project {project_id}");
            for session_file in fs_util::glob_scan(&full, "storage/session/info/*.json", true)? {
                let basename = session_file
                    .file_name()
                    .map(|b| b.to_string_lossy().into_owned())
                    .unwrap_or_default();
                let dest = dir.join("session").join(&project_id).join(basename);
                tracing::info!(session = %session_file.display(), dest = %dest.display(), "copying");
                let session = read_json(&absolute(&full, &session_file))?;
                let info = decode_session(&session);
                write_json_pretty(&dest, &session)?;
                let Some(info) = info else { continue };

                tracing::info!("migrating messages for session {}", info.id);
                let pattern = format!("storage/session/message/{}/*.json", info.id);
                for msg_file in fs_util::glob_scan(&full, &pattern, true)? {
                    let msg_basename = msg_file
                        .file_name()
                        .map(|b| b.to_string_lossy().into_owned())
                        .unwrap_or_default();
                    let next = dir.join("message").join(&info.id).join(msg_basename);
                    tracing::info!(msg = %msg_file.display(), dest = %next.display(), "copying");
                    let message = read_json(&absolute(&full, &msg_file))?;
                    let item = decode_message(&message);
                    write_json_pretty(&next, &message)?;
                    let Some(item) = item else { continue };

                    tracing::info!("migrating parts for message {}", item.id);
                    let part_pattern =
                        format!("storage/session/part/{}/{}/{}.json", info.id, item.id, "*");
                    for part_file in fs_util::glob_scan(&full, &part_pattern, true)? {
                        let part_basename = part_file
                            .file_name()
                            .map(|b| b.to_string_lossy().into_owned())
                            .unwrap_or_default();
                        let out = dir.join("part").join(&item.id).join(part_basename);
                        tracing::info!(part = %part_file.display(), dest = %out.display(), "copying");
                        let part = read_json(&absolute(&full, &part_file))?;
                        write_json_pretty(&out, &part)?;
                    }
                }
            }
        }
    }
    Ok(())
}

/// Ported from: storage.ts:182-210 (Storage.migration.2)
pub fn migration_2(dir: &Path) -> std::io::Result<()> {
    for item in fs_util::glob_scan(dir, "session/*/*.json", true)? {
        let raw = read_json(&absolute(dir, &item))?;
        let Some(session) = decode_summary(&raw) else {
            continue;
        };
        let diffs = session.summary.diffs;
        write_json_pretty(
            &dir.join("session_diff")
                .join(format!("{}.json", session.id)),
            &serde_json::to_value(&diffs).unwrap_or(Value::Null),
        )?;
        let additions: u64 = diffs.iter().map(|x| x.additions).sum();
        let deletions: u64 = diffs.iter().map(|x| x.deletions).sum();
        let mut next = raw;
        if let Value::Object(map) = &mut next {
            map.insert(
                "summary".to_string(),
                serde_json::json!({ "additions": additions, "deletions": deletions }),
            );
        }
        write_json_pretty(
            &dir.join("session")
                .join(&session.project_id)
                .join(format!("{}.json", session.id)),
            &next,
        )?;
    }
    Ok(())
}
