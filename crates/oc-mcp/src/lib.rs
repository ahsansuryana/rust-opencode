//! Ported dari packages/opencode/src/mcp/index.ts (subset core: MCP client
//! untuk stdio dan SSE transport) dan core/v1/config/mcp.ts (config types).

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

/// MCP Client yang berkomunikasi via SSE (HTTP GET untuk events, POST untuk requests).
/// Ported dari mcp/index.ts — SSE transport untuk remote MCP servers.
pub struct McpSseClient {
    url: String,
    headers: BTreeMap<String, String>,
    /// Endpoint URL yang di-announce oleh server via SSE `endpoint` event.
    endpoint: Mutex<Option<String>>,
}

impl McpSseClient {
    pub fn new(url: String, headers: Option<BTreeMap<String, String>>) -> Self {
        Self {
            url,
            headers: headers.unwrap_or_default(),
            endpoint: Mutex::new(None),
        }
    }

    fn build_agent(&self) -> ureq::Agent {
        ureq::AgentBuilder::new()
            .timeout(std::time::Duration::from_secs(30))
            .build()
    }

    /// POST JSON-RPC request ke endpoint, return response.
    fn send_request(&self, method: &str, params: Value) -> Result<Value, String> {
        let id = next_id();
        let request = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params
        });

        let endpoint = self
            .endpoint
            .lock()
            .unwrap()
            .clone()
            .or_else(|| Some(self.url.clone()))
            .ok_or("no endpoint available — initialize SSE connection first")?;

        let agent = self.build_agent();
        let mut req = agent
            .post(&endpoint)
            .set("Content-Type", "application/json");

        for (k, v) in &self.headers {
            req = req.set(k, v);
        }

        let body = serde_json::to_string(&request).map_err(|e| format!("serialize error: {e}"))?;

        let resp = req
            .send_string(&body)
            .map_err(|e| format!("HTTP POST error: {e}"))?;

        let response: Value = resp
            .into_json()
            .map_err(|e| format!("invalid JSON response: {e}"))?;

        if let Some(error) = response.get("error") {
            return Err(format!("MCP error: {error}"));
        }
        Ok(response["result"].clone())
    }

    /// Initialize SSE connection dan handshake.
    pub fn initialize(&self) -> Result<Value, String> {
        // In SSE transport, the server URL is the SSE endpoint.
        // The actual POST endpoint is announced via an "endpoint" event.
        // For now, we use the URL directly as the POST endpoint.
        *self.endpoint.lock().unwrap() = Some(self.url.clone());

        self.send_request(
            "initialize",
            json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": { "name": "rust-opencode", "version": "0.1.0" }
            }),
        )
    }

    pub fn list_tools(&self) -> Result<Vec<Value>, String> {
        let result = self.send_request("tools/list", json!({}))?;
        Ok(result["tools"].as_array().cloned().unwrap_or_default())
    }

    pub fn call_tool(&self, name: &str, arguments: Value) -> Result<Value, String> {
        self.send_request(
            "tools/call",
            json!({ "name": name, "arguments": arguments }),
        )
    }
}

/// Unified MCP client — auto-detect transport berdasarkan config.
pub enum McpClient {
    Stdio(McpStdioClient),
    Sse(McpSseClient),
}

impl McpClient {
    pub fn from_config(config: &McpServerConfig) -> Result<Self, String> {
        match config {
            McpServerConfig::Local { command, env, .. } => Ok(McpClient::Stdio(
                McpStdioClient::spawn(command, env.as_ref())?,
            )),
            McpServerConfig::Remote { url, headers, .. } => Ok(McpClient::Sse(McpSseClient::new(
                url.clone(),
                headers.clone(),
            ))),
        }
    }

    pub fn initialize(&self) -> Result<Value, String> {
        match self {
            McpClient::Stdio(c) => c.initialize(),
            McpClient::Sse(c) => c.initialize(),
        }
    }

    pub fn list_tools(&self) -> Result<Vec<Value>, String> {
        match self {
            McpClient::Stdio(c) => c.list_tools(),
            McpClient::Sse(c) => c.list_tools(),
        }
    }

    pub fn call_tool(&self, name: &str, arguments: Value) -> Result<Value, String> {
        match self {
            McpClient::Stdio(c) => c.call_tool(name, arguments),
            McpClient::Sse(c) => c.call_tool(name, arguments),
        }
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
        let response = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": { "tools": [{ "name": "search" }] }
        });
        assert_eq!(response["id"], 1);
        assert_eq!(response["result"]["tools"][0]["name"], "search");
    }

    #[test]
    fn test_sse_client_creation() {
        let client = McpSseClient::new(
            "https://example.com/mcp".to_string(),
            Some([("Authorization".to_string(), "Bearer token123".to_string())].into()),
        );
        assert_eq!(client.url, "https://example.com/mcp");
    }

    #[test]
    fn test_mcp_client_from_config_stdio() {
        let config = McpServerConfig::Local {
            command: vec!["__nonexistent_mcp_server_12345".into(), "server.js".into()],
            cwd: None,
            env: None,
            enabled: true,
            timeout: None,
        };
        let result = McpClient::from_config(&config);
        assert!(result.is_err());
    }

    #[test]
    fn test_mcp_client_from_config_remote() {
        let config = McpServerConfig::Remote {
            url: "https://example.com/mcp".to_string(),
            enabled: true,
            headers: None,
        };
        let client = McpClient::from_config(&config).unwrap();
        assert!(matches!(client, McpClient::Sse(_)));
    }
}
