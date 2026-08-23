//! Ported from: packages/opencode/src/config/variable.ts

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::v1::error::InvalidError;

/// Ported from: packages/opencode/src/config/variable.ts:8-17 (ParseSource)
#[derive(Debug, Clone)]
pub enum ParseSource {
    Path { path: PathBuf },
    Virtual { source: String, dir: PathBuf },
}

/// Ported from: packages/opencode/src/config/variable.ts:19-23 (SubstituteInput)
#[derive(Debug, Clone)]
pub struct SubstituteInput<'a> {
    pub parse_source: &'a ParseSource,
    pub text: String,
    pub missing: MissingPolicy,
    /// Padanan `env?: Record<string,string>`; None = pakai process env.
    pub env: Option<&'a BTreeMap<String, String>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MissingPolicy {
    #[default]
    Error,
    Empty,
}

fn parse_source_source(input: &ParseSource) -> String {
    match input {
        ParseSource::Path { path } => path.to_string_lossy().into_owned(),
        ParseSource::Virtual { source, .. } => source.clone(),
    }
}

fn parse_source_dir(input: &ParseSource) -> PathBuf {
    match input {
        ParseSource::Path { path } => path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from(".")),
        ParseSource::Virtual { dir, .. } => dir.clone(),
    }
}

fn lookup_env(env: Option<&BTreeMap<String, String>>, name: &str) -> Option<String> {
    // TS: input.env?.[varName] ?? process.env[varName]
    if let Some(map) = env {
        if let Some(value) = map.get(name) {
            return Some(value.clone());
        }
    }
    std::env::var(name).ok()
}

/// Ported from: packages/opencode/src/config/variable.ts:34-91 (substitute)
pub fn substitute(input: &SubstituteInput) -> Result<String, InvalidError> {
    let missing = if matches!(input.missing, MissingPolicy::Empty) {
        "empty"
    } else {
        "error"
    };
    let text = replace_env_tokens(&input.text, input.env);

    let file_matches = collect_file_token_spans(&text);
    if file_matches.is_empty() {
        return Ok(text);
    }

    let config_dir = parse_source_dir(input.parse_source);
    let config_source = parse_source_source(input.parse_source);
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::new();
    let mut cursor = 0usize;

    for (index, token) in file_matches {
        out.extend(chars[cursor..index].iter());
        let line_start = chars[..index]
            .iter()
            .rposition(|&c| c == '\n')
            .map(|p| p + 1)
            .unwrap_or(0);
        let prefix: String = chars[line_start..index]
            .iter()
            .collect::<String>()
            .trim_start()
            .to_string();
        if prefix.starts_with("//") {
            out.push_str(&token);
            cursor = index + token.chars().count();
            continue;
        }

        let mut file_path = token
            .strip_prefix("{file:")
            .and_then(|s| s.strip_suffix('}'))
            .unwrap_or("")
            .to_string();
        if let Some(rest) = file_path.strip_prefix("~/") {
            // path.join(os.homedir(), filePath.slice(2))
            let home = std::env::var_os("USERPROFILE")
                .or_else(|| std::env::var_os("HOME"))
                .map(PathBuf::from)
                .unwrap_or_default();
            file_path = home.join(rest).to_string_lossy().into_owned();
        }

        let resolved_path = if Path::new(&file_path).is_absolute() {
            PathBuf::from(&file_path)
        } else {
            normalize_join(&config_dir, &file_path)
        };
        let read = std::fs::read_to_string(&resolved_path);
        let file_content = match read {
            Ok(content) => content.trim().to_string(),
            Err(error) => {
                if missing == "empty" {
                    String::new()
                } else {
                    let err_msg = format!("bad file reference: \"{token}\"");
                    let message = if error.kind() == std::io::ErrorKind::NotFound {
                        format!("{err_msg} {} does not exist", resolved_path.display())
                    } else {
                        err_msg
                    };
                    return Err(InvalidError {
                        path: config_source.clone(),
                        issues: None,
                        message: Some(message),
                    });
                }
            }
        };

        let escaped = serde_json::to_string(&file_content).unwrap_or_else(|_| "\"\"".to_string());
        out.push_str(&escaped[1..escaped.len() - 1]);
        cursor = index + token.chars().count();
    }

    out.extend(chars[cursor..].iter());
    Ok(out)
}

/// Meniru `text.replace(/\{env:([^}]+)\}/g, cb)` — token kosong (`{env:}`)
/// bukan match dan dilewati.
fn replace_env_tokens(text: &str, env: Option<&BTreeMap<String, String>>) -> String {
    let needle = "{env:";
    let mut result = String::with_capacity(text.len());
    let mut rest = text;
    loop {
        let Some(start) = rest.find(needle) else {
            result.push_str(rest);
            break;
        };
        let after = start + needle.len();
        let bytes = rest.as_bytes();
        if after >= bytes.len() || bytes[after] == b'}' {
            result.push_str(&rest[..after]);
            rest = &rest[after..];
            continue;
        }
        let close_rel = rest[after..].find('}');
        let Some(close_rel) = close_rel else {
            result.push_str(rest);
            break;
        };
        let name = &rest[after..after + close_rel];
        let value = lookup_env(env, name).unwrap_or_default();
        result.push_str(&rest[..start]);
        result.push_str(&value);
        rest = &rest[after + close_rel + 1..];
    }
    result
}

/// Meniru Array.from(text.matchAll(/\{file:[^}]+\}/g)) → (offset_char, token).
fn collect_file_token_spans(text: &str) -> Vec<(usize, String)> {
    let needle = "{file:";
    let mut spans = Vec::new();
    let mut rest = text;
    loop {
        let byte_start = text.len() - rest.len();
        let Some(rel) = rest.find(needle) else { break };
        let after = rel + needle.len();
        let bytes = rest.as_bytes();
        if after >= bytes.len() || bytes[after] == b'}' {
            rest = &rest[after..];
            continue;
        }
        let Some(close_rel) = rest[after..].find('}') else {
            break;
        };
        let end = after + close_rel + 1;
        let token = &rest[rel..end];
        // offset dalam satuan char (konsisten dengan pemakaian slice di atas)
        let offset = text[..byte_start + rel].chars().count();
        spans.push((offset, token.to_string()));
        rest = &rest[end..];
    }
    spans
}

fn normalize_join(base: &Path, relative: &str) -> PathBuf {
    // path.resolve(configDir, filePath)
    let mut components: Vec<std::ffi::OsString> = Vec::new();
    let rel = Path::new(relative);
    for comp in base.components() {
        components.push(comp.as_os_str().to_os_string());
    }
    for comp in rel.components() {
        use std::path::Component;
        match comp {
            Component::CurDir => {}
            Component::ParentDir => {
                components.pop();
            }
            other => components.push(other.as_os_str().to_os_string()),
        }
    }
    let mut joined = PathBuf::new();
    for comp in components {
        joined.push(comp);
    }
    joined
}

#[cfg(test)]
mod tests {
    use super::*;

    fn src_path(p: &str) -> ParseSource {
        ParseSource::Path {
            path: PathBuf::from(p),
        }
    }

    fn run(source: ParseSource, text: &str) -> Result<String, InvalidError> {
        substitute(&SubstituteInput {
            parse_source: &source,
            text: text.to_string(),
            missing: MissingPolicy::Error,
            env: None,
        })
    }

    #[test]
    fn env_substitution_uses_map_and_process_env() {
        std::env::set_var("OC_TEST_VAR_PROC", "proc-value");
        let mut map = BTreeMap::new();
        map.insert("OC_TEST_VAR_MAP".to_string(), "map-value".to_string());
        let source = src_path("/tmp/x/opencode.json");
        let out = substitute(&SubstituteInput {
            parse_source: &source,
            text: "{\"a\": \"{env:OC_TEST_VAR_MAP}\", \"b\": \"{env:OC_TEST_VAR_PROC}\", \"c\": \"{env:OC_TEST_MISSING}\"}"
                .to_string(),
            missing: MissingPolicy::Error,
            env: Some(&map),
        })
        .unwrap();
        assert_eq!(out, r#"{"a": "map-value", "b": "proc-value", "c": ""}"#);
    }

    #[test]
    fn empty_env_name_is_not_a_token() {
        let source = src_path("/tmp/x/opencode.json");
        let out = run(source, "{\"a\": \"{env:}\"}").unwrap();
        assert_eq!(out, r#"{"a": "{env:}"}"#);
    }

    #[test]
    fn file_substitution_relative_and_comment_skipped() {
        let root = std::env::temp_dir().join(format!("oc-var-test-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("key.txt"), "  secret-key \n").unwrap();
        let source = src_path(root.join("opencode.json").to_str().unwrap());
        let text = "{\n  // \"{file:key.txt}\"\n  \"key\": \"{file:key.txt}\"\n}";
        let out = run(source, text).unwrap();
        assert_eq!(
            out,
            "{\n  // \"{file:key.txt}\"\n  \"key\": \"secret-key\"\n}"
        );
    }

    #[test]
    fn missing_file_error_message_matches_ts_shape() {
        let source = src_path("/definitely/not/here/opencode.json");
        let err = run(source, "{\"key\": \"{file:nope.txt}\"}").unwrap_err();
        let message = err.message.unwrap();
        assert!(message.starts_with("bad file reference: \"{file:nope.txt}\" "));
        assert!(message.ends_with(" does not exist"));
    }

    #[test]
    fn missing_file_empty_policy_returns_empty_string() {
        let source = src_path("/definitely/not/here/opencode.json");
        let out = substitute(&SubstituteInput {
            parse_source: &source,
            text: "{\"key\": \"{file:nope.txt}\"}".to_string(),
            missing: MissingPolicy::Empty,
            env: None,
        })
        .unwrap();
        assert_eq!(out, "{\"key\": \"\"}");
    }
}
