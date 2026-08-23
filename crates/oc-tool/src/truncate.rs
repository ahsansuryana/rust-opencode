//! Ported from: packages/opencode/src/tool/truncate.ts + truncation-dir.ts

use std::path::PathBuf;

use oc_global::global;

/// Ported from: truncate.ts:14-15 (MAX_LINES, MAX_BYTES)
pub const MAX_LINES: usize = 2000;
pub const MAX_BYTES: usize = 50 * 1024;

/// Ported from: truncation-dir.ts (TRUNCATION_DIR = Global.Path.data/tool-output)
pub fn truncation_dir() -> PathBuf {
    global::path().data.join("tool-output")
}

fn tool_id_ascending() -> String {
    // padanan ToolID.ascending() — prefix "tool_" (dipakai filter cleanup)
    static COUNTER: std::sync::OnceLock<std::sync::Mutex<u64>> = std::sync::OnceLock::new();
    let counter = COUNTER.get_or_init(|| std::sync::Mutex::new(0));
    let mut guard = counter.lock().unwrap();
    *guard += 1;
    let millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    format!("tool_{millis:08x}{:04x}", *guard)
}

#[derive(Debug, Clone)]
pub enum TruncateResult {
    Content(String),
    Truncated {
        content: String,
        output_path: PathBuf,
    },
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Options {
    pub max_lines: Option<usize>,
    pub max_bytes: Option<usize>,
    /// true = tail (dari akhir); default head
    pub tail: bool,
}

/// Ported from: truncate.ts:cleanup — hapus file `tool_*` berumur > 7 hari.
pub fn cleanup(now_millis: u64) {
    let dir = truncation_dir();
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return;
    };
    const RETENTION_MILLIS: u64 = 7 * 24 * 60 * 60 * 1000;
    let cutoff = now_millis.saturating_sub(RETENTION_MILLIS);
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if !name.starts_with("tool_") {
            continue;
        }
        let modified = entry
            .metadata()
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_millis() as u64);
        match modified {
            Some(mtime) if mtime >= cutoff => continue,
            None => continue,
            _ => {}
        }
        let _ = std::fs::remove_file(entry.path());
    }
}

fn write_full(text: &str) -> Result<PathBuf, crate::ToolError> {
    let dir = truncation_dir();
    std::fs::create_dir_all(&dir)?;
    let file = dir.join(tool_id_ascending());
    std::fs::write(&file, text)?;
    Ok(file)
}

/// Ported from: truncate.ts output() — kembalikan teks utuh bila muat; kalau
/// tidak, simpan penuh ke truncation dir dan kembalikan preview + hint.
/// Varian hint "Task tool" butuh agent permission (sprint 8) → varian plain.
pub fn output(text: &str, options: Options) -> Result<TruncateResult, crate::ToolError> {
    let max_lines = options.max_lines.unwrap_or(MAX_LINES);
    let max_bytes = options.max_bytes.unwrap_or(MAX_BYTES);
    let lines: Vec<&str> = text.split('\n').collect();
    let total_bytes = text.len();

    if lines.len() <= max_lines && total_bytes <= max_bytes {
        return Ok(TruncateResult::Content(text.to_string()));
    }

    let mut out: Vec<&str> = Vec::new();
    let mut bytes = 0usize;
    let mut hit_bytes = false;

    if !options.tail {
        for (i, line) in lines.iter().enumerate() {
            if i >= max_lines {
                break;
            }
            let size = line.len() + usize::from(i > 0);
            if bytes + size > max_bytes {
                hit_bytes = true;
                break;
            }
            out.push(line);
            bytes += size;
        }
    } else {
        for line in lines.iter().rev() {
            if out.len() >= max_lines {
                break;
            }
            let size = line.len() + usize::from(!out.is_empty());
            if bytes + size > max_bytes {
                hit_bytes = true;
                break;
            }
            out.insert(0, line);
            bytes += size;
        }
    }

    let removed = if hit_bytes {
        total_bytes - bytes
    } else {
        lines.len() - out.len()
    };
    let unit = if hit_bytes { "bytes" } else { "lines" };
    let preview = out.join("\n");
    let file = write_full(text)?;

    let hint = format!(
        "The tool call succeeded but the output was truncated. Full output saved to: {}\nUse Grep to search the full content or Read with offset/limit to view specific sections.",
        file.display()
    );

    let content = if !options.tail {
        format!("{preview}\n\n...{removed} {unit} truncated...\n\n{hint}")
    } else {
        format!("...{removed} {unit} truncated...\n\n{hint}\n\n{preview}")
    };

    Ok(TruncateResult::Truncated {
        content,
        output_path: file,
    })
}
