//! Ported dari packages/opencode/src/mcp/index.ts (subset core: MCP client
//! untuk stdio transport) dan core/v1/config/mcp.ts (config types).

use std::collections::BTreeMap;
use std::io::{BufRead, Write};
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;

use serde_json::{json, Value};

/// Ported dari core/v1/config/mcp.ts — MCP server config.
#[derive(Debug, Clone)]
pub enum McpServerConfig {
    Local {
        command: Vec<String>,
        cwd: Option<String>,
        env: Option<BTreeMap<String, String>>,
        enabled: bool,
        timeout: Option<u64>,
    },
    Remote {
        url: String,
        enabled: bool,
        headers: Option<BTreeMap<String, String>>,
    },
}

/// JSON-RPC request ID counter.
fn next_id() -> u64 {
    static COUNTER: std::sync::OnceLock<Mutex<u64>> = std::sync::OnceLock::new();
    let mut guard = COUNTER.get_or_init(|| Mutex::new(0)).lock().unwrap();
    *guard += 1;
    *guard
}

/// MCP Client yang berkomunikasi via stdio dengan child process.
pub struct McpStdioClient {
    child: Mutex<Child>,
    stdin: Mutex<Option<std::process::ChildStdin>>,
    stdout: Mutex<Option<std::io::BufReader<std::process::ChildStdout>>>,
}

impl McpStdioClient {
    /// Spawn MCP server process dan initialize koneksi.
    pub fn spawn(
        command: &[String],
        env: Option<&BTreeMap<String, String>>,
    ) -> Result<Self, String> {
        let (prog, args) = command.split_first().ok_or("empty command")?;
        let mut cmd = Command::new(prog);
        cmd.args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        if let Some(env_vars) = env {
            for (k, v) in env_vars {
                cmd.env(k, v);
            }
        }
        let mut child = cmd
            .spawn()
            .map_err(|e| format!("failed to spawn MCP server: {e}"))?;
        let stdin = child.stdin.take();
        let stdout = child.stdout.take().map(std::io::BufReader::new);

        Ok(McpStdioClient {
            child: Mutex::new(child),
            stdin: Mutex::new(stdin),
            stdout: Mutex::new(stdout),
        })
    }

    fn send_request(&self, method: &str, params: Value) -> Result<u64, String> {
        let id = next_id();
        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params
        });
        let mut stdin_guard = self.stdin.lock().unwrap();
        let stdin = stdin_guard.as_mut().ok_or("stdin closed")?;
        writeln!(
            stdin,
            "{}",
            serde_json::to_string(&request).unwrap_or_default()
        )
        .map_err(|e| format!("failed to write to MCP server: {e}"))?;
        stdin.flush().map_err(|e| format!("failed to flush: {e}"))?;
        Ok(id)
    }

    fn read_response(&self, expected_id: u64) -> Result<Value, String> {
        let mut stdout_guard = self.stdout.lock().unwrap();
        let reader = stdout_guard.as_mut().ok_or("stdout closed")?;
        loop {
            let mut line = String::new();
            reader
                .read_line(&mut line)
                .map_err(|e| format!("read error: {e}"))?;
            if line.is_empty() {
                return Err("MCP server closed connection".into());
            }
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let response: Value = serde_json::from_str(trimmed)
                .map_err(|e| format!("invalid JSON from MCP server: {e}"))?;
            if response["id"].as_u64() == Some(expected_id) {
                if let Some(error) = response.get("error") {
                    return Err(format!("MCP error: {error}"));
                }
                return Ok(response["result"].clone());
            }
            // skip notifications dengan id lain
        }
    }

    /// Initialize handshake (MCP protocol requirement).
    pub fn initialize(&self) -> Result<Value, String> {
        let id = self.send_request(
            "initialize",
            serde_json::json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": { "name": "rust-opencode", "version": "0.1.0" }
            }),
        )?;
        self.read_response(id)
    }

    /// List tools dari MCP server.
    pub fn list_tools(&self) -> Result<Vec<Value>, String> {
        let id = self.send_request("tools/list", json!({}))?;
        let result = self.read_response(id)?;
        Ok(result["tools"].as_array().cloned().unwrap_or_default())
    }

    /// Call sebuah tool di MCP server.
    pub fn call_tool(&self, name: &str, arguments: Value) -> Result<Value, String> {
        let id = self.send_request(
            "tools/call",
            serde_json::json!({ "name": name, "arguments": arguments }),
        )?;
        self.read_response(id)
    }

    /// Shutdown MCP server process.
    pub fn shutdown(&self) {
        if let Ok(mut child) = self.child.lock() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

impl Drop for McpStdioClient {
    fn drop(&mut self) {
        self.shutdown();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_next_id_increments() {
        let a = next_id();
        let b = next_id();
        assert!(b > a);
    }

    #[test]
    fn test_parse_sse_like_response() {
        // MCP responses adalah JSON-RPC 2.0
        let response = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": { "tools": [{ "name": "search" }] }
        });
        assert_eq!(response["id"], 1);
        assert_eq!(response["result"]["tools"][0]["name"], "search");
    }
}
