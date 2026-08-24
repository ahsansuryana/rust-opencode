//! Ported from: packages/opencode/src/tool/edit.ts
//! (replacer strategies, replace(), trimDiff, execute flow).

use std::path::Path;

use serde_json::json;

use crate::path_safety::assert_external_directory;
use crate::{relative, resolve, Context, ExecuteResult, ToolDef, ToolError};

// --- helpers line ending & BOM ---

/// Ported from: edit.ts:22-24 (normalizeLineEndings)
fn normalize_line_endings(text: &str) -> String {
    text.replace("\r\n", "\n")
}

/// Ported from: edit.ts:26-28 (detectLineEnding)
fn detect_line_ending(text: &str) -> &'static str {
    if text.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    }
}

/// Ported from: edit.ts:30-33 (convertToLineEnding)
fn convert_to_line_ending(text: &str, ending: &str) -> String {
    if ending == "\n" {
        return text.to_string();
    }
    text.replace('\n', "\r\n")
}

/// Padanan Bom.readFile/Bom.split/Bom.join (util/bom.ts): deteksi BOM UTF-8,
/// proses tanpa BOM, pertahankan bila sumber atau konten baru membawanya.
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

// --- replacers (edit.ts:217-644) ---

type Replacer = fn(&str, &str) -> Vec<String>;

/// Ported from: edit.ts:226-242 (levenshtein)
fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    if a.is_empty() || b.is_empty() {
        return a.len().max(b.len());
    }
    let mut matrix = vec![vec![0usize; b.len() + 1]; a.len() + 1];
    for (i, row) in matrix.iter_mut().enumerate() {
        row[0] = i;
    }
    for (j, cell) in matrix[0].iter_mut().enumerate() {
        *cell = j;
    }
    for i in 1..=a.len() {
        for j in 1..=b.len() {
            let cost = if a[i - 1] == b[j - 1] { 0 } else { 1 };
            matrix[i][j] = (matrix[i - 1][j] + 1)
                .min(matrix[i][j - 1] + 1)
                .min(matrix[i - 1][j - 1] + cost);
        }
    }
    matrix[a.len()][b.len()]
}

/// Ported from: edit.ts:248-286 (LineTrimmedReplacer)
fn line_trimmed_replacer(content: &str, find: &str) -> Vec<String> {
    let original_lines: Vec<&str> = content.split('\n').collect();
    let mut search_lines: Vec<&str> = find.split('\n').collect();
    if search_lines.last() == Some(&"") {
        search_lines.pop();
    }

    let mut results = Vec::new();
    if original_lines.len() < search_lines.len() {
        return results;
    }
    for i in 0..=(original_lines.len() - search_lines.len()) {
        let mut matches = true;
        for (j, search) in search_lines.iter().enumerate() {
            if original_lines[i + j].trim() != search.trim() {
                matches = false;
                break;
            }
        }
        if matches {
            let mut start = 0usize;
            for line in &original_lines[..i] {
                start += line.chars().count() + 1;
            }
            let mut end = start;
            for (k, line) in original_lines[i..i + search_lines.len()].iter().enumerate() {
                end += line.chars().count();
                if k < search_lines.len() - 1 {
                    end += 1;
                }
            }
            // substring by chars
            let collected: String = content.chars().collect::<Vec<char>>()
                [start.min(content.chars().count())..end.min(content.chars().count())]
                .iter()
                .collect();
            results.push(collected);
        }
    }
    results
}

const SINGLE_CANDIDATE_SIMILARITY_THRESHOLD: f64 = 0.65;
const MULTIPLE_CANDIDATES_SIMILARITY_THRESHOLD: f64 = 0.65;

struct Candidate {
    start_line: usize,
    end_line: usize,
}

/// Ported from: edit.ts:288-425 (BlockAnchorReplacer)
fn block_anchor_replacer(content: &str, find: &str) -> Vec<String> {
    let original_lines: Vec<&str> = content.split('\n').collect();
    let mut search_lines: Vec<&str> = find.split('\n').collect();

    if search_lines.len() < 3 {
        return Vec::new();
    }
    if search_lines.last() == Some(&"") {
        search_lines.pop();
    }

    let first_search = search_lines[0].trim();
    let last_search = search_lines[search_lines.len() - 1].trim();
    let search_block_size = search_lines.len();
    let max_line_delta = 1.max(search_block_size / 4);

    let mut candidates: Vec<Candidate> = Vec::new();
    for i in 0..original_lines.len() {
        if original_lines[i].trim() != first_search {
            continue;
        }
        for j in original_lines
            .iter()
            .enumerate()
            .skip(i + 2)
            .map(|(j, _)| j)
        {
            if original_lines[j].trim() == last_search {
                let actual = j - i + 1;
                if actual.abs_diff(search_block_size) <= max_line_delta {
                    candidates.push(Candidate {
                        start_line: i,
                        end_line: j,
                    });
                }
                break;
            }
        }
    }

    if candidates.is_empty() {
        return Vec::new();
    }

    let similarity_for = |candidate: &Candidate| -> f64 {
        let actual = candidate.end_line - candidate.start_line + 1;
        let lines_to_check = (search_block_size - 2).min(actual.saturating_sub(2));
        if lines_to_check == 0 {
            return 1.0;
        }
        let mut similarity = 0.0f64;
        for j in 1..search_block_size - 1 {
            if j >= actual - 1 {
                break;
            }
            let original_line = original_lines[candidate.start_line + j].trim();
            let search_line = search_lines[j].trim();
            let max_len = original_line
                .chars()
                .count()
                .max(search_line.chars().count());
            if max_len == 0 {
                continue;
            }
            let distance = levenshtein(original_line, search_line);
            similarity += (1.0 - distance as f64 / max_len as f64) / lines_to_check as f64;
            if similarity >= SINGLE_CANDIDATE_SIMILARITY_THRESHOLD {
                break;
            }
        }
        similarity
    };

    let yield_candidate = |c: &Candidate| -> String {
        let mut start = 0usize;
        for line in &original_lines[..c.start_line] {
            start += line.chars().count() + 1;
        }
        let mut end = start;
        for line in &original_lines[c.start_line..=c.end_line] {
            end += line.chars().count() + 1;
        }
        end -= 1; // newline hanya antar baris
        let chars: Vec<char> = content.chars().collect();
        chars[start.min(chars.len())..end.min(chars.len())]
            .iter()
            .collect()
    };

    if candidates.len() == 1 {
        let c = &candidates[0];
        if similarity_for(c) >= SINGLE_CANDIDATE_SIMILARITY_THRESHOLD {
            return vec![yield_candidate(c)];
        }
        return Vec::new();
    }

    let mut best: Option<(&Candidate, f64)> = None;
    for candidate in &candidates {
        // multi-candidate memakai rata-rata (tanpa early-exit), sesuai TS
        let actual = candidate.end_line - candidate.start_line + 1;
        let lines_to_check = (search_block_size - 2).min(actual.saturating_sub(2));
        let mut similarity = 0.0f64;
        if lines_to_check > 0 {
            for j in 1..search_block_size - 1 {
                if j >= actual - 1 {
                    break;
                }
                let original_line = original_lines[candidate.start_line + j].trim();
                let search_line = search_lines[j].trim();
                let max_len = original_line
                    .chars()
                    .count()
                    .max(search_line.chars().count());
                if max_len == 0 {
                    continue;
                }
                let distance = levenshtein(original_line, search_line);
                similarity += 1.0 - distance as f64 / max_len as f64;
            }
            similarity /= lines_to_check as f64;
        } else {
            similarity = 1.0;
        }
        if best.map(|(_, s)| similarity > s).unwrap_or(true) {
            best = Some((candidate, similarity));
        }
    }
    if let Some((best_match, score)) = best {
        if score >= MULTIPLE_CANDIDATES_SIMILARITY_THRESHOLD {
            return vec![yield_candidate(best_match)];
        }
    }
    Vec::new()
}

fn normalize_whitespace(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Ported from: edit.ts:427-469 (WhitespaceNormalizedReplacer) — subset tanpa
/// regex word-pattern (kasus substring multi-kata mengandung regex khusus
/// di-skip; kasus utama normalized-line equality tercakup).
fn whitespace_normalized_replacer(content: &str, find: &str) -> Vec<String> {
    let normalized_find = normalize_whitespace(find);
    let mut results = Vec::new();
    for line in content.split('\n') {
        if normalize_whitespace(line) == normalized_find {
            results.push(line.to_string());
        } else if normalize_whitespace(line).contains(&normalized_find) {
            // substring: ambil span literal kata-kata `find` dalam line asli
            if let Some(span) = literal_span(line, find) {
                results.push(span);
            }
        }
    }
    // multi-line blocks
    let find_line_count = find.split('\n').count();
    if find_line_count > 1 {
        let lines: Vec<&str> = content.split('\n').collect();
        if lines.len() >= find_line_count {
            for i in 0..=(lines.len() - find_line_count) {
                let block = lines[i..i + find_line_count].join("\n");
                if normalize_whitespace(&block) == normalized_find {
                    results.push(block);
                }
            }
        }
    }
    results
}

fn literal_span(line: &str, find: &str) -> Option<String> {
    // Padanan regex TS: kata-kata find digabung dengan \s+ di antaranya.
    let words: Vec<&str> = find.split_whitespace().collect();
    if words.is_empty() {
        return None;
    }
    let mut search_from = 0usize;
    while let Some(rel) = line[search_from..].find(words[0]) {
        let start = search_from + rel;
        let mut cursor = start + words[0].len();
        let mut ok = true;
        for word in &words[1..] {
            // \s+ wajib minimal satu spasi sebelum kata berikutnya
            if !line[cursor..].starts_with(|c: char| c.is_whitespace()) {
                ok = false;
                break;
            }
            let after_ws = cursor + line[cursor..].len() - line[cursor..].trim_start().len();
            match line[after_ws..].find(word) {
                Some(gap)
                    if line[after_ws..after_ws + gap]
                        .chars()
                        .all(|c| c.is_whitespace()) =>
                {
                    cursor = after_ws + gap + word.len();
                }
                _ => {
                    ok = false;
                    break;
                }
            }
        }
        if ok {
            return Some(line[start..cursor].to_string());
        }
        search_from += rel + 1;
    }
    None
}

/// Ported from: edit.ts:471-497 (IndentationFlexibleReplacer)
fn remove_indentation(text: &str) -> String {
    let lines: Vec<&str> = text.split('\n').collect();
    let min_indent = lines
        .iter()
        .filter(|l| !l.trim().is_empty())
        .map(|l| l.len() - l.trim_start().len())
        .min();
    let Some(min_indent) = min_indent else {
        return text.to_string();
    };
    lines
        .iter()
        .map(|line| {
            if line.trim().is_empty() {
                (*line).to_string()
            } else {
                line.chars().skip(min_indent).collect()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn indentation_flexible_replacer(content: &str, find: &str) -> Vec<String> {
    let normalized_find = remove_indentation(find);
    let content_lines: Vec<&str> = content.split('\n').collect();
    let find_count = find.split('\n').count();
    let mut results = Vec::new();
    if content_lines.len() >= find_count {
        for i in 0..=(content_lines.len() - find_count) {
            let block = content_lines[i..i + find_count].join("\n");
            if remove_indentation(&block) == normalized_find {
                results.push(block);
            }
        }
    }
    results
}

/// Ported from: edit.ts:499-546 (EscapeNormalizedReplacer)
fn unescape_string(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let chars: Vec<char> = input.chars().collect();
    let mut i = 0usize;
    while i < chars.len() {
        if chars[i] == '\\' && i + 1 < chars.len() {
            match chars[i + 1] {
                'n' => {
                    out.push('\n');
                    i += 2;
                    continue;
                }
                't' => {
                    out.push('\t');
                    i += 2;
                    continue;
                }
                'r' => {
                    out.push('\r');
                    i += 2;
                    continue;
                }
                '\'' | '"' | '`' | '$' | '\\' => {
                    out.push(chars[i + 1]);
                    i += 2;
                    continue;
                }
                '\n' => {
                    out.push('\n');
                    i += 2;
                    continue;
                }
                _ => {}
            }
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

fn escape_normalized_replacer(content: &str, find: &str) -> Vec<String> {
    let unescaped_find = unescape_string(find);
    let mut results = Vec::new();
    if content.contains(&unescaped_find) {
        results.push(unescaped_find.clone());
    }
    let lines: Vec<&str> = content.split('\n').collect();
    let find_count = unescaped_find.split('\n').count();
    if lines.len() >= find_count {
        for i in 0..=(lines.len() - find_count) {
            let block = lines[i..i + find_count].join("\n");
            if unescape_string(&block) == unescaped_find {
                results.push(block);
            }
        }
    }
    results
}

/// Ported from: edit.ts:562-586 (TrimmedBoundaryReplacer)
fn trimmed_boundary_replacer(content: &str, find: &str) -> Vec<String> {
    let trimmed = find.trim();
    if trimmed == find {
        return Vec::new();
    }
    let mut results = Vec::new();
    if content.contains(trimmed) {
        results.push(trimmed.to_string());
    }
    let lines: Vec<&str> = content.split('\n').collect();
    let find_count = find.split('\n').count();
    if lines.len() >= find_count {
        for i in 0..=(lines.len() - find_count) {
            let block = lines[i..i + find_count].join("\n");
            if block.trim() == trimmed {
                results.push(block);
            }
        }
    }
    results
}

/// Ported from: edit.ts:588-644 (ContextAwareReplacer)
fn context_aware_replacer(content: &str, find: &str) -> Vec<String> {
    let mut find_lines: Vec<&str> = find.split('\n').collect();
    if find_lines.len() < 3 {
        return Vec::new();
    }
    if find_lines.last() == Some(&"") {
        find_lines.pop();
    }
    let content_lines: Vec<&str> = content.split('\n').collect();
    let first_line = find_lines[0].trim();
    let last_line = find_lines[find_lines.len() - 1].trim();

    for i in 0..content_lines.len() {
        if content_lines[i].trim() != first_line {
            continue;
        }
        for j in (i + 2)..content_lines.len() {
            if content_lines[j].trim() == last_line {
                let block_lines = &content_lines[i..=j];
                if block_lines.len() == find_lines.len() {
                    let mut matching = 0usize;
                    let mut total_non_empty = 0usize;
                    for k in 1..block_lines.len() - 1 {
                        let block_line = block_lines[k].trim();
                        let find_line = find_lines[k].trim();
                        if !block_line.is_empty() || !find_line.is_empty() {
                            total_non_empty += 1;
                            if block_line == find_line {
                                matching += 1;
                            }
                        }
                    }
                    if total_non_empty == 0 || matching * 2 >= total_non_empty {
                        return vec![block_lines.join("\n")];
                    }
                }
                break;
            }
        }
    }
    Vec::new()
}

/// Ported from: edit.ts:548-560 (MultiOccurrenceReplacer)
fn multi_occurrence_replacer(content: &str, find: &str) -> Vec<String> {
    let mut results = Vec::new();
    let mut start = 0usize;
    while let Some(index) = content[start..].find(find) {
        results.push(find.to_string());
        start += index + find.len();
    }
    results
}

// --- replace() orchestration (edit.ts:682-729) ---

const REPLACERS: &[Replacer] = &[
    simple_replacer,
    line_trimmed_replacer,
    block_anchor_replacer,
    whitespace_normalized_replacer,
    indentation_flexible_replacer,
    escape_normalized_replacer,
    trimmed_boundary_replacer,
    context_aware_replacer,
    multi_occurrence_replacer,
];

fn simple_replacer(_content: &str, find: &str) -> Vec<String> {
    vec![find.to_string()]
}

/// Ported from: edit.ts:731-737 (isDisproportionateMatch)
fn is_disproportionate_match(search: &str, old_string: &str) -> bool {
    let old_lines = old_string.split('\n').count();
    let search_lines = search.split('\n').count();
    if search_lines >= (old_lines + 3).max(old_lines * 2) {
        return true;
    }
    if old_lines == 1 {
        return false;
    }
    search.trim().chars().count()
        > (old_string.trim().chars().count() + 500).max(old_string.trim().chars().count() * 4)
}

/// Ported from: edit.ts:682-729 (replace)
pub fn replace(
    content: &str,
    old_string: &str,
    new_string: &str,
    replace_all: bool,
) -> Result<String, ToolError> {
    if old_string == new_string {
        return Err(ToolError::Message(
            "No changes to apply: oldString and newString are identical.".to_string(),
        ));
    }
    if old_string.is_empty() {
        return Err(ToolError::Message(
            "oldString cannot be empty when editing an existing file. Provide the exact text to replace, or use write for an intentional full-file replacement.".to_string(),
        ));
    }

    let mut not_found = true;

    for replacer in REPLACERS {
        for search in replacer(content, old_string) {
            let index = match content.find(&search) {
                Some(index) => index,
                None => continue,
            };
            not_found = false;
            if is_disproportionate_match(&search, old_string) {
                return Err(ToolError::Message(
                    "Refusing replacement because the matched span is much larger than oldString. Re-read the file and provide the full exact oldString for the intended replacement.".to_string(),
                ));
            }
            if replace_all {
                return Ok(content.replace(&search, new_string));
            }
            let last_index = content.rfind(&search).unwrap();
            if index != last_index {
                continue;
            }
            return Ok(format!(
                "{}{new_string}{}",
                &content[..index],
                &content[index + search.len()..]
            ));
        }
    }

    if not_found {
        return Err(ToolError::Message(
            "Could not find oldString in the file. It must match exactly, including whitespace, indentation, and line endings.".to_string(),
        ));
    }
    Err(ToolError::Message(
        "Found multiple matches for oldString. Provide more surrounding context to make the match unique."
            .to_string(),
    ))
}

// --- diff metadata (jsdiff createTwoFilesPatch + diffLines subset) ---

/// Hitungan additions/deletions padanan diffLines (LCS per baris).
fn count_changes(old_text: &str, new_text: &str) -> (u64, u64) {
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

/// Padanan createTwoFilesPatch + trimDiff — header minimal + hunk konten.
pub fn two_file_patch(old_text: &str, new_text: &str) -> String {
    let mut lines = vec!["--- a/file".to_string(), "+++ b/file".to_string()];
    // unified-ish: semua -old lalu +new (tanpa alignment LCS penuh; cukup untuk
    // metadata diff — catatan di NAMING_MAP)
    for line in old_text.split('\n') {
        if !old_text.is_empty() {
            lines.push(format!("-{line}"));
        }
    }
    for line in new_text.split('\n') {
        if !new_text.is_empty() {
            lines.push(format!("+{line}"));
        }
    }
    trim_diff(&lines.join("\n"))
}

/// Ported from: edit.ts:646-680 (trimDiff)
pub fn trim_diff(diff: &str) -> String {
    let lines: Vec<&str> = diff.split('\n').collect();
    fn is_content(line: &&str) -> bool {
        (line.starts_with('+') || line.starts_with('-') || line.starts_with(' '))
            && !line.starts_with("---")
            && !line.starts_with("+++")
    }
    let content_lines: Vec<&&str> = lines.iter().filter(|l| is_content(l)).collect();
    if content_lines.is_empty() {
        return diff.to_string();
    }
    let mut min = usize::MAX;
    for line in &content_lines {
        let content = &line[1..];
        if !content.trim().is_empty() {
            let indent = content.len() - content.trim_start().len();
            min = min.min(indent);
        }
    }
    if min == usize::MAX || min == 0 {
        return diff.to_string();
    }
    lines
        .iter()
        .map(|line| {
            if is_content(line) {
                let prefix = &line[0..1];
                let content: String = line.chars().skip(1).collect();
                let stripped: String = content.chars().skip(min).collect();
                format!("{prefix}{stripped}")
            } else {
                (*line).to_string()
            }
        })
        .collect::<Vec<_>>()
        .join(
            "
",
        )
}

// --- execute ---

pub fn execute(params: &serde_json::Value, ctx: &Context) -> Result<ExecuteResult, ToolError> {
    let file_path_param = params["filePath"]
        .as_str()
        .ok_or_else(|| ToolError::Message("filePath is required".to_string()))?;
    if file_path_param.is_empty() {
        return Err(ToolError::Message("filePath is required".to_string()));
    }
    let old_string = params["oldString"]
        .as_str()
        .ok_or_else(|| ToolError::Message("oldString is required".to_string()))?;
    let new_string = params["newString"]
        .as_str()
        .ok_or_else(|| ToolError::Message("newString is required".to_string()))?;
    let replace_all = params
        .get("replaceAll")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);

    if old_string == new_string {
        return Err(ToolError::Message(
            "No changes to apply: oldString and newString are identical.".to_string(),
        ));
    }

    let file_path = resolve(&ctx.directory, file_path_param);
    assert_external_directory(ctx, Some(&file_path), false, false)?;

    let existed_before = file_path.exists();
    let (content_old_raw, source_bom) = if old_string.is_empty() {
        if existed_before {
            return Err(ToolError::Message(
                "oldString cannot be empty when editing an existing file. Provide the exact text to replace, or use write for an intentional full-file replacement.".to_string(),
            ));
        }
        (String::new(), false)
    } else {
        let raw = std::fs::read_to_string(&file_path)
            .map_err(|_| ToolError::Message(format!("File {} not found", file_path.display())))?;
        if file_path.is_dir() {
            return Err(ToolError::Message(format!(
                "Path is a directory, not a file: {}",
                file_path.display()
            )));
        }
        let (bom, text) = bom_split(&raw);
        (text, bom)
    };

    let content_new = if old_string.is_empty() {
        let (next_bom, next_text) = bom_split(new_string);
        bom_join(&next_text, next_bom)
    } else {
        let ending = detect_line_ending(&content_old_raw);
        let old = convert_to_line_ending(&normalize_line_endings(old_string), ending);
        let replacement = convert_to_line_ending(&normalize_line_endings(new_string), ending);
        let replaced = replace(&content_old_raw, &old, &replacement, replace_all)?;
        let (next_bom, next_text) = bom_split(&replaced);
        bom_join(&next_text, source_bom || next_bom)
    };

    ctx.ask(
        "edit",
        vec![relative(&ctx.worktree, &file_path)],
        vec!["*".to_string()],
        {
            let mut metadata = oc_config::v1::OrderedMap::new();
            metadata.insert("filepath".to_string(), json!(file_path.to_string_lossy()));
            metadata.insert(
                "diff".to_string(),
                json!(two_file_patch(&content_old_raw, &content_new)),
            );
            metadata
        },
    )?;

    if let Some(parent) = file_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&file_path, &content_new)?;

    let (additions, deletions) = count_changes(&content_old_raw, &content_new);

    Ok(ExecuteResult {
        title: relative(&ctx.worktree, &file_path),
        output: "Edit applied successfully.".to_string(),
        metadata: json!({
            "diagnostics": {},
            "diff": two_file_patch(&content_old_raw, &content_new),
            "filediff": {
                "file": file_path.to_string_lossy(),
                "patch": two_file_patch(&content_old_raw, &content_new),
                "additions": additions,
                "deletions": deletions,
            },
        }),
    })
}

#[allow(dead_code)]
fn unused(_: &Path) {}

/// Ported from: tool/edit.ts:58-215 + DESCRIPTION edit.txt (verbatim).
pub const EDIT_TOOL: ToolDef = ToolDef {
    id: "edit",
    description: include_str!("../assets/edit.txt"),
    execute,
};
