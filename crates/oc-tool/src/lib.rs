//! Ported from: packages/opencode/src/tool/tool.ts (framework types) dan
//! tool/external-directory.ts (path safety). Registry minimal generik sesuai
//! scope sprint (resolusi via allowlist — dependency agent ditunda).

pub mod glob;
pub mod grep;
pub mod path_safety;
pub mod read;
pub mod ripgrep;
pub mod write;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use oc_permission::PermissionService;
use serde_json::Value;

/// Ported from: tool/tool.ts:24-34 (InvalidArgumentsError)
#[derive(Debug)]
pub struct InvalidArgumentsError {
    pub tool: String,
    pub detail: String,
}

impl std::fmt::Display for InvalidArgumentsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "The {} tool was called with invalid arguments: {}.\nPlease rewrite the input so it satisfies the expected schema.",
            self.tool, self.detail
        )
    }
}

/// Ported from: tool/tool.ts:36-46 (Context)
/// Effect services (abort signal, metadata callback, messages) menyusul di
/// sprint session; permission ask sudah terhubung ke oc-permission.
#[derive(Clone)]
pub struct Context {
    pub session_id: String,
    pub message_id: String,
    pub agent: String,
    pub directory: PathBuf,
    pub worktree: PathBuf,
    pub bypass_cwd_check: bool,
    pub permission: Arc<PermissionService>,
}

impl Context {
    /// Ported dari `ctx.ask(...)` (tool.ts:45): membentuk AskInput dengan
    /// ruleset kosong + id otomatis.
    pub fn ask(
        &self,
        permission: &str,
        patterns: Vec<String>,
        always: Vec<String>,
        metadata: oc_config::v1::OrderedMap<Value>,
    ) -> Result<(), oc_permission::Error> {
        self.permission.ask(oc_permission::AskInput {
            session_id: self.session_id.clone(),
            permission: permission.to_string(),
            patterns,
            always,
            metadata,
            tool: None,
            id: None,
            ruleset: Vec::new(),
        })
    }
}

/// Ported from: tool/tool.ts:48-53 (ExecuteResult)
#[derive(Debug, Clone)]
pub struct ExecuteResult {
    pub title: String,
    pub metadata: Value,
    pub output: String,
}

/// Error eksekusi tool (padanan Effect.orDie → panic di TS; di sini Result).
#[derive(Debug)]
pub enum ToolError {
    Message(String),
    Permission(oc_permission::Error),
    Io(std::io::Error),
}

impl From<String> for ToolError {
    fn from(value: String) -> Self {
        ToolError::Message(value)
    }
}

impl From<&str> for ToolError {
    fn from(value: &str) -> Self {
        ToolError::Message(value.to_string())
    }
}

impl From<std::io::Error> for ToolError {
    fn from(value: std::io::Error) -> Self {
        ToolError::Io(value)
    }
}

impl From<oc_permission::Error> for ToolError {
    fn from(value: oc_permission::Error) -> Self {
        ToolError::Permission(value)
    }
}

impl std::fmt::Display for ToolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ToolError::Message(m) => write!(f, "{m}"),
            ToolError::Permission(e) => write!(f, "{e}"),
            ToolError::Io(e) => write!(f, "{e}"),
        }
    }
}

/// Padanan `Tool.define()` — def dengan deskripsi verbatim + executor.
pub struct ToolDef {
    pub id: &'static str,
    pub description: &'static str,
    #[allow(clippy::type_complexity)]
    pub execute: fn(params: &Value, ctx: &Context) -> Result<ExecuteResult, ToolError>,
}

impl ToolDef {
    /// Ported dari wrapper decode di tool.ts:113-146: validasi argumen
    /// dilakukan pemanggil (serde) sebelum masuk sini.
    pub fn run(&self, args: Value, ctx: &Context) -> Result<ExecuteResult, InvalidArgumentsError> {
        (self.execute)(&args, ctx).map_err(|error| InvalidArgumentsError {
            tool: self.id.to_string(),
            detail: error.to_string(),
        })
    }
}

/// Ported from: tool/registry.ts (subset sprint ini): resolusi tool
/// berdasarkan daftar nama yang diizinkan.
pub struct ToolRegistry {
    tools: Vec<ToolDef>,
}

impl ToolRegistry {
    pub fn builtin() -> Self {
        ToolRegistry {
            tools: vec![
                read::READ_TOOL,
                write::WRITE_TOOL,
                glob::GLOB_TOOL,
                grep::GREP_TOOL,
            ],
        }
    }

    /// Resolusi tool untuk suatu allowlist nama (filter permission agent —
    /// versi generic per instruksi sprint).
    pub fn resolve(&self, allowed: &[String]) -> Vec<&ToolDef> {
        self.tools
            .iter()
            .filter(|tool| allowed.iter().any(|name| name == tool.id))
            .collect()
    }

    pub fn get(&self, id: &str) -> Option<&ToolDef> {
        self.tools.iter().find(|tool| tool.id == id)
    }

    pub fn all(&self) -> &[ToolDef] {
        &self.tools
    }
}

// --- helper path ---

/// path.isAbsolute padanan lintas platform (drive letter Windows dihitung absolut).
pub fn is_absolute(path: &Path) -> bool {
    if path.is_absolute() {
        return true;
    }
    // "/foo" di Windows dianggap absolute oleh Node
    path.to_string_lossy().starts_with('/')
}

/// path.resolve(directory, maybe_relative)
pub fn resolve(base: &Path, target: &str) -> PathBuf {
    let candidate = Path::new(target);
    if is_absolute(candidate) {
        return candidate.to_path_buf();
    }
    base.join(candidate)
}

/// path.relative(from, to) — komponen-prefix match sederhana.
pub fn relative(from: &Path, to: &Path) -> String {
    let norm = |p: &Path| -> Vec<String> {
        p.components()
            .map(|c| c.as_os_str().to_string_lossy().into_owned())
            .collect()
    };
    let a = norm(from);
    let b = norm(to);
    let mut common = 0usize;
    while common < a.len() && common < b.len() && a[common].eq_ignore_ascii_case(&b[common]) {
        common += 1;
    }
    let mut parts = Vec::new();
    for _ in common..a.len() {
        parts.push("..".to_string());
    }
    parts.extend(b[common..].iter().cloned());
    if parts.is_empty() {
        return ".".to_string();
    }
    parts.join("/")
}
