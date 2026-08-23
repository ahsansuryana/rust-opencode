//! Ported from: packages/opencode/src/config/paths.ts

use std::io;
use std::path::{Path, PathBuf};

use oc_global::flag;
use oc_global::global;

use crate::fs_util;

/// Ported from: packages/opencode/src/config/paths.ts:10-21
/// (files — ConfigPaths.projectFiles; termasuk `.toReversed()`)
pub fn files(name: &str, directory: &Path, worktree: Option<&Path>) -> io::Result<Vec<PathBuf>> {
    let targets = vec![format!("{name}.jsonc"), format!("{name}.json")];
    let mut found = fs_util::up(&targets, directory, worktree)?;
    found.reverse();
    Ok(found)
}

/// Ported from: packages/opencode/src/config/paths.ts:23-41 (directories)
pub fn directories(directory: &Path, worktree: Option<&Path>) -> io::Result<Vec<PathBuf>> {
    let paths = global::path();
    let mut items = vec![paths.config.clone()];

    if !flag::open_code_disable_project_config() {
        items.extend(fs_util::up(
            &[".opencode".to_string()],
            directory,
            worktree,
        )?);
    }

    {
        let home = paths.home();
        items.extend(fs_util::up(
            &[".opencode".to_string()],
            &home,
            Some(home.as_path()),
        )?);
    }

    if let Some(config_dir) = flag::open_code_config_dir() {
        items.push(PathBuf::from(config_dir));
    }

    Ok(fs_util::unique(items))
}

/// Ported from: packages/opencode/src/config/paths.ts:43-45 (fileInDirectory)
pub fn file_in_directory(dir: &Path, name: &str) -> [PathBuf; 2] {
    [
        dir.join(format!("{name}.json")),
        dir.join(format!("{name}.jsonc")),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_in_directory_orders_json_then_jsonc() {
        let dir = Path::new("/x");
        let [json, jsonc] = file_in_directory(dir, "opencode");
        assert_eq!(json, Path::new("/x/opencode.json"));
        assert_eq!(jsonc, Path::new("/x/opencode.jsonc"));
    }

    #[test]
    fn files_reverses_up_order_outermost_first() {
        let root = std::env::temp_dir().join(format!("oc-paths-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("a/b")).unwrap();
        std::fs::write(root.join("a/opencode.json"), "{}").unwrap();
        std::fs::write(root.join("a/b/opencode.json"), "{}").unwrap();

        let found = files("opencode", &root.join("a/b"), Some(root.as_path())).unwrap();
        // toReversed: paling luar dulu, yang terdekat terakhir (menimpa)
        assert_eq!(
            found,
            vec![root.join("a/opencode.json"), root.join("a/b/opencode.json")]
        );
    }
}
