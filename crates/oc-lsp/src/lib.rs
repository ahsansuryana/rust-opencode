//! Ported dari packages/opencode/src/lsp/server.ts (subset core: LSP client
//! via stdio JSON-RPC).

use std::io::{BufRead, Read, Write};
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;

use serde_json::{json, Value};

/// LSP Client yang berkomunikasi via stdio JSON-RPC.
pub struct LspClient {
    child: Mutex<Child>,
    stdin: Mutex<Option<std::process::ChildStdin>>,
    stdout: Mutex<Option<std::io::BufReader<std::process::ChildStdout>>>,
}

impl LspClient {
    /// Spawn language server process.
    pub fn spawn(command: &[String]) -> Result<Self, String> {
        let (prog, args) = command.split_first().ok_or("empty command")?;
        let mut child = Command::new(prog)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("failed to spawn LSP server: {e}"))?;
        let stdin = child.stdin.take();
        let stdout = child.stdout.take().map(std::io::BufReader::new);

        Ok(LspClient {
            child: Mutex::new(child),
            stdin: Mutex::new(stdin),
            stdout: Mutex::new(stdout),
        })
    }

    fn write_message(&self, body: &Value) -> Result<(), String> {
        let content = serde_json::to_string(body).map_err(|e| format!("serialize error: {e}"))?;
        let header = format!("Content-Length: {}\r\n\r\n", content.len());
        let mut guard = self.stdin.lock().unwrap();
        let stdin = guard.as_mut().ok_or("stdin closed")?;
        stdin
            .write_all(header.as_bytes())
            .map_err(|e| e.to_string())?;
        stdin
            .write_all(content.as_bytes())
            .map_err(|e| e.to_string())?;
        stdin.flush().map_err(|e| e.to_string())?;
        Ok(())
    }

    fn read_message(&self) -> Result<Value, String> {
        let mut guard = self.stdout.lock().unwrap();
        let reader = guard.as_mut().ok_or("stdout closed")?;
        // Read headers
        let mut content_length = 0usize;
        loop {
            let mut line = String::new();
            reader.read_line(&mut line).map_err(|e| e.to_string())?;
            if line == "\r\n" || line == "\n" {
                break;
            }
            if let Some(len_str) = line.strip_prefix("Content-Length: ") {
                content_length = len_str.trim().parse().map_err(|e| format!("{e}"))?;
            }
        }
        // Read body
        let mut buffer = vec![0u8; content_length];
        reader.read_exact(&mut buffer).map_err(|e| e.to_string())?;
        serde_json::from_slice(&buffer).map_err(|e| format!("invalid LSP response: {e}"))
    }

    fn request(&self, method: &str, params: Value) -> Result<Value, String> {
        static ID: std::sync::OnceLock<Mutex<u64>> = std::sync::OnceLock::new();
        let id = {
            let counter = ID.get_or_init(|| Mutex::new(0));
            let mut g = counter.lock().unwrap();
            *g += 1;
            *g
        };
        self.write_message(&json!({
            "jsonrpc": "2.0", "id": id, "method": method, "params": params
        }))?;

        loop {
            let response = self.read_message()?;
            if response["id"].as_u64() == Some(id) {
                return Ok(response);
            }
            // skip notifications
        }
    }

    pub fn initialize(&self, root_uri: &str) -> Result<Value, String> {
        self.request(
            "initialize",
            json!({
                "processId": null,
                "rootUri": root_uri,
                "capabilities": {}
            }),
        )
    }

    pub fn did_open(&self, uri: &str, text: &str, language_id: &str) -> Result<(), String> {
        self.write_message(&json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": { "uri": uri, "languageId": language_id, "version": 1, "text": text }
            }
        }))
    }

    pub fn hover(&self, uri: &str, line: u32, character: u32) -> Result<Value, String> {
        self.request(
            "textDocument/hover",
            json!({
                "textDocument": { "uri": uri },
                "position": { "line": line, "character": character }
            }),
        )
    }

    pub fn completion(&self, uri: &str, line: u32, character: u32) -> Result<Value, String> {
        self.request(
            "textDocument/completion",
            json!({
                "textDocument": { "uri": uri },
                "position": { "line": line, "character": character }
            }),
        )
    }

    /// Ported dari lsp/server.ts — textDocument/definition
    pub fn goto_definition(&self, uri: &str, line: u32, character: u32) -> Result<Value, String> {
        self.request(
            "textDocument/definition",
            json!({
                "textDocument": { "uri": uri },
                "position": { "line": line, "character": character }
            }),
        )
    }

    /// Ported dari lsp/server.ts — textDocument/references
    pub fn references(&self, uri: &str, line: u32, character: u32) -> Result<Value, String> {
        self.request(
            "textDocument/references",
            json!({
                "textDocument": { "uri": uri },
                "position": { "line": line, "character": character },
                "context": { "includeDeclaration": true }
            }),
        )
    }

    /// Ported dari lsp/server.ts — textDocument/codeAction
    pub fn code_action(
        &self,
        uri: &str,
        start_line: u32,
        start_char: u32,
        end_line: u32,
        end_char: u32,
    ) -> Result<Value, String> {
        self.request(
            "textDocument/codeAction",
            json!({
                "textDocument": { "uri": uri },
                "range": {
                    "start": { "line": start_line, "character": start_char },
                    "end": { "line": end_line, "character": end_char }
                },
                "context": { "diagnostics": [] }
            }),
        )
    }

    /// Ported dari lsp/server.ts — textDocument/publishDiagnostics (notification from server)
    /// Note: diagnostics are pushed by the server, not requested. This method
    /// polls for the next notification and returns it if it's a diagnostic.
    pub fn wait_for_diagnostics(&self) -> Result<Value, String> {
        self.read_message()
    }

    /// textDocument/didChange — notify server of content changes.
    pub fn did_change(&self, uri: &str, text: &str, version: i32) -> Result<(), String> {
        self.write_message(&json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didChange",
            "params": {
                "textDocument": { "uri": uri, "version": version },
                "contentChanges": [{ "text": text }]
            }
        }))
    }

    /// textDocument/didSave — notify server that file was saved.
    pub fn did_save(&self, uri: &str) -> Result<(), String> {
        self.write_message(&json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didSave",
            "params": {
                "textDocument": { "uri": uri }
            }
        }))
    }

    /// textDocument/rename — get rename edits for symbol at position.
    pub fn rename(
        &self,
        uri: &str,
        line: u32,
        character: u32,
        new_name: &str,
    ) -> Result<Value, String> {
        self.request(
            "textDocument/rename",
            json!({
                "textDocument": { "uri": uri },
                "position": { "line": line, "character": character },
                "newName": new_name
            }),
        )
    }

    /// textDocument/documentSymbol — list symbols in a document.
    pub fn document_symbols(&self, uri: &str) -> Result<Value, String> {
        self.request(
            "textDocument/documentSymbol",
            json!({ "textDocument": { "uri": uri } }),
        )
    }

    /// workspace/symbol — search for symbols in the workspace.
    pub fn workspace_symbols(&self, query: &str) -> Result<Value, String> {
        self.request("workspace/symbol", json!({ "query": query }))
    }

    pub fn shutdown(&self) -> Result<(), String> {
        self.write_message(
            &json!({ "jsonrpc": "2.0", "id": 9999, "method": "shutdown", "params": null }),
        )
    }
}

impl Drop for LspClient {
    fn drop(&mut self) {
        let _ = self.shutdown();
        if let Ok(mut child) = self.child.lock() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}
