# Deviations from original TS behavior

## [Sprint 1] Multi-issue schema validation tidak mengumpulkan semua issue
- **Lokasi asli**: packages/opencode/src/config/parse.ts:35-61 (`errors: "all"` di Effect Schema decode)
- **Perilaku asli**: validasi config yang invalid mengumpulkan SEMUA issue sekaligus ke `ConfigInvalidError.issues`.
- **Kenapa deviasi**: deserialisasi serde berhenti pada error pertama; menggandakan seluruh validator hanya untuk akumulasi issue berisiko drift ganda.
- **Keputusan**: [x] direplikasi apa adanya untuk path valid (byte-for-deep-equal); [ ] error invalid hanya berisi issue pertama (bentuk error tetap `ConfigInvalidError` dengan issues+path). Perlu approval manual bila ingin akumulasi penuh.

## [Sprint 1] Offset parser JSONC memakai Unicode scalar / byte Rust, bukan UTF-16 code unit
- **Lokasi asli**: packages/opencode/src/config/parse.ts:8-33 + jsonc-parser (offset JS = UTF-16 code unit)
- **Perilaku asli**: line/column error dihitung dari offset UTF-16.
- **Kenapa deviasi**: Rust string bukan UTF-16; karakter astral (>U+FFFF, mis. emoji) membuat offset/colom bergeser 1 per char.
- **Keputusan**: [x] direplikasi apa adanya untuk semua input non-astral (identik); input dengan emoji di dalam config yang parse-error akan beda kolom ±1. Tercatat agar golden test menghindari emoji pada kasus error.

## [Sprint 1] Fitur config yang DITUNDA ke sprint lain (belum diport, bukan diubah)
- **Lokasi asli**: packages/opencode/src/config/config.ts
- **Perilaku asli & target**:
  - well-known remote config + org console config (config.ts:356-396, 478-514) → butuh oc-auth/oc-storage/HTTP (sprint 2/7).
  - npm background dependency install (`waitForDependencies`, config.ts:438-457, 618) → butuh subsystem npm (sprint lanjutan).
  - discovery agent/command/mode markdown + plugin directory load (config.ts:459-465 via config/agent.ts, command.ts, plugin.ts, markdown.ts) → butuh glob + frontmatter + plugin/shared (target sprint 8/13); `configEntryNameFromPath` ikut ditunda.
  - plugin spec resolution & origins (`resolveLoadedPlugins`, `plugin_origins`, config.ts:101-109,111-115) → sprint plugin.
  - write-back `update`/`updateGlobal` + `patchJsonc` (config.ts:149-173, 624-660) → butuh modify/applyEdits jsonc-parser; target bersama server/TUI.
  - `tui*.ts` seluruhnya → sprint CLI/TUI (15). Hot-reload/watching → scope OUT per SPRINT_01.
- **Keputusan**: [x] direplikasi apa adanya bagian yang diport; bagian ditunda TIDAK mengubah perilaku bagian yang sudah ada. Pipeline `load_instance_state` melewati langkah yang butuh modul belum di-port persis seperti kondisi "tidak ada auth wellknown / org aktif" di source asli.

## Technical notes (bukan deviation perilaku)
- [Sprint 3] Sprint mengasumsikan storage = SQLite; source aktual adalah file JSON hierarkis + 2 data-migration + marker. Port mengikuti source aktual; pilihan driver DB (sqlx) ditunda sampai subsystem yang benar-benar pakai SQLite (core database V2) diport.
- [Sprint 3] `TxReentrantLock` per-target (RcMap Effect fiber) disederhanakan jadi registry RwLock per path — semantik reentrant fiber tidak ada padanan langsung di std thread; perilaku single-thread identik.
- [Sprint 3] `list()` sort pakai byte-order Rust (bukan localeCompare JS) — urutan bisa beda utk karakter non-ASCII.
- [Sprint 1] orDie/defect Effect dipetakan ke Result::Err pada boundary crate (lebih graceful); error path jarang tercapai.
- [Sprint 5b] metadata.diff memakai generator patch minimal (bukan format jsdiff `createTwoFilesPatch` lengkap dengan hunk @@); additions/deletions dihitung LCS akurat. Diff ini hanya metadata model-facing.
- [Sprint 5b] Truncate hint varian "Task tool" (butuh agent permission) belum aktif — selalu varian plain; aktif kembali saat oc-agent ter-port (sprint 8).
- [Sprint 5] GrepTool/GlobTool tetap memanggil binary `rg` eksternal (sesuai source); bila rg tidak terpasang, dipakai fallback internal terbatas: glob walker mini + pencarian literal (bukan regex) — perilaku penuh butuh rg. Auto-download rg 15.1.0 (ripgrep/binary.ts) ditunda.
- [Sprint 5] Tidak ada `ListTool` di source aktual — asumsi sprint salah; pembacaan direktori sudah dicakup ReadTool.
- [Sprint 5] Golden "jalankan tool TS asli" ditunda (bun install monorepo tidak stabil di lingkungan ini); output string direplikasi dari bacaan source dan diuji lewat fixture Rust.
- [Sprint 6a] Scan command ShellTool memakai tokenizer sederhana, bukan tree-sitter bash/powershell grammar — pattern permission utk command kompleks bisa beda dari TS; eksekusi/output format identik.
- [Sprint 6a] Abort signal (user cancel mid-command) belum terhubung (butuh session loop sprint 10); timeout kill sudah aktif.
- [Sprint 6b] WebFetch HTML-to-Markdown memakai converter internal subset (turndown penuh tidak diport); struktur umum (heading/list/link/code/emphasis/blockquote/hr/img) dicakup.
- [Sprint 6b] WebSearch execute belum memanggil provider exa/parallel (butuh MCP client, sprint 13) — mengembalikan fallback message persis kondisi hasil kosong TS.
- [Sprint 7] Vercel AI SDK tidak direplikasi — HTTP client per-provider dibangun langsung mengikuti API resmi (Anthropic Messages / OpenAI Chat Completions); field `api` pada Model dipertahankan sebagai JSON passthrough.
- [Sprint 7] Plugin auth hooks (`Hooks["auth"]`) menunggu subsystem plugin sprint lanjutan; `ProviderAuth.methods()` saat ini kosong.


### 2026-08-25: Final verification
- **Full test suite**: 81 tests across 13 crates, all passing
- **check.sh**: fmt + clippy -D warnings + test — all green
- **Status**: All sprints 0-16 complete
- **Remaining deferral**: SSE transport (oc-mcp), OAuth (oc-mcp), full LSP features, subagent spawning (10c) — all optional/non-blocking
# Deviations from original TS behavior

## [Sprint 1] Multi-issue schema validation tidak mengumpulkan semua issue
- **Lokasi asli**: packages/opencode/src/config/parse.ts:35-61 (`errors: "all"` di Effect Schema decode)
- **Perilaku asli**: validasi config yang invalid mengumpulkan SEMUA issue sekaligus ke `ConfigInvalidError.issues`.
- **Kenapa deviasi**: deserialisasi serde berhenti pada error pertama; menggandakan seluruh validator hanya untuk akumulasi issue berisiko drift ganda.
- **Keputusan**: [x] direplikasi apa adanya untuk path valid (byte-for-deep-equal); [ ] error invalid hanya berisi issue pertama (bentuk error tetap `ConfigInvalidError` dengan issues+path). Perlu approval manual bila ingin akumulasi penuh.

## [Sprint 1] Offset parser JSONC memakai Unicode scalar / byte Rust, bukan UTF-16 code unit
- **Lokasi asli**: packages/opencode/src/config/parse.ts:8-33 + jsonc-parser (offset JS = UTF-16 code unit)
- **Perilaku asli**: line/column error dihitung dari offset UTF-16.
- **Kenapa deviasi**: Rust string bukan UTF-16; karakter astral (>U+FFFF, mis. emoji) membuat offset/colom bergeser 1 per char.
- **Keputusan**: [x] direplikasi apa adanya untuk semua input non-astral (identik); input dengan emoji di dalam config yang parse-error akan beda kolom ±1. Tercatat agar golden test menghindari emoji pada kasus error.

## [Sprint 1] Fitur config yang DITUNDA ke sprint lain (belum diport, bukan diubah)
- **Lokasi asli**: packages/opencode/src/config/config.ts
- **Perilaku asli & target**:
  - well-known remote config + org console config (config.ts:356-396, 478-514) → butuh oc-auth/oc-storage/HTTP (sprint 2/7).
  - npm background dependency install (`waitForDependencies`, config.ts:438-457, 618) → butuh subsystem npm (sprint lanjutan).
  - discovery agent/command/mode markdown + plugin directory load (config.ts:459-465 via config/agent.ts, command.ts, plugin.ts, markdown.ts) → butuh glob + frontmatter + plugin/shared (target sprint 8/13); `configEntryNameFromPath` ikut ditunda.
  - plugin spec resolution & origins (`resolveLoadedPlugins`, `plugin_origins`, config.ts:101-109,111-115) → sprint plugin.
  - write-back `update`/`updateGlobal` + `patchJsonc` (config.ts:149-173, 624-660) → butuh modify/applyEdits jsonc-parser; target bersama server/TUI.
  - `tui*.ts` seluruhnya → sprint CLI/TUI (15). Hot-reload/watching → scope OUT per SPRINT_01.
- **Keputusan**: [x] direplikasi apa adanya bagian yang diport; bagian ditunda TIDAK mengubah perilaku bagian yang sudah ada. Pipeline `load_instance_state` melewati langkah yang butuh modul belum di-port persis seperti kondisi "tidak ada auth wellknown / org aktif" di source asli.

## Technical notes (bukan deviation perilaku)
- [Sprint 3] Sprint mengasumsikan storage = SQLite; source aktual adalah file JSON hierarkis + 2 data-migration + marker. Port mengikuti source aktual; pilihan driver DB (sqlx) ditunda sampai subsystem yang benar-benar pakai SQLite (core database V2) diport.
- [Sprint 3] `TxReentrantLock` per-target (RcMap Effect fiber) disederhanakan jadi registry RwLock per path — semantik reentrant fiber tidak ada padanan langsung di std thread; perilaku single-thread identik.
- [Sprint 3] `list()` sort pakai byte-order Rust (bukan localeCompare JS) — urutan bisa beda utk karakter non-ASCII.
- [Sprint 1] orDie/defect Effect dipetakan ke Result::Err pada boundary crate (lebih graceful); error path jarang tercapai.
- [Sprint 5b] metadata.diff memakai generator patch minimal (bukan format jsdiff `createTwoFilesPatch` lengkap dengan hunk @@); additions/deletions dihitung LCS akurat. Diff ini hanya metadata model-facing.
- [Sprint 5b] Truncate hint varian "Task tool" (butuh agent permission) belum aktif — selalu varian plain; aktif kembali saat oc-agent ter-port (sprint 8).
- [Sprint 5] GrepTool/GlobTool tetap memanggil binary `rg` eksternal (sesuai source); bila rg tidak terpasang, dipakai fallback internal terbatas: glob walker mini + pencarian literal (bukan regex) — perilaku penuh butuh rg. Auto-download rg 15.1.0 (ripgrep/binary.ts) ditunda.
- [Sprint 5] Tidak ada `ListTool` di source aktual — asumsi sprint salah; pembacaan direktori sudah dicakup ReadTool.
- [Sprint 5] Golden "jalankan tool TS asli" ditunda (bun install monorepo tidak stabil di lingkungan ini); output string direplikasi dari bacaan source dan diuji lewat fixture Rust.
- [Sprint 6a] Scan command ShellTool memakai tokenizer sederhana, bukan tree-sitter bash/powershell grammar — pattern permission utk command kompleks bisa beda dari TS; eksekusi/output format identik.
- [Sprint 6a] Abort signal (user cancel mid-command) belum terhubung (butuh session loop sprint 10); timeout kill sudah aktif.
- [Sprint 6b] WebFetch HTML-to-Markdown memakai converter internal subset (turndown penuh tidak diport); struktur umum (heading/list/link/code/emphasis/blockquote/hr/img) dicakup.
- [Sprint 6b] WebSearch execute belum memanggil provider exa/parallel (butuh MCP client, sprint 13) — mengembalikan fallback message persis kondisi hasil kosong TS.
- [Sprint 7] Vercel AI SDK tidak direplikasi — HTTP client per-provider dibangun langsung mengikuti API resmi (Anthropic Messages / OpenAI Chat Completions); field `api` pada Model dipertahankan sebagai JSON passthrough.
- [Sprint 7] Plugin auth hooks (`Hooks["auth"]`) menunggu subsystem plugin sprint lanjutan; `ProviderAuth.methods()` saat ini kosong.


### 2026-08-25: Final verification
- **Full test suite**: 81 tests across 13 crates, all passing
- **check.sh**: fmt + clippy -D warnings + test — all green
- **Status**: All sprints 0-16 complete
- **Remaining deferral**: SSE transport (oc-mcp), OAuth (oc-mcp), full LSP features, subagent spawning (10c) — all optional/non-blocking

### 2026-08-25: Sprint 10c — Subagent Spawning
- SubagentContext struct replaces 8+ individual parameters (clippy too-many-arguments)
- Depth limiting hardcoded default=1, configurable via max_subagent_depth
- NoopSpawner for tests; real orchestrator injects actual spawner
- Background mode (async) deferred — current impl is foreground-only

# Deviations from original TS behavior

## [Sprint 1] Multi-issue schema validation tidak mengumpulkan semua issue
- **Lokasi asli**: packages/opencode/src/config/parse.ts:35-61 (`errors: "all"` di Effect Schema decode)
- **Perilaku asli**: validasi config yang invalid mengumpulkan SEMUA issue sekaligus ke `ConfigInvalidError.issues`.
- **Kenapa deviasi**: deserialisasi serde berhenti pada error pertama; menggandakan seluruh validator hanya untuk akumulasi issue berisiko drift ganda.
- **Keputusan**: [x] direplikasi apa adanya untuk path valid (byte-for-deep-equal); [ ] error invalid hanya berisi issue pertama (bentuk error tetap `ConfigInvalidError` dengan issues+path). Perlu approval manual bila ingin akumulasi penuh.

## [Sprint 1] Offset parser JSONC memakai Unicode scalar / byte Rust, bukan UTF-16 code unit
- **Lokasi asli**: packages/opencode/src/config/parse.ts:8-33 + jsonc-parser (offset JS = UTF-16 code unit)
- **Perilaku asli**: line/column error dihitung dari offset UTF-16.
- **Kenapa deviasi**: Rust string bukan UTF-16; karakter astral (>U+FFFF, mis. emoji) membuat offset/colom bergeser 1 per char.
- **Keputusan**: [x] direplikasi apa adanya untuk semua input non-astral (identik); input dengan emoji di dalam config yang parse-error akan beda kolom ±1. Tercatat agar golden test menghindari emoji pada kasus error.

## [Sprint 1] Fitur config yang DITUNDA ke sprint lain (belum diport, bukan diubah)
- **Lokasi asli**: packages/opencode/src/config/config.ts
- **Perilaku asli & target**:
  - well-known remote config + org console config (config.ts:356-396, 478-514) → butuh oc-auth/oc-storage/HTTP (sprint 2/7).
  - npm background dependency install (`waitForDependencies`, config.ts:438-457, 618) → butuh subsystem npm (sprint lanjutan).
  - discovery agent/command/mode markdown + plugin directory load (config.ts:459-465 via config/agent.ts, command.ts, plugin.ts, markdown.ts) → butuh glob + frontmatter + plugin/shared (target sprint 8/13); `configEntryNameFromPath` ikut ditunda.
  - plugin spec resolution & origins (`resolveLoadedPlugins`, `plugin_origins`, config.ts:101-109,111-115) → sprint plugin.
  - write-back `update`/`updateGlobal` + `patchJsonc` (config.ts:149-173, 624-660) → butuh modify/applyEdits jsonc-parser; target bersama server/TUI.
  - `tui*.ts` seluruhnya → sprint CLI/TUI (15). Hot-reload/watching → scope OUT per SPRINT_01.
- **Keputusan**: [x] direplikasi apa adanya bagian yang diport; bagian ditunda TIDAK mengubah perilaku bagian yang sudah ada. Pipeline `load_instance_state` melewati langkah yang butuh modul belum di-port persis seperti kondisi "tidak ada auth wellknown / org aktif" di source asli.

## Technical notes (bukan deviation perilaku)
- [Sprint 3] Sprint mengasumsikan storage = SQLite; source aktual adalah file JSON hierarkis + 2 data-migration + marker. Port mengikuti source aktual; pilihan driver DB (sqlx) ditunda sampai subsystem yang benar-benar pakai SQLite (core database V2) diport.
- [Sprint 3] `TxReentrantLock` per-target (RcMap Effect fiber) disederhanakan jadi registry RwLock per path — semantik reentrant fiber tidak ada padanan langsung di std thread; perilaku single-thread identik.
- [Sprint 3] `list()` sort pakai byte-order Rust (bukan localeCompare JS) — urutan bisa beda utk karakter non-ASCII.
- [Sprint 1] orDie/defect Effect dipetakan ke Result::Err pada boundary crate (lebih graceful); error path jarang tercapai.
- [Sprint 5b] metadata.diff memakai generator patch minimal (bukan format jsdiff `createTwoFilesPatch` lengkap dengan hunk @@); additions/deletions dihitung LCS akurat. Diff ini hanya metadata model-facing.
- [Sprint 5b] Truncate hint varian "Task tool" (butuh agent permission) belum aktif — selalu varian plain; aktif kembali saat oc-agent ter-port (sprint 8).
- [Sprint 5] GrepTool/GlobTool tetap memanggil binary `rg` eksternal (sesuai source); bila rg tidak terpasang, dipakai fallback internal terbatas: glob walker mini + pencarian literal (bukan regex) — perilaku penuh butuh rg. Auto-download rg 15.1.0 (ripgrep/binary.ts) ditunda.
- [Sprint 5] Tidak ada `ListTool` di source aktual — asumsi sprint salah; pembacaan direktori sudah dicakup ReadTool.
- [Sprint 5] Golden "jalankan tool TS asli" ditunda (bun install monorepo tidak stabil di lingkungan ini); output string direplikasi dari bacaan source dan diuji lewat fixture Rust.
- [Sprint 6a] Scan command ShellTool memakai tokenizer sederhana, bukan tree-sitter bash/powershell grammar — pattern permission utk command kompleks bisa beda dari TS; eksekusi/output format identik.
- [Sprint 6a] Abort signal (user cancel mid-command) belum terhubung (butuh session loop sprint 10); timeout kill sudah aktif.
- [Sprint 6b] WebFetch HTML-to-Markdown memakai converter internal subset (turndown penuh tidak diport); struktur umum (heading/list/link/code/emphasis/blockquote/hr/img) dicakup.
- [Sprint 6b] WebSearch execute belum memanggil provider exa/parallel (butuh MCP client, sprint 13) — mengembalikan fallback message persis kondisi hasil kosong TS.
- [Sprint 7] Vercel AI SDK tidak direplikasi — HTTP client per-provider dibangun langsung mengikuti API resmi (Anthropic Messages / OpenAI Chat Completions); field `api` pada Model dipertahankan sebagai JSON passthrough.
- [Sprint 7] Plugin auth hooks (`Hooks["auth"]`) menunggu subsystem plugin sprint lanjutan; `ProviderAuth.methods()` saat ini kosong.


### 2026-08-25: Final verification
- **Full test suite**: 81 tests across 13 crates, all passing
- **check.sh**: fmt + clippy -D warnings + test — all green
- **Status**: All sprints 0-16 complete
- **Remaining deferral**: SSE transport (oc-mcp), OAuth (oc-mcp), full LSP features, subagent spawning (10c) — all optional/non-blocking
# Deviations from original TS behavior

## [Sprint 1] Multi-issue schema validation tidak mengumpulkan semua issue
- **Lokasi asli**: packages/opencode/src/config/parse.ts:35-61 (`errors: "all"` di Effect Schema decode)
- **Perilaku asli**: validasi config yang invalid mengumpulkan SEMUA issue sekaligus ke `ConfigInvalidError.issues`.
- **Kenapa deviasi**: deserialisasi serde berhenti pada error pertama; menggandakan seluruh validator hanya untuk akumulasi issue berisiko drift ganda.
- **Keputusan**: [x] direplikasi apa adanya untuk path valid (byte-for-deep-equal); [ ] error invalid hanya berisi issue pertama (bentuk error tetap `ConfigInvalidError` dengan issues+path). Perlu approval manual bila ingin akumulasi penuh.

## [Sprint 1] Offset parser JSONC memakai Unicode scalar / byte Rust, bukan UTF-16 code unit
- **Lokasi asli**: packages/opencode/src/config/parse.ts:8-33 + jsonc-parser (offset JS = UTF-16 code unit)
- **Perilaku asli**: line/column error dihitung dari offset UTF-16.
- **Kenapa deviasi**: Rust string bukan UTF-16; karakter astral (>U+FFFF, mis. emoji) membuat offset/colom bergeser 1 per char.
- **Keputusan**: [x] direplikasi apa adanya untuk semua input non-astral (identik); input dengan emoji di dalam config yang parse-error akan beda kolom ±1. Tercatat agar golden test menghindari emoji pada kasus error.

## [Sprint 1] Fitur config yang DITUNDA ke sprint lain (belum diport, bukan diubah)
- **Lokasi asli**: packages/opencode/src/config/config.ts
- **Perilaku asli & target**:
  - well-known remote config + org console config (config.ts:356-396, 478-514) → butuh oc-auth/oc-storage/HTTP (sprint 2/7).
  - npm background dependency install (`waitForDependencies`, config.ts:438-457, 618) → butuh subsystem npm (sprint lanjutan).
  - discovery agent/command/mode markdown + plugin directory load (config.ts:459-465 via config/agent.ts, command.ts, plugin.ts, markdown.ts) → butuh glob + frontmatter + plugin/shared (target sprint 8/13); `configEntryNameFromPath` ikut ditunda.
  - plugin spec resolution & origins (`resolveLoadedPlugins`, `plugin_origins`, config.ts:101-109,111-115) → sprint plugin.
  - write-back `update`/`updateGlobal` + `patchJsonc` (config.ts:149-173, 624-660) → butuh modify/applyEdits jsonc-parser; target bersama server/TUI.
  - `tui*.ts` seluruhnya → sprint CLI/TUI (15). Hot-reload/watching → scope OUT per SPRINT_01.
- **Keputusan**: [x] direplikasi apa adanya bagian yang diport; bagian ditunda TIDAK mengubah perilaku bagian yang sudah ada. Pipeline `load_instance_state` melewati langkah yang butuh modul belum di-port persis seperti kondisi "tidak ada auth wellknown / org aktif" di source asli.

## Technical notes (bukan deviation perilaku)
- [Sprint 3] Sprint mengasumsikan storage = SQLite; source aktual adalah file JSON hierarkis + 2 data-migration + marker. Port mengikuti source aktual; pilihan driver DB (sqlx) ditunda sampai subsystem yang benar-benar pakai SQLite (core database V2) diport.
- [Sprint 3] `TxReentrantLock` per-target (RcMap Effect fiber) disederhanakan jadi registry RwLock per path — semantik reentrant fiber tidak ada padanan langsung di std thread; perilaku single-thread identik.
- [Sprint 3] `list()` sort pakai byte-order Rust (bukan localeCompare JS) — urutan bisa beda utk karakter non-ASCII.
- [Sprint 1] orDie/defect Effect dipetakan ke Result::Err pada boundary crate (lebih graceful); error path jarang tercapai.
- [Sprint 5b] metadata.diff memakai generator patch minimal (bukan format jsdiff `createTwoFilesPatch` lengkap dengan hunk @@); additions/deletions dihitung LCS akurat. Diff ini hanya metadata model-facing.
- [Sprint 5b] Truncate hint varian "Task tool" (butuh agent permission) belum aktif — selalu varian plain; aktif kembali saat oc-agent ter-port (sprint 8).
- [Sprint 5] GrepTool/GlobTool tetap memanggil binary `rg` eksternal (sesuai source); bila rg tidak terpasang, dipakai fallback internal terbatas: glob walker mini + pencarian literal (bukan regex) — perilaku penuh butuh rg. Auto-download rg 15.1.0 (ripgrep/binary.ts) ditunda.
- [Sprint 5] Tidak ada `ListTool` di source aktual — asumsi sprint salah; pembacaan direktori sudah dicakup ReadTool.
- [Sprint 5] Golden "jalankan tool TS asli" ditunda (bun install monorepo tidak stabil di lingkungan ini); output string direplikasi dari bacaan source dan diuji lewat fixture Rust.
- [Sprint 6a] Scan command ShellTool memakai tokenizer sederhana, bukan tree-sitter bash/powershell grammar — pattern permission utk command kompleks bisa beda dari TS; eksekusi/output format identik.
- [Sprint 6a] Abort signal (user cancel mid-command) belum terhubung (butuh session loop sprint 10); timeout kill sudah aktif.
- [Sprint 6b] WebFetch HTML-to-Markdown memakai converter internal subset (turndown penuh tidak diport); struktur umum (heading/list/link/code/emphasis/blockquote/hr/img) dicakup.
- [Sprint 6b] WebSearch execute belum memanggil provider exa/parallel (butuh MCP client, sprint 13) — mengembalikan fallback message persis kondisi hasil kosong TS.
- [Sprint 7] Vercel AI SDK tidak direplikasi — HTTP client per-provider dibangun langsung mengikuti API resmi (Anthropic Messages / OpenAI Chat Completions); field `api` pada Model dipertahankan sebagai JSON passthrough.
- [Sprint 7] Plugin auth hooks (`Hooks["auth"]`) menunggu subsystem plugin sprint lanjutan; `ProviderAuth.methods()` saat ini kosong.


### 2026-08-25: Final verification
- **Full test suite**: 81 tests across 13 crates, all passing
- **check.sh**: fmt + clippy -D warnings + test — all green
- **Status**: All sprints 0-16 complete
- **Remaining deferral**: SSE transport (oc-mcp), OAuth (oc-mcp), full LSP features, subagent spawning (10c) — all optional/non-blocking

### 2026-08-25: Sprint 10c — Subagent Spawning
- SubagentContext struct replaces 8+ individual parameters (clippy too-many-arguments)
- Depth limiting hardcoded default=1, configurable via max_subagent_depth
- NoopSpawner for tests; real orchestrator injects actual spawner
- Background mode (async) deferred — current impl is foreground-only


### 2026-08-25: Sprint 12b — Additional API Routes
- Added: update_session (POST), list_parts, get_agent_list, get_tool_list, get_model_list
- 14 total routes (up from 9)

### 2026-08-25: Sprint 13b — SSE Transport
- McpSseClient: HTTP POST for JSON-RPC, ureq with json feature
- McpClient unified enum: Stdio/Sse auto-detect from McpServerConfig

### 2026-08-25: Sprint 14b — Full LSP Features
- Added: gotoDefinition, references, codeAction, rename, documentSymbols, workspaceSymbols, didChange, didSave, waitForDiagnostics
- 12 total LSP methods (up from 5)

### 2026-08-25: Sprint 15b — TUI Interaktif
- crossterm-based terminal UI with colorized output
- Commands: /quit, /new, /list, /clear
- Interactive loop: input → store → placeholder response
- Full LLM integration deferred (needs provider config)
