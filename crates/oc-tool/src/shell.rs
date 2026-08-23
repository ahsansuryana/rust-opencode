//! Ported from: packages/opencode/src/tool/shell.ts (run/scan subset) dan
//! core/shell.ts args().
//!
//! DEVIASI tercatat: scan command memakai tokenizer sederhana, BUKAN
//! tree-sitter bash/powershell seperti source asli — pola permission untuk
//! command kompleks bisa berbeda; eksekusi & format output mengikuti source.

use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;

use serde_json::json;

use crate::path_safety::contains_path;
use crate::truncate;
use crate::{resolve, Context, ExecuteResult, ToolDef, ToolError};

const MAX_METADATA_LENGTH: usize = 30_000;
const DEFAULT_TIMEOUT_MS: u64 = 2 * 60 * 1000;

const CWD_COMMANDS: &[&str] = &[
    "cd",
    "chdir",
    "popd",
    "pushd",
    "push-location",
    "set-location",
];
const FILE_COMMANDS: &[&str] = &[
    "cd",
    "chdir",
    "popd",
    "pushd",
    "push-location",
    "set-location",
    "rm",
    "cp",
    "mv",
    "mkdir",
    "touch",
    "chmod",
    "chown",
    "cat",
    "get-content",
    "set-content",
    "add-content",
    "copy-item",
    "move-item",
    "remove-item",
    "new-item",
    "rename-item",
];
const CMD_FILE_COMMANDS: &[&str] = &[
    "copy", "del", "dir", "erase", "md", "mkdir", "move", "rd", "ren", "rename", "rmdir", "type",
];

/// Ported from: shell.ts:220-223 (preview)
fn preview(text: &str) -> String {
    if text.len() <= MAX_METADATA_LENGTH {
        return text.to_string();
    }
    let tail: String = text
        .chars()
        .skip(text.chars().count() - MAX_METADATA_LENGTH)
        .collect();
    format!("...\n\n{tail}")
}

/// Tokenizer sederhana pengganti tree-sitter (deviasi tercatat).
fn tokenize(command: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut quote: Option<char> = None;
    for c in command.chars() {
        if let Some(q) = quote {
            current.push(c);
            if c == q {
                quote = None;
            }
            continue;
        }
        match c {
            '"' | '\'' => {
                quote = Some(c);
                current.push(c);
            }
            c if c.is_whitespace() => {
                if !current.is_empty() {
                    tokens.push(std::mem::take(&mut current));
                }
            }
            _ => current.push(c),
        }
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    tokens
}

/// Ported dari expand()/home(): ~ dan $HOME/PWD.
fn expand_basic(text: &str, cwd: &Path) -> String {
    let unquoted = text.trim_matches(|c| c == '"' || c == '\'');
    let home =
        std::env::var(if cfg!(windows) { "USERPROFILE" } else { "HOME" }).unwrap_or_default();
    let out = unquoted.replace("$HOME", &home).replace('~', &home);
    // $PWD hanya bila token berawalan $PWD
    if let Some(rest) = out.strip_prefix("${PWD}") {
        return format!("{}{rest}", cwd.display());
    }
    out
}

struct Scan {
    dirs: Vec<PathBuf>,
    patterns: Vec<String>,
    always: Vec<String>,
}

/// Padanan collect()+ask() — versi tokenizer (deviasi vs tree-sitter).
fn scan_and_ask(
    ctx: &Context,
    command: &str,
    cwd: &Path,
    ps_shell: bool,
    cmd_kind: bool,
) -> Result<(), ToolError> {
    let mut scan = Scan {
        dirs: Vec::new(),
        patterns: Vec::new(),
        always: Vec::new(),
    };
    let tokens = tokenize(command);
    let first_raw = tokens.first().cloned().unwrap_or_default();
    let cmd = if ps_shell || cmd_kind {
        first_raw.to_lowercase()
    } else {
        first_raw.clone()
    };

    let is_files = FILE_COMMANDS.contains(&cmd.as_str())
        || (cmd_kind && CMD_FILE_COMMANDS.contains(&cmd.as_str()));
    if is_files {
        for arg in tokens.iter().skip(1) {
            let arg_text = arg.as_str();
            if arg_text.starts_with('-')
                || (cmd_kind && arg_text.starts_with('/'))
                || (first_raw == "chmod" && arg_text.starts_with('+'))
            {
                continue;
            }
            let expanded = expand_basic(arg_text, cwd);
            if expanded.starts_with(['?', '*', '[']) {
                continue;
            }
            let resolved = resolve(cwd, &expanded);
            if contains_path(&resolved, &ctx.directory)
                || contains_path(&resolved, &ctx.worktree)
                || resolved == ctx.directory
                || resolved == ctx.worktree
            {
                continue;
            }
            let dir = if resolved.is_dir() {
                resolved
            } else {
                resolved.parent().map(Path::to_path_buf).unwrap_or(resolved)
            };
            if !scan.dirs.contains(&dir) {
                scan.dirs.push(dir);
            }
        }
    }

    if !tokens.is_empty() && !CWD_COMMANDS.contains(&cmd.as_str()) {
        scan.patterns.push(command.to_string());
        let prefix_tokens: Vec<String> = oc_permission::arity::prefix(
            &tokens
                .iter()
                .map(|t| t.trim_matches('"').trim_matches('\'').to_string())
                .collect::<Vec<_>>(),
        );
        scan.always.push(format!("{} *", prefix_tokens.join(" ")));
    }

    if !scan.dirs.is_empty() {
        let globs: Vec<String> = scan
            .dirs
            .iter()
            .map(|dir| format!("{}/{}", dir.display(), "*").replace('\\', "/"))
            .collect();
        let mut metadata = oc_config::v1::OrderedMap::new();
        metadata.insert("command".to_string(), json!(command));
        metadata.insert(
            "directories".to_string(),
            json!(scan
                .dirs
                .iter()
                .map(|d| d.to_string_lossy())
                .collect::<Vec<_>>()),
        );
        ctx.ask("external_directory", globs.clone(), globs.clone(), metadata)?;
    }

    if scan.patterns.is_empty() {
        return Ok(());
    }
    let mut metadata = oc_config::v1::OrderedMap::new();
    metadata.insert("command".to_string(), json!(command));
    ctx.ask("bash", scan.patterns.clone(), scan.always.clone(), metadata)?;
    Ok(())
}

/// Ported from: shell.ts:225-255 (tail)
fn tail_text(text: &str, max_lines: usize, max_bytes: usize) -> (String, bool) {
    let lines: Vec<&str> = text.split('\n').collect();
    if lines.len() <= max_lines && text.len() <= max_bytes {
        return (text.to_string(), false);
    }
    let mut out: Vec<&str> = Vec::new();
    let mut bytes = 0usize;
    for line in lines.iter().rev() {
        if out.len() >= max_lines {
            break;
        }
        let size = line.len() + usize::from(!out.is_empty());
        if bytes + size > max_bytes {
            if out.is_empty() {
                // potong byte-level dengan boundary UTF-8
                let buf = line.as_bytes();
                let mut start = buf.len().saturating_sub(max_bytes);
                while start < buf.len() && (buf[start] & 0xc0) == 0x80 {
                    start += 1;
                }
                out.insert(0, std::str::from_utf8(&buf[start..]).unwrap_or(""));
            }
            break;
        }
        out.insert(0, line);
        bytes += size;
    }
    (out.join("\n"), true)
}

pub fn execute(params: &serde_json::Value, ctx: &Context) -> Result<ExecuteResult, ToolError> {
    let command = params["command"]
        .as_str()
        .ok_or_else(|| ToolError::Message("command is required".to_string()))?;
    let timeout_param = params.get("timeout").and_then(serde_json::Value::as_u64);
    if let Some(negative @ 0) = params
        .get("timeout")
        .and_then(serde_json::Value::as_i64)
        .filter(|v| *v < 0)
    {
        let _ = negative;
        return Err(ToolError::Message(format!(
            "Invalid timeout value: {}. Timeout must be a positive number.",
            params
                .get("timeout")
                .and_then(serde_json::Value::as_i64)
                .unwrap_or(0)
        )));
    }
    let timeout = timeout_param.unwrap_or(DEFAULT_TIMEOUT_MS);

    let shell_config: Option<String> = None; // config.shell menyusul
    let shell = crate::shell_detect::acceptable(shell_config.as_deref());
    let name = crate::shell_detect::shell_name(&shell);
    let ps_shell = crate::shell_detect::ps(&shell);
    let cmd_kind = name == "cmd";

    let cwd = match params.get("workdir").and_then(serde_json::Value::as_str) {
        Some(workdir) => resolve(&ctx.directory, workdir),
        None => ctx.directory.clone(),
    };

    scan_and_ask(ctx, command, &cwd, ps_shell, cmd_kind)?;

    // eksekusi
    use crate::shell_detect::exec_args;
    let args = exec_args(&shell, command, &cwd.to_string_lossy());
    let mut child = std::process::Command::new(&shell)
        .args(&args)
        .current_dir(&cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| ToolError::Message(format!("failed to spawn shell {}: {e}", shell)))?;

    // gabungkan stdout+stderr (handle.all di TS): dua thread → ring buffer
    let keep = truncate::MAX_BYTES * 2;
    let mut list: Vec<(String, usize)> = Vec::new();
    let mut used = 0usize;
    let mut full = String::new();
    let mut file: Option<PathBuf> = None;
    let mut cut = false;
    let mut expired = false;

    let mut stdout = child.stdout.take().expect("stdout");
    let mut stderr = child.stderr.take().expect("stderr");

    let started = std::time::Instant::now();
    let deadline = started + std::time::Duration::from_millis(timeout + 100);

    let mut buffer = [0u8; 8192];
    let mut exited: Option<i32> = None;

    loop {
        // baca non-blocking-ish via poll sederhana: coba read stdout/stderr
        // bergantian dengan timeout total (pendekatan sinkron sederhana).
        let mut chunk = String::new();
        let mut got = false;
        let mut streams: [&mut dyn Read; 2] = [&mut stdout, &mut stderr];
        for stream in streams.iter_mut() {
            match stream.read(&mut buffer) {
                Ok(0) => {}
                Ok(n) => {
                    chunk.push_str(&String::from_utf8_lossy(&buffer[..n]));
                    got = true;
                }
                Err(_) => {}
            }
            if got {
                break;
            }
        }

        if got {
            let size = chunk.len();
            list.push((chunk.clone(), size));
            used += size;
            while used > keep && list.len() > 1 {
                let removed = list.remove(0);
                used -= removed.1;
                cut = true;
            }
            full.push_str(&chunk);
            if full.len() > truncate::MAX_BYTES && file.is_none() {
                let saved = truncate_to_disk(&full)?;
                file = Some(saved);
                cut = true;
                full.clear();
            }
        } else if let Some(status) = child.try_wait()? {
            exited = status.code();
            break;
        } else if std::time::Instant::now() >= deadline {
            expired = true;
            let _ = child.kill();
            let _ = child.wait();
            break;
        } else {
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }

    // drain sisa output setelah exit
    loop {
        let mut chunk = String::new();
        let mut got = false;
        let mut streams: [&mut dyn Read; 2] = [&mut stdout, &mut stderr];
        for stream in streams.iter_mut() {
            match stream.read(&mut buffer) {
                Ok(0) => {}
                Ok(n) => {
                    chunk.push_str(&String::from_utf8_lossy(&buffer[..n]));
                    got = true;
                }
                Err(_) => {}
            }
            if got {
                break;
            }
        }
        if !got {
            break;
        }
        let size = chunk.len();
        list.push((chunk.clone(), size));
        used += size;
        while used > keep && list.len() > 1 {
            let removed = list.remove(0);
            used -= removed.1;
            cut = true;
        }
        if file.is_none() {
            full.push_str(&chunk);
        }
    }
    if full.len() > truncate::MAX_BYTES && file.is_none() {
        let saved = truncate_to_disk(&full)?;
        file = Some(saved);
        cut = true;
        full.clear();
    }

    let code = exited;

    let mut meta_messages: Vec<String> = Vec::new();
    if expired {
        meta_messages.push(format!(
            "shell tool terminated command after exceeding timeout {timeout} ms. If this command is expected to take longer and is not waiting for interactive input, retry with a larger timeout value in milliseconds."
        ));
    }
    let raw: String = list.into_iter().map(|(text, _)| text).collect();
    let (end_text, end_cut) = tail_text(&raw, truncate::MAX_LINES, truncate::MAX_BYTES);
    if end_cut {
        cut = true;
    }
    let final_file = if file.is_none() && end_cut {
        Some(truncate_to_disk(&raw)?)
    } else {
        file
    };

    let mut output = end_text;
    if output.is_empty() {
        output = "(no output)".to_string();
    }
    if cut {
        if let Some(saved) = &final_file {
            output = format!(
                "...output truncated...\n\nFull output saved to: {}\n\n{}",
                saved.display(),
                output
            );
        }
    }
    if !meta_messages.is_empty() {
        output.push_str(&format!(
            "\n\n<shell_metadata>\n{}\n</shell_metadata>",
            meta_messages.join("\n")
        ));
    }

    let last_preview = preview(&output);

    Ok(ExecuteResult {
        title: command.to_string(),
        metadata: json!({
            "output": last_preview,
            "exit": code,
            "truncated": cut,
            "outputPath": final_file.as_ref().map(|p| p.to_string_lossy().into_owned()),
        }),
        output,
    })
}

fn truncate_to_disk(text: &str) -> Result<PathBuf, ToolError> {
    let dir = crate::truncate::truncation_dir();
    std::fs::create_dir_all(&dir)?;
    let file = dir.join(format!(
        "tool_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    std::fs::write(&file, text)?;
    Ok(file)
}

#[allow(dead_code)]
type CtxArc = Arc<Context>;

/// Ported from: tool/shell.ts ShellTool — deskripsi template disalin verbatim;
/// render substitusi ${key} penuh menyusul (prompt.ts).
pub const SHELL_TOOL: ToolDef = ToolDef {
    id: "bash",
    description: include_str!("../assets/shell.txt"),
    execute,
};
