//! Ported from: packages/core/src/fs-util.ts (subset: ensureDir, writeWithDirs,
//! isDir) dan helper glob mini untuk kebutuhan storage.

use std::io;
use std::path::{Path, PathBuf};

/// Ported from: packages/core/src/fs-util.ts:69-72 (isDir)
pub fn is_dir(path: &Path) -> bool {
    path.is_dir()
}

/// Ported from: packages/core/src/fs-util.ts:116-125 (ensureDir)
pub fn ensure_dir(path: &Path) -> io::Result<()> {
    match std::fs::create_dir_all(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists && path.is_dir() => Ok(()),
        Err(error) => Err(error),
    }
}

/// Ported from: packages/core/src/fs-util.ts:127-145 (writeWithDirs)
/// Menulis langsung; bila parent belum ada, mkdir recursive lalu tulis ulang.
pub fn write_with_dirs(path: &Path, content: &str) -> io::Result<()> {
    match std::fs::write(path, content) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(path, content)
        }
        Err(error) => Err(error),
    }
}

fn is_separator(c: char) -> bool {
    c == '/' || c == '\\'
}

/// Cocokkan komponen pola terhadap komponen path relatif.
/// `*` = segmen apa pun tanpa `/`; `**` = nol atau lebih segmen.
fn match_segments(pattern: &[&str], segments: &[&str]) -> bool {
    match pattern.split_first() {
        None => segments.is_empty(),
        Some((first, rest)) => {
            if *first == "**" {
                for skip in 0..=segments.len() {
                    if match_segments(rest, &segments[skip..]) {
                        return true;
                    }
                }
                false
            } else {
                match segments.split_first() {
                    Some((segment, tail)) => {
                        segment_matches(first, segment) && match_segments(rest, tail)
                    }
                    None => false,
                }
            }
        }
    }
}

fn segment_matches(pattern: &str, segment: &str) -> bool {
    // wildcard `*` dalam segmen
    let parts: Vec<&str> = pattern.split('*').collect();
    if parts.len() == 1 {
        return pattern == segment;
    }
    let mut cursor = 0usize;
    let Some(first) = parts.first() else {
        return false;
    };
    if !segment.starts_with(first) {
        return false;
    }
    cursor += first.len();
    let middle = &parts[1..parts.len() - 1];
    for part in middle {
        match segment[cursor..].find(part) {
            Some(index) => cursor += index + part.len(),
            None => return false,
        }
    }
    let Some(last) = parts.last() else {
        return false;
    };
    segment[cursor..].ends_with(last)
}

/// Kumpulkan seluruh entri relatif (komponen) di bawah `root` beserta flag
/// apakah itu direktori.
fn walk(root: &Path) -> io::Result<Vec<(Vec<String>, bool)>> {
    let mut result = Vec::new();
    let mut stack = vec![Vec::<String>::new()];
    while let Some(relative) = stack.pop() {
        let full = root.join(relative.join("/"));
        let read = match std::fs::read_dir(&full) {
            Ok(read) => read,
            Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
            Err(error) => return Err(error),
        };
        for entry in read {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().into_owned();
            let mut child = relative.clone();
            child.push(name);
            let is_dir = entry.file_type()?.is_dir();
            if is_dir {
                stack.push(child.clone());
            }
            result.push((child, is_dir));
        }
    }
    Ok(result)
}

/// Ported dari Glob.scan (core/util/glob.ts) subset yang dipakai storage:
/// pattern segmen berbasis `/`, `*` per segmen, `**` lintas depth.
/// Mengembalikan path RELATIF terhadap `cwd`; direktori ikut disertakan bila
/// `include_files_only == false` (padanan `include: "all"`).
pub fn glob_scan(cwd: &Path, pattern: &str, include_files_only: bool) -> io::Result<Vec<PathBuf>> {
    let all = walk(cwd)?;
    let pattern_parts: Vec<&str> = pattern
        .split(is_separator)
        .filter(|s| !s.is_empty())
        .collect();
    let mut matched = Vec::new();
    for (segments, is_dir) in all {
        if include_files_only && is_dir {
            continue;
        }
        let refs: Vec<&str> = segments.iter().map(String::as_str).collect();
        if match_segments(&pattern_parts, &refs) {
            matched.push(PathBuf::from(segments.join("/")));
        }
    }
    Ok(matched)
}
