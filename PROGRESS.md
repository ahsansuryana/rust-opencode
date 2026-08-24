# Progress

Status: `not-started` / `in-progress` / `done` / `blocked (alasan)`.

| Sprint | Crate | Status | Tanggal | Catatan singkat |
|---|---|---|---|---|
| 0 | (workspace) | done | 2026-08-23 | Bootstrap Cargo workspace + file konvensi; tanpa logic port (murni scaffolding, tidak ada unit/golden test yang relevan) |
| 1 | oc-global, oc-config | done | 2026-08-23 | Port global.ts+flag.ts (subset), config loading/merging/schema ConfigV1 lengkap, variable substitute, JSONC parser port jsonc-parser@3.3.1, managed dir. Unit test 33 + golden test vs paket TS asli (jsonc-parser/remeda/xdg-basedir). DITUNDA (lihat DEVIATIONS.md): jalur remote well-known/org console, npm install, discovery agent/command/plugin markdown, write-back update/updateGlobal, tui*.ts; golden loader TS penuh menunggu bun install monorepo yang tidak stabil di lingkungan ini |
| 2 | oc-auth | done | 2026-08-23 | Port auth/index.ts penuh: enum Info (oauth/api/wellknown, tag type), OPENCODE_AUTH_CONTENT override, normalisasi key strip slash, chmod 0600 unix. Test CRUD/normalisasi/round-trip/permission/env-fallback |
| 3 | oc-storage | done | 2026-08-23 | Port storage.ts: file-based hierarchical JSON store (read/write/update/list/remove), marker migration + 2 data-migration lengkap (incl. git rev-list helper). CATATAN: source asli BUKAN SQLite — lihat DEVIATIONS. Test CRUD/list/migrasi golden+idempotensi dengan git repo nyata |
| 4 | oc-permission | done | 2026-08-23 | Port permission/index.ts + arity.ts + wildcard.ts + PermissionV1 contracts: evaluate findLast, ask/reply blocking (Condvar), cascade reject/always per-session, fromConfig expand ~/$HOME, disabled/visibleTools, ARITY table verbatim. 16 unit+integration test, flake-free 8x run |
| 5 | oc-tool (bag. 1: filesystem) | done | 2026-08-23 | 5a+5b lengkap: Edit tool penuh (9 strategi replacer + levenshtein anchor matching + disproportionate guard + trimDiff + BOM/CRLF), truncate (head/tail, cleanup fn), tools_for_model filter gpt-*. 10 test integration stabil. Sisa deferred: LSP diagnostics/format/watcher/instruction/image-attachment, apply_patch tool (sprint 6), golden vs TS runner (env bun) |
| 6 | oc-tool (bag. 2: shell/search/web) | done | 2026-08-23 | 6a DONE: shell_detect (meta/args/login-scripts), ShellTool run (ring-buffer, overflow->truncation dir, tail, timeout+<shell_metadata>), permission scan (tokenizer; tree-sitter deviasi tercatat). 6b DONE: webfetch (HTTP+converter MD subset), websearch helpers+checksum FNV-1a (provider call menunggu MCP/sprint 13), shell_prompt render ${key} penuh. Penutup 6: apply_patch tool penuh (parser codex-patch, seekSequence 4-pass, move/delete) + cygpath workdir resolution. WebSearch provider call menunggu MCP/sprint 13 (fallback message) |
| 7 | oc-provider | in-progress | 2026-08-23 | 7a DONE: tipe Model/Info/ListResult, sort/parse/default_model_ids, error classes (pesan persis), ProviderAuth service (authorize/callback -> oc-auth). 7b: transform.ts (1856 baris). 7c: HTTP client Anthropic/OpenAI/Google + SSE streaming |
| 8 | oc-agent | not-started | - | - |
| 9 | oc-session (bag. 1: data model) | not-started | - | - |
| 10 | oc-session (bag. 2: prompt loop) | not-started | - | - |
| 11 | oc-session (bag. 3: context & compaction) | not-started | - | - |
| 12 | oc-server | not-started | - | - |
| 13 | oc-mcp | not-started | - | - |
| 14 | oc-lsp | not-started | - | - |
| 15 | oc-cli | not-started | - | - |
| 16 | (semua) | not-started | - | - |
