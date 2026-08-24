//! Ported from: packages/opencode/src/patch/index.ts

use std::path::Path;

use crate::ToolError;

#[derive(Debug, Clone)]
pub struct UpdateFileChunk {
    pub old_lines: Vec<String>,
    pub new_lines: Vec<String>,
    pub change_context: Option<String>,
    pub is_end_of_file: bool,
}

/// Ported from: patch/index.ts:19-29 (Hunk union)
#[derive(Debug, Clone)]
pub enum Hunk {
    Add {
        path: String,
        contents: String,
    },
    Delete {
        path: String,
    },
    Update {
        path: String,
        move_path: Option<String>,
        chunks: Vec<UpdateFileChunk>,
    },
}

impl Hunk {
    pub fn path(&self) -> &str {
        match self {
            Hunk::Add { path, .. } | Hunk::Delete { path } | Hunk::Update { path, .. } => path,
        }
    }
}

fn strip_heredoc(input: &str) -> String {
    // /^(?:cat\s+)?<<['"]?(\w+)['"]?\s*\n([\s\S]*?)\n\1\s*$/
    let trimmed = input.trim();
    let mut rest = trimmed;
    if let Some(after_cat) = rest.strip_prefix("cat") {
        let after_ws = after_cat.trim_start();
        let consumed = after_cat.len() - after_ws.len();
        if consumed > 0 {
            rest = after_ws;
        }
    }
    let Some(rest_heredoc) = rest.strip_prefix("<<") else {
        return input.to_string();
    };
    let after_marker = rest_heredoc
        .strip_prefix('\'')
        .or_else(|| rest_heredoc.strip_prefix('"'))
        .unwrap_or(rest_heredoc);
    let word: String = after_marker
        .chars()
        .take_while(|c| c.is_alphanumeric() || *c == '_')
        .collect();
    if word.is_empty() {
        return input.to_string();
    }
    let after_word = &after_marker[word.len()..];
    let after_close_quote = after_word
        .strip_prefix('\'')
        .or_else(|| after_word.strip_prefix('"'))
        .unwrap_or(after_word);
    let Some(newline_pos) = after_close_quote.find('\n') else {
        return input.to_string();
    };
    if !after_close_quote[..newline_pos].trim().is_empty() {
        return input.to_string();
    }
    let body_start = newline_pos + 1;
    let body_and_tail = &after_close_quote[body_start..];
    // cari penutup \n{word} di akhir
    let closer = format!("\n{word}");
    match body_and_tail.rfind(&closer) {
        Some(pos) => {
            let body = &body_and_tail[..pos];
            let tail = &body_and_tail[pos + closer.len()..];
            if tail.trim().is_empty() {
                body.to_string()
            } else {
                input.to_string()
            }
        }
        None => input.to_string(),
    }
}

/// Ported from: patch/index.ts:185-241 (parsePatch)
pub fn parse_patch(patch_text: &str) -> Result<Vec<Hunk>, ToolError> {
    const BEGIN: &str = "*** Begin Patch";
    const END: &str = "*** End Patch";

    let cleaned = strip_heredoc(patch_text);
    let lines: Vec<&str> = cleaned.split('\n').collect();

    let begin_idx = lines.iter().position(|line| line.trim() == BEGIN);
    let end_idx = lines.iter().position(|line| line.trim() == END);

    let (Some(begin_idx), Some(end_idx)) = (begin_idx, end_idx) else {
        return Err(ToolError::Message(
            "Invalid patch format: missing Begin/End markers".to_string(),
        ));
    };
    if begin_idx >= end_idx {
        return Err(ToolError::Message(
            "Invalid patch format: missing Begin/End markers".to_string(),
        ));
    }

    let mut hunks = Vec::new();
    let mut i = begin_idx + 1;

    while i < end_idx {
        let line = lines[i];
        if let Some(file_path) = line.strip_prefix("*** Add File:") {
            let file_path = file_path.trim();
            if file_path.is_empty() {
                i += 1;
                continue;
            }
            let (content, next_idx) = parse_add_file_content(&lines, i + 1);
            hunks.push(Hunk::Add {
                path: file_path.to_string(),
                contents: content,
            });
            i = next_idx;
            continue;
        }
        if let Some(file_path) = line.strip_prefix("*** Delete File:") {
            let file_path = file_path.trim();
            if file_path.is_empty() {
                i += 1;
                continue;
            }
            hunks.push(Hunk::Delete {
                path: file_path.to_string(),
            });
            i += 1;
            continue;
        }
        if let Some(file_path) = line.strip_prefix("*** Update File:") {
            let file_path = file_path.trim();
            if file_path.is_empty() {
                i += 1;
                continue;
            }
            let mut move_path = None;
            let mut next_idx = i + 1;
            if next_idx < lines.len() && lines[next_idx].starts_with("*** Move to:") {
                move_path = Some(lines[next_idx]["*** Move to:".len()..].trim().to_string());
                next_idx += 1;
            }
            let (chunks, next_idx) = parse_update_file_chunks(&lines, next_idx);
            hunks.push(Hunk::Update {
                path: file_path.to_string(),
                move_path,
                chunks,
            });
            i = next_idx;
            continue;
        }
        i += 1;
    }

    Ok(hunks)
}

/// Ported from: patch/index.ts:103-155 (parseUpdateFileChunks)
fn parse_update_file_chunks(lines: &[&str], start_idx: usize) -> (Vec<UpdateFileChunk>, usize) {
    let mut chunks = Vec::new();
    let mut i = start_idx;

    while i < lines.len() && !lines[i].starts_with("***") {
        if lines[i].starts_with("@@") {
            let context_line = lines[i][2..].trim().to_string();
            i += 1;

            let mut old_lines: Vec<String> = Vec::new();
            let mut new_lines: Vec<String> = Vec::new();
            let mut is_end_of_file = false;

            while i < lines.len() && !lines[i].starts_with("@@") && !lines[i].starts_with("***") {
                let change_line = lines[i];
                if change_line == "*** End of File" {
                    is_end_of_file = true;
                    i += 1;
                    break;
                }
                if let Some(content) = change_line.strip_prefix(' ') {
                    old_lines.push(content.to_string());
                    new_lines.push(content.to_string());
                } else if let Some(content) = change_line.strip_prefix('-') {
                    old_lines.push(content.to_string());
                } else if let Some(content) = change_line.strip_prefix('+') {
                    new_lines.push(content.to_string());
                }
                i += 1;
            }

            chunks.push(UpdateFileChunk {
                old_lines,
                new_lines,
                change_context: if context_line.is_empty() {
                    None
                } else {
                    Some(context_line)
                },
                is_end_of_file,
            });
        } else {
            i += 1;
        }
    }

    (chunks, i)
}

/// Ported from: patch/index.ts:157-174 (parseAddFileContent)
fn parse_add_file_content(lines: &[&str], start_idx: usize) -> (String, usize) {
    let mut content = String::new();
    let mut i = start_idx;
    while i < lines.len() && !lines[i].starts_with("***") {
        if let Some(body) = lines[i].strip_prefix('+') {
            content.push_str(body);
            content.push('\n');
        }
        i += 1;
    }
    if content.ends_with('\n') {
        content.pop();
    }
    (content, i)
}

// --- apply helpers (patch/index.ts:342-511) ---

/// Ported from: patch/index.ts:418-425 (normalizeUnicode)
fn normalize_unicode(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for c in input.chars() {
        match c {
            '\u{2018}' | '\u{2019}' | '\u{201A}' | '\u{201B}' => out.push('\''),
            '\u{201C}' | '\u{201D}' | '\u{201E}' | '\u{201F}' => out.push('"'),
            '\u{2010}' | '\u{2011}' | '\u{2012}' | '\u{2013}' | '\u{2014}' | '\u{2015}' => {
                out.push('-')
            }
            '\u{2026}' => out.push_str("..."),
            '\u{00A0}' => out.push(' '),
            other => out.push(other),
        }
    }
    out
}

type Comparator = fn(&str, &str) -> bool;

fn compare_exact(a: &str, b: &str) -> bool {
    a == b
}
fn compare_rstrip(a: &str, b: &str) -> bool {
    a.trim_end() == b.trim_end()
}
fn compare_trim(a: &str, b: &str) -> bool {
    a.trim() == b.trim()
}
fn compare_normalized(a: &str, b: &str) -> bool {
    normalize_unicode(a.trim()) == normalize_unicode(b.trim())
}

/// Ported from: patch/index.ts:429-458 (tryMatch)
fn try_match(
    lines: &[&str],
    pattern: &[&str],
    start_index: usize,
    compare: Comparator,
    eof: bool,
) -> isize {
    if eof && pattern.len() <= lines.len() {
        let from_end = lines.len() - pattern.len();
        if from_end >= start_index {
            let mut matches = true;
            for j in 0..pattern.len() {
                if !compare(lines[from_end + j], pattern[j]) {
                    matches = false;
                    break;
                }
            }
            if matches {
                return from_end as isize;
            }
        }
    }
    if lines.len() >= pattern.len() {
        for i in start_index..=(lines.len() - pattern.len()) {
            let mut matches = true;
            for j in 0..pattern.len() {
                if !compare(lines[i + j], pattern[j]) {
                    matches = false;
                    break;
                }
            }
            if matches {
                return i as isize;
            }
        }
    }
    -1
}

/// Ported from: patch/index.ts:460-484 (seekSequence)
fn seek_sequence(lines: &[&str], pattern: &[&str], start_index: usize, eof: bool) -> isize {
    if pattern.is_empty() {
        return -1;
    }
    let exact = try_match(lines, pattern, start_index, compare_exact, eof);
    if exact != -1 {
        return exact;
    }
    let rstrip = try_match(lines, pattern, start_index, compare_rstrip, eof);
    if rstrip != -1 {
        return rstrip;
    }
    let trim = try_match(lines, pattern, start_index, compare_trim, eof);
    if trim != -1 {
        return trim;
    }
    try_match(lines, pattern, start_index, compare_normalized, eof)
}

type Replacement = (usize, usize, Vec<String>);

/// Ported from: patch/index.ts:342-396 (computeReplacements)
fn compute_replacements(
    original_lines: &[&str],
    file_path: &Path,
    chunks: &[UpdateFileChunk],
) -> Result<Vec<Replacement>, ToolError> {
    let mut replacements: Vec<Replacement> = Vec::new();
    let mut line_index = 0usize;

    for chunk in chunks {
        if let Some(context) = &chunk.change_context {
            let context_idx = seek_sequence(original_lines, &[context.as_str()], line_index, false);
            if context_idx == -1 {
                return Err(ToolError::Message(format!(
                    "Failed to find context '{context}' in {}",
                    file_path.display()
                )));
            }
            line_index = (context_idx + 1) as usize;
        }

        if chunk.old_lines.is_empty() {
            let insertion_idx = if !original_lines.is_empty() && original_lines.last() == Some(&"")
            {
                original_lines.len() - 1
            } else {
                original_lines.len()
            };
            replacements.push((insertion_idx, 0, chunk.new_lines.clone()));
            continue;
        }

        let mut pattern = chunk.old_lines.clone();
        let mut new_slice = chunk.new_lines.clone();
        let pattern_refs: Vec<&str> = pattern.iter().map(String::as_str).collect();
        let mut found = seek_sequence(
            original_lines,
            &pattern_refs,
            line_index,
            chunk.is_end_of_file,
        );

        if found == -1 && pattern.last().map(String::is_empty).unwrap_or(false) {
            pattern.pop();
            if new_slice.last().map(String::is_empty).unwrap_or(false) {
                new_slice.pop();
            }
            let refs: Vec<&str> = pattern.iter().map(String::as_str).collect();
            found = seek_sequence(original_lines, &refs, line_index, chunk.is_end_of_file);
        }

        if found != -1 {
            replacements.push((found as usize, pattern.len(), new_slice));
            line_index = found as usize + pattern.len();
        } else {
            return Err(ToolError::Message(format!(
                "Failed to find expected lines in {}:\n{}",
                file_path.display(),
                chunk.old_lines.join("\n")
            )));
        }
    }

    replacements.sort_by_key(|(index, _, _)| *index);
    Ok(replacements)
}

/// Ported from: patch/index.ts:398-415 (applyReplacements)
fn apply_replacements(lines: &[&str], replacements: &[Replacement]) -> Vec<String> {
    let mut result: Vec<String> = lines.iter().map(|s| s.to_string()).collect();
    for (start_idx, old_len, new_segment) in replacements.iter().rev() {
        result.drain(*start_idx..*start_idx + *old_len);
        for (j, segment) in new_segment.iter().enumerate() {
            result.insert(start_idx + j, segment.clone());
        }
    }
    result
}

#[derive(Debug)]
pub struct FileUpdate {
    pub unified_diff: String,
    pub content: String,
    pub bom: bool,
}

fn bom_split(content: &str) -> (bool, String) {
    match content.strip_prefix('\u{feff}') {
        Some(rest) => (true, rest.to_string()),
        None => (false, content.to_string()),
    }
}

/// Padanan generateUnifiedDiff sederhana dari source (baris-per-indeks).
fn generate_unified_diff(old_content: &str, new_content: &str) -> String {
    let old_lines: Vec<&str> = old_content.split('\n').collect();
    let new_lines: Vec<&str> = new_content.split('\n').collect();
    let mut diff = String::from("@@ -1 +1 @@\n");
    let max_len = old_lines.len().max(new_lines.len());
    let mut has_changes = false;
    for i in 0..max_len {
        let old_line = old_lines.get(i).copied().unwrap_or("");
        let new_line = new_lines.get(i).copied().unwrap_or("");
        if old_line != new_line {
            if !old_line.is_empty() {
                diff.push_str(&format!("-{old_line}\n"));
            }
            if !new_line.is_empty() {
                diff.push_str(&format!("+{new_line}\n"));
            }
            has_changes = true;
        } else if !old_line.is_empty() {
            diff.push_str(&format!(" {old_line}\n"));
        }
    }
    if has_changes {
        diff
    } else {
        String::new()
    }
}

/// Ported from: patch/index.ts:307-340 (deriveNewContentsFromChunks)
pub fn derive_new_contents_from_chunks(
    _file_path: &Path,
    chunks: &[UpdateFileChunk],
    original_text: &str,
) -> Result<FileUpdate, ToolError> {
    let (original_bom, original_text_stripped) = bom_split(original_text);

    let mut original_lines: Vec<&str> = original_text_stripped.split('\n').collect();
    if !original_lines.is_empty() && original_lines.last() == Some(&"") {
        original_lines.pop();
    }

    let replacements = compute_replacements(&original_lines, _file_path, chunks)?;
    let mut new_lines = apply_replacements(&original_lines, &replacements);

    if new_lines.is_empty() || new_lines.last() != Some(&String::new()) {
        new_lines.push(String::new());
    }

    let joined = new_lines.join("\n");
    let (next_bom, next_text) = bom_split(&joined);
    let unified_diff = generate_unified_diff(&original_text_stripped, &next_text);

    Ok(FileUpdate {
        unified_diff,
        content: next_text,
        bom: original_bom || next_bom,
    })
}
