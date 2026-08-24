# Naming Map — oc-mcp

| TS asli | TS identifier | Rust identifier | Catatan |
|---|---|---|---|
| mcp/index.ts | MCP client (stdio) | `McpStdioClient` | spawn/send/read/initialize/list_tools/call_tool/shutdown |
| core/v1/config/mcp.ts | `McpLocal`/`McpRemote` | `McpServerConfig` enum | Local{command,cwd,env,enabled,timeout} / Remote{url,enabled,headers} |
