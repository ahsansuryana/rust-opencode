//! Ported dari packages/core/src/ripgrep.ts + ripgrep/binary.ts (subset yang
//! dipakai GlobTool/GrepTool): resolve binary `rg`, jalankan dengan argumen
//! identik, parse output.
//!
//! Keputusan (DEVIATIONS § technical notes): tetap memanggil binary `rg`
//! eksternal seperti source asli; auto-download rg 15.1.0 DITUNDA — bila tidak
//! ada di PATH maupun Global.Path.bin, kembalikan error jelas.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use oc_global::global;
use serde_json::Value;

#[derive(Debug)]
pub enum RipgrepError {
    BinaryNotFound,
    Failed(String),
    InvalidPattern(String),
}

impl std::fmt::Display for RipgrepError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RipgrepError::BinaryNotFound => write!(
                f,
                "ripgrep binary not found (expected `rg` on PATH or in opencode bin dir)"
            ),
            RipgrepError::Failed(message) => write!(f, "{message}"),
            RipgrepError::InvalidPattern(message) => write!(f, "{message}"),
        }
    }
}

/// Ported dari RipgrepBinary.filepath: which("rg") → fallback bin dir.
pub fn filepath() -> Result<PathBuf, RipgrepError> {
    if let Ok(output) = Command::new("rg")
        .arg("--version")
        .stdin(Stdio::null())
        .output()
    {
        if output.status.success() {
            return Ok(PathBuf::from("rg"));
        }
    }
    let paths = [
        global::path().bin.join("rg"),
        global::path().bin.join("rg.exe"),
    ];
    for path in paths {
        if path.exists() {
            return Ok(path);
        }
    }
    Err(RipgrepError::BinaryNotFound)
}

struct RunOutcome {
    code: i32,
    stdout: String,
    stderr: String,
}

fn run(binary: &Path, args: &[String], cwd: &Path) -> Result<RunOutcome, RipgrepError> {
    let mut child = Command::new(binary)
        .args(args)
        .current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| RipgrepError::Failed(error.to_string()))?;

    // Baca stdout penuh lalu stderr (output rg kecil untuk limit 100).
    let stdout = child.stdout.take().expect("stdout piped");
    let stderr = child.stderr.take().expect("stderr piped");
    let out_handle = std::thread::spawn(move || {
        let mut buffer = String::new();
        let mut reader = std::io::BufReader::new(stdout);
        let _ = std::io::Read::read_to_string(&mut reader, &mut buffer);
        buffer
    });
    let err_handle = std::thread::spawn(move || {
        let mut buffer = String::new();
        let mut reader = std::io::BufReader::new(stderr);
        let _ = std::io::Read::read_to_string(&mut reader, &mut buffer);
        buffer
    });
    let status = child
        .wait()
        .map_err(|error| RipgrepError::Failed(error.to_string()))?;
    Ok(RunOutcome {
        code: status.code().unwrap_or(-1),
        stdout: out_handle.join().unwrap_or_default(),
        stderr: err_handle.join().unwrap_or_default(),
    })
}

fn is_invalid_pattern(stderr: &str) -> bool {
    stderr.contains("regex parse error") || stderr.contains("error parsing regex")
}

/// Hasil run generik: items + flag truncated (limit+1 sentinel).
fn run_limited(
    args: &[String],
    cwd: &Path,
    limit: usize,
    pattern: Option<&str>,
) -> Result<(Vec<String>, bool), RipgrepError> {
    let binary = filepath()?;
    let outcome = run(&binary, args, cwd)?;
    let lines: Vec<String> = outcome
        .stdout
        .lines()
        .filter(|l| !l.is_empty())
        .map(str::to_string)
        .collect();
    let truncated = lines.len() > limit;
    let items: Vec<String> = lines.into_iter().take(limit).collect();
    if !truncated && pattern.is_some() && outcome.code == 2 && is_invalid_pattern(&outcome.stderr) {
        return Err(RipgrepError::InvalidPattern(
            outcome.stderr.trim().to_string(),
        ));
    }
    if !truncated && outcome.code != 0 && outcome.code != 1 && outcome.code != 2 {
        return Err(RipgrepError::Failed(if outcome.stderr.trim().is_empty() {
            format!("ripgrep failed with code {}", outcome.code)
        } else {
            outcome.stderr.trim().to_string()
        }));
    }
    let items = if outcome.code == 1 { Vec::new() } else { items };
    Ok((items, truncated))
}

/// Padanan `Ripgrep.glob` — mengembalikan relative path file.
pub fn glob(cwd: &Path, pattern: &str, limit: usize) -> Result<Vec<String>, RipgrepError> {
    let args = vec![
        "--no-config".to_string(),
        "--files".to_string(),
        format!("--glob={pattern}"),
        "--glob=!**/.git/**".to_string(),
        ".".to_string(),
    ];
    let (lines, _) = run_limited(&args, cwd, limit, None)?;
    Ok(lines.iter().map(|line| normalize_relative(line)).collect())
}

/// Baris match hasil `rg --json` setelah dinormalisasi.
#[derive(Debug, Clone)]
pub struct GrepMatch {
    pub entry_path: String,
    pub line_number: u64,
    pub text: String,
}

/// Padanan `Ripgrep.grep`.
pub fn grep(
    cwd: &Path,
    pattern: &str,
    include: Option<&str>,
    limit: usize,
) -> Result<Vec<GrepMatch>, RipgrepError> {
    let mut args = vec![
        "--no-config".to_string(),
        "--json".to_string(),
        "--hidden".to_string(),
        "--no-messages".to_string(),
    ];
    if let Some(include) = include {
        args.push(format!("--glob={include}"));
    }
    args.push("--glob=!**/.git/**".to_string());
    args.push("--".to_string());
    args.push(pattern.to_string());
    args.push(".".to_string());

    let (lines, _) = run_limited(&args, cwd, limit, Some(pattern))?;
    let mut matches = Vec::new();
    for line in &lines {
        let Ok(json) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        if json.get("type").and_then(Value::as_str) != Some("match") {
            continue;
        }
        let Some(data) = json.get("data") else {
            continue;
        };
        let path_text = data["path"]["text"].as_str().unwrap_or_default();
        let line_text = data["lines"]["text"].as_str().unwrap_or_default();
        let line_number = data["line_number"].as_u64().unwrap_or(1);
        let text = truncate_line(line_text);
        matches.push(GrepMatch {
            entry_path: normalize_relative(path_text),
            line_number,
            text,
        });
    }
    Ok(matches)
}

/// Padanan pemotongan 2000 char pada match.lines.text.
fn truncate_line(text: &str) -> String {
    if text.chars().count() > 2_000 {
        let cut: String = text.chars().take(2_000).collect();
        format!("{cut}...")
    } else {
        text.to_string()
    }
}

fn normalize_relative(line: &str) -> String {
    // Port regex TS: ^(?:\.[\\/])+ lalu ^[\\/]+ lalu \\→/
    let mut rest = line;
    loop {
        if let Some(next) = rest.strip_prefix("./") {
            rest = next;
            continue;
        }
        if let Some(next) = rest.strip_prefix(".\\") {
            rest = next;
            continue;
        }
        if let Some(next) = rest.strip_prefix('/') {
            rest = next;
            continue;
        }
        if let Some(next) = rest.strip_prefix('\\') {
            rest = next;
            continue;
        }
        break;
    }
    rest.replace('\\', "/")
}
