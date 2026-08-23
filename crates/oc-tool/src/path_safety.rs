//! Ported from: packages/opencode/src/tool/external-directory.ts
//! (assertExternalDirectoryEffect) dan FSUtil.contains.

use std::path::Path;

use serde_json::json;

use crate::{Context, ToolError};

/// Padanan `containsPath(parent, child)` dari project/instance-context.
pub fn contains_path(parent: &Path, child: &Path) -> bool {
    let norm = |p: &Path| -> Vec<String> {
        p.components()
            .map(|c| c.as_os_str().to_string_lossy().into_owned())
            .collect()
    };
    let a = norm(parent);
    let b = norm(child);
    if b.len() < a.len() {
        return false;
    }
    a.iter()
        .zip(b.iter())
        .all(|(x, y)| x.eq_ignore_ascii_case(y))
}

/// Ported from: external-directory.ts:17-46
/// Mengembalikan true bila approval diminta; false bila di dalam boundary /
/// bypass. Di luar boundary → ctx.ask(permission "external_directory").
pub fn assert_external_directory(
    ctx: &Context,
    target: Option<&Path>,
    bypass: bool,
    kind_is_directory: bool,
) -> Result<bool, ToolError> {
    let Some(target) = target else {
        return Ok(false);
    };
    if bypass {
        return Ok(false);
    }

    // containsPath(full, ins) — ins punya directory + worktree; source memakai
    // InstanceContext {directory, worktree}; cek keduanya.
    if contains_path(&ctx.worktree, target) || contains_path(&ctx.directory, target) {
        return Ok(false);
    }

    let dir = if kind_is_directory {
        target.to_path_buf()
    } else {
        target
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| target.to_path_buf())
    };
    let glob = format!("{}/{}", dir.display(), "*").replace('\\', "/");

    let mut metadata = oc_config::v1::OrderedMap::new();
    metadata.insert("filepath".to_string(), json!(target.to_string_lossy()));
    metadata.insert("parentDir".to_string(), json!(dir.to_string_lossy()));

    ctx.ask(
        "external_directory",
        vec![glob.clone()],
        vec![glob],
        metadata,
    )?;
    Ok(true)
}
