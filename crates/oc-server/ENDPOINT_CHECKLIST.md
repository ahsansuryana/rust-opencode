# Endpoint Checklist — openapi.json coverage

## Implemented (Sprint 12)

- [x] GET /health
- [x] POST /session
- [x] GET /session
- [x] GET /session/{sessionID}
- [x] DELETE /session/{sessionID}
- [x] GET /session/{sessionID}/message
- [x] POST /session/{sessionID}/message
- [x] GET /config
- [x] GET /provider

## TODO (untuk sprint lanjutan)

### Session & Messages
- [ ] PATCH /session/{id}
- [ ] GET /session/{id}/children
- [ ] POST /session/{id}/prompt_async
- [ ] POST /session/{id}/abort
- [ ] POST /session/{id}/compact
- [ ] POST /session/{id}/summarize
- [ ] POST /session/{id}/shell
- [ ] POST /session/{id}/revert, /unrevert, /clear
- [ ] DELETE /session/{id}/message/{msgID}
- [ ] PATCH /session/{id}/message/{msgID}/part/{partID}
- [ ] DELETE /session/{id}/message/{msgID}/part/{partID}

### Provider & Auth
- [ ] GET /provider/{providerID}
- [ ] PUT/DELETE /auth/{providerID}
- [ ] POST /provider/{providerID}/oauth/*

### Config
- [ ] PATCH /config

### Tool & File
- [ ] GET /file, /file/content, /file/status
- [ ] GET /find, /find/file, /find/symbol
- [ ] GET /formatter

### Event SSE
- [ ] GET /event (SSE streaming)

### LSP & MCP
- [ ] GET /lsp
- [ ] GET/POST /mcp, MCP auth/connect/disconnect

### PTY
- [ ] Full PTY CRUD + WebSocket connect

### TUI
- [ ] POST /tui/* endpoints

### Experimental
- [ ] /experimental/* endpoints (workspace, worktree, tool, dll)
