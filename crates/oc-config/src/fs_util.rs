//! Ported from: packages/core/src/fs-util.ts (subset yang dipakai jalur config:
//! up / findUp / existsSafe / readFileStringSafe — sisanya menyusul di crate
//! yang membutuhkan, lihat NAMING_MAP.md)

use std::io;
use std::path::{Path, PathBuf};

/// Ported from: packages/core/src/fs-util.ts:58-60 (existsSafe)
pub fn exists_safe(path: &Path) -> bool {
    path.exists()
}

/// Ported from: packages/core/src/fs-util.ts:62-67 (readFileStringSafe)
/// None untuk NotFound/PermissionDenied; error lain tetap gagal.
pub fn read_file_string_safe(path: &Path) -> io::Result<Option<String>> {
    match std::fs::read_to_string(path) {
        Ok(text) => Ok(Some(text)),
        Err(error)
            if matches!(
                error.kind(),
                io::ErrorKind::NotFound | io::ErrorKind::PermissionDenied
            ) =>
        {
            Ok(None)
        }
        Err(error) => Err(error),
    }
}

fn dirname(current: &Path) -> Option<PathBuf> {
    current.parent().map(Path::to_path_buf)
}

/// Ported from: packages/core/src/fs-util.ts:154-166 (findUp)
pub fn find_up(target: &str, start: &Path, stop: Option<&Path>) -> io::Result<Vec<PathBuf>> {
    let mut result = Vec::new();
    let mut current = start.to_path_buf();
    loop {
        let search = current.join(target);
        if search.exists() {
            result.push(search);
        }
        if stop == Some(current.as_path()) {
            break;
        }
        match dirname(&current) {
            Some(parent) if parent != current => current = parent,
            _ => break,
        }
    }
    Ok(result)
}

/// Ported from: packages/core/src/fs-util.ts:168-182 (up)
pub fn up(targets: &[String], start: &Path, stop: Option<&Path>) -> io::Result<Vec<PathBuf>> {
    let mut result = Vec::new();
    let mut current = start.to_path_buf();
    loop {
        for target in targets {
            let search = current.join(target);
            if search.exists() {
                result.push(search);
            }
        }
        if stop == Some(current.as_path()) {
            break;
        }
        match dirname(&current) {
            Some(parent) if parent != current => current = parent,
            _ => break,
        }
    }
    Ok(result)
}

/// Meniru remeda `unique` — dedupe first-occurrence order-preserving.
pub fn unique<T: PartialEq + Clone>(items: Vec<T>) -> Vec<T> {
    let mut result: Vec<T> = Vec::with_capacity(items.len());
    for item in items {
        if !result.contains(&item) {
            result.push(item);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_tree(tag: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!("oc-fsutil-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("a/b/c")).unwrap();
        root
    }

    #[test]
    fn up_walks_from_start_upwards_and_stops_at_worktree() {
        let root = make_tree("walk");
        std::fs::write(root.join("a/opencode.json"), "{}").unwrap();
        std::fs::write(root.join("opencode.jsonc"), "{}").unwrap();

        let found = up(
            &["opencode.jsonc".into(), "opencode.json".into()],
            &root.join("a/b"),
            Some(&root.join("a")),
        )
        .unwrap();
        // urutan: level terdekat dulu (start), berhenti SEBELUM memproses
        // parent dari stop? Tidak — stop diproses juga (break SETELAH cek).
        assert_eq!(found, vec![root.join("a/opencode.json")]);
    }

    #[test]
    fn up_processes_stop_directory_too() {
        let root = make_tree("stop");
        std::fs::write(root.join("a/.opencode"), "").unwrap();

        let found = up(
            &[".opencode".into()],
            &root.join("a/b"),
            Some(&root.join("a")),
        )
        .unwrap();
        assert_eq!(found, vec![root.join("a/.opencode")]);
    }

    #[test]
    fn unique_preserves_first_occurrence() {
        assert_eq!(unique(vec![3, 1, 3, 2, 1]), vec![3, 1, 2]);
    }

    #[test]
    fn read_file_string_safe_returns_none_for_missing() {
        let missing = std::env::temp_dir().join("oc-fsutil-missing-file");
        assert_eq!(read_file_string_safe(&missing).unwrap(), None);
    }
}
