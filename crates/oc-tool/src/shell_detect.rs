//! Ported from: packages/core/src/shell.ts (Shell detection & args) dan
//! packages/opencode/src/tool/shell/id.ts (toKind).

use std::path::{Path, PathBuf};

/// Padanan `which` minimal: cari executable di PATH.
pub fn which(command: &str) -> Option<PathBuf> {
    let ext_candidates: Vec<String> = if cfg!(windows) {
        vec![
            format!("{command}.exe"),
            format!("{command}.cmd"),
            command.to_string(),
        ]
    } else {
        vec![command.to_string()]
    };
    let path_var = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path_var) {
        for candidate in &ext_candidates {
            let full = dir.join(candidate);
            if full.is_file() {
                return Some(full);
            }
        }
    }
    None
}

/// Ported from: core/shell.ts gitbash()
pub fn gitbash() -> Option<PathBuf> {
    if !cfg!(windows) {
        return None;
    }
    if let Ok(custom) = std::env::var("OPENCODE_GIT_BASH_PATH") {
        return Some(PathBuf::from(custom));
    }
    let git = which("git")?;
    let file = git.parent()?.parent()?.join("bin").join("bash.exe");
    if file.metadata().map(|m| m.len() > 0).unwrap_or(false) {
        return Some(file);
    }
    None
}

/// Ported from: core/shell.ts name()
pub fn shell_name(file: &str) -> String {
    let normalized = file.replace('\\', "/");
    let base = normalized
        .rsplit('/')
        .next()
        .unwrap_or(&normalized)
        .to_lowercase();
    if cfg!(windows) {
        base.strip_suffix(".exe").unwrap_or(&base).to_string()
    } else {
        base
    }
}

/// Ported from: core/shell.ts ok()
pub fn acceptable_name(name: &str) -> bool {
    !matches!(name, "fish" | "nu")
}

/// Ported from: core/shell.ts ps()
pub fn ps(file: &str) -> bool {
    matches!(shell_name(file).as_str(), "powershell" | "pwsh")
}

/// Ported from: core/shell.ts posix()
pub fn posix(file: &str) -> bool {
    matches!(
        shell_name(file).as_str(),
        "bash" | "dash" | "ksh" | "sh" | "zsh"
    )
}

/// Ported dari win(): kandidat shell di Windows.
fn windows_candidates() -> Vec<String> {
    let mut items: Vec<String> = Vec::new();
    for candidate in [
        which("pwsh").map(|p| p.to_string_lossy().into_owned()),
        which("powershell").map(|p| p.to_string_lossy().into_owned()),
        gitbash().map(|p| p.to_string_lossy().into_owned()),
        std::env::var("COMSPEC").ok(),
    ]
    .into_iter()
    .flatten()
    {
        if !items.contains(&candidate) {
            items.push(candidate);
        }
    }
    items
}

/// Ported dari unix(): baca /etc/shells atau fallback default.
fn unix_candidates() -> Vec<String> {
    if let Ok(text) = std::fs::read_to_string("/etc/shells") {
        let list: Vec<String> = text
            .lines()
            .filter(|line| !line.trim().is_empty() && !line.starts_with('#'))
            .map(str::to_string)
            .collect();
        let mut unique = Vec::new();
        for item in list {
            if !unique.contains(&item) {
                unique.push(item);
            }
        }
        if !unique.is_empty() {
            return unique;
        }
    }
    vec![
        "/bin/bash".to_string(),
        "/bin/zsh".to_string(),
        "/bin/sh".to_string(),
    ]
}

fn fallback() -> String {
    if cfg!(target_os = "macos") {
        return "/bin/zsh".to_string();
    }
    which("bash")
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|| "/bin/sh".to_string())
}

/// Ported dari select(): config override → SHELL env → platform fallback.
pub fn acceptable(config_shell: Option<&str>) -> String {
    if let Some(configured) = config_shell {
        return resolve_shell(configured).unwrap_or_else(fallback);
    }
    let shell_env = std::env::var("SHELL").ok();
    if let Some(shell_env) = shell_env {
        if let Some(resolved) = resolve_shell(&shell_env) {
            return resolved;
        }
    }
    if cfg!(windows) {
        for candidate in windows_candidates() {
            if let Some(resolved) = resolve_shell(&candidate) {
                return resolved;
            }
        }
    } else {
        for candidate in unix_candidates() {
            if let Some(resolved) = resolve_shell(&candidate) {
                return resolved;
            }
        }
    }
    fallback()
}

fn resolve_shell(file: &str) -> Option<String> {
    let name = shell_name(file);
    if !acceptable_name(&name) {
        return None;
    }
    if let Some(found) = which(file) {
        return Some(found.to_string_lossy().into_owned());
    }
    if Path::new(file).is_file() {
        return Some(file.to_string());
    }
    None
}

/// Ported from: core/shell.ts args() — argumen eksekusi per shell.
pub fn exec_args(shell: &str, command: &str, cwd: &str) -> Vec<String> {
    let n = shell_name(shell);
    if n == "nu" || n == "fish" {
        return vec!["-c".into(), command.into()];
    }
    if n == "zsh" {
        return vec![
            "-l".into(),
            "-c".into(),
            format!(
                "\n        [[ -f ~/.zshenv ]] && source ~/.zshenv >/dev/null 2>&1 || true\n        [[ -f \"${{ZDOTDIR:-$HOME}}/.zshrc\" ]] && source \"${{ZDOTDIR:-$HOME}}/.zshrc\" >/dev/null 2>&1 || true\n        cd -- \"$1\"\n        eval {}\n      ",
                serde_json::to_string(command).unwrap_or_default()
            ),
            "opencode".into(),
            cwd.into(),
        ];
    }
    if n == "bash" {
        return vec![
            "-l".into(),
            "-c".into(),
            format!(
                "\n        shopt -s expand_aliases\n        [[ -f ~/.bashrc ]] && source ~/.bashrc >/dev/null 2>&1 || true\n        cd -- \"$1\"\n        eval {}\n      ",
                serde_json::to_string(command).unwrap_or_default()
            ),
            "opencode".into(),
            cwd.into(),
        ];
    }
    if n == "cmd" {
        return vec!["/c".into(), command.into()];
    }
    if ps(shell) {
        return vec!["-NoProfile".into(), "-Command".into(), command.into()];
    }
    vec!["-c".into(), command.into()]
}

/// Ported from: tool/shell/id.ts toKind — kind ringkas utk pemakaian scan.
pub fn to_kind(shell_name_value: &str) -> &'static str {
    match shell_name_value {
        "powershell" | "pwsh" => "powershell",
        "cmd" => "cmd",
        _ => "posix",
    }
}
