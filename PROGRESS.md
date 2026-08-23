# Progress

Status: `not-started` / `in-progress` / `done` / `blocked (alasan)`.

| Sprint | Crate | Status | Tanggal | Catatan singkat |
|---|---|---|---|---|
| 0 | (workspace) | done | 2026-08-23 | Bootstrap Cargo workspace + file konvensi; tanpa logic port (murni scaffolding, tidak ada unit/golden test yang relevan) |
| 1 | oc-global, oc-config | done | 2026-08-23 | Port global.ts+flag.ts (subset), config loading/merging/schema ConfigV1 lengkap, variable substitute, JSONC parser port jsonc-parser@3.3.1, managed dir. Unit test 33 + golden test vs paket TS asli (jsonc-parser/remeda/xdg-basedir). DITUNDA (lihat DEVIATIONS.md): jalur remote well-known/org console, npm install, discovery agent/command/plugin markdown, write-back update/updateGlobal, tui*.ts; golden loader TS penuh menunggu bun install monorepo yang tidak stabil di lingkungan ini |
| 2 | oc-auth | not-started | - | - |
| 3 | oc-storage | not-started | - | - |
| 4 | oc-permission | not-started | - | - |
| 5 | oc-tool (bag. 1: filesystem) | not-started | - | - |
| 6 | oc-tool (bag. 2: shell/search/web) | not-started | - | - |
| 7 | oc-provider | not-started | - | - |
| 8 | oc-agent | not-started | - | - |
| 9 | oc-session (bag. 1: data model) | not-started | - | - |
| 10 | oc-session (bag. 2: prompt loop) | not-started | - | - |
| 11 | oc-session (bag. 3: context & compaction) | not-started | - | - |
| 12 | oc-server | not-started | - | - |
| 13 | oc-mcp | not-started | - | - |
| 14 | oc-lsp | not-started | - | - |
| 15 | oc-cli | not-started | - | - |
| 16 | (semua) | not-started | - | - |
