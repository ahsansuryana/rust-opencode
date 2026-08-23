# Naming Map — oc-tool

Format tabel sesuai `rust-opencode-plan/01_NAMING_CONVENTION.md`.
Sprint yang mengisi tabel ini: Sprint 5 (bagian 5a; Edit tool menyusul).

CATATAN SUMBER: tidak ada `ListTool` di source aktual (folder `tool/` tidak
memiliki list.ts) — asumsi sprint salah; read.ts menangani direktori.

| TS asli (path:baris) | TS identifier | Rust identifier | Rust lokasi | Catatan |
|---|---|---|---|---|
| tool/tool.ts:24-34 | `InvalidArgumentsError` | `InvalidArgumentsError` | crates/oc-tool/src/lib.rs | pesan model-facing persis |
| tool.ts:36-46 | `Context` | `Context` (struct) | crates/oc-tool/src/lib.rs | ask() terhubung PermissionService; abort/metadata/messages ditunda (session sprint) |
| tool.ts:48-53 | `ExecuteResult` | `ExecuteResult` | crates/oc-tool/src/lib.rs | attachments ditunda |
| tool.ts:55-65,99-149,151-169 | `Def`,`wrap`,`define` | `ToolDef` (+`run` decode wrapper) | crates/oc-tool/src/lib.rs | truncation/LSP hooks ditunda (truncate subsystem) |
| tool/registry.ts | `ToolRegistry` (subset) | `ToolRegistry::builtin/resolve/get/all` | crates/oc-tool/src/lib.rs | resolusi via allowlist generik sesuai scope sprint; permission-arity analysis menyusul sprint 6 |
| tool/read.ts:64-386 | `ReadTool`,`Parameters`,`miss`,`list`,`lines`,`isBinaryFile` | `read::READ_TOOL` + fn pendamping | crates/oc-tool/src/read.rs | format output `<path>/<type>/<content>`, cap 50KB, suffix line-truncate, offset range error, saran miss, listing direktori — direplikasi; LSP warm/instruction/image-pdf attachment DITUNDA |
| tool/write.ts | `WriteTool` | `write::WRITE_TOOL` | crates/oc-tool/src/write.rs | BOM/format/LSP diagnostics/watcher events DITUNDA; metadata.diff placeholder kosong |
| tool/glob.ts | `GlobTool` | `glob::GLOB_TOOL` | crates/oc-tool/src/glob.rs | limit 100, output absolut, truncated note, fallback walker mini saat rg absen |
| tool/grep.ts | `GrepTool` | `grep::GREP_TOOL` | crates/oc-tool/src/grep.rs | format `Found N matches`, `  Line X:`, truncated note; fallback literal-search saat rg absen |
| tool/external-directory.ts | `assertExternalDirectoryEffect` | `path_safety::assert_external_directory` + `contains_path` | crates/oc-tool/src/path_safety.rs | helper shared; glob pattern dir/* untuk ask always |
| core/ripgrep.ts | `Ripgrep.Service` (`glob`,`grep`) | `ripgrep::{glob,grep,GrepMatch}` | crates/oc-tool/src/ripgrep.rs | argumen rg identik (--files/--json/--glob=!**/.git/** dll); InvalidPatternError dipetakan |
| core/ripgrep/binary.ts | `RipgrepBinary.filepath` | `ripgrep::filepath` | crates/oc-tool/src/ripgrep.rs | PATH → Global.Path.bin; AUTO-DOWNLOAD rg 15.1.0 DITUNDA |
| tool/read.txt, write.txt, glob.txt, grep.txt | DESCRIPTION import | `include_str!("../assets/*.txt")` | crates/oc-tool/assets/ | disalin VERBATIM dari repo asli |

## Ditunda ke commit/sprint lanjutan (lihat PROGRESS.md)

| TS asli | Alasan |
|---|---|
| tool/edit.ts (737 baris) | modul terbesar; dikerjakan sebagai sprint-5b agar review per bagian |
| tool/truncate.ts, registry.ts penuh (agent filtering, permission arity) | butuh Agent/Truncate subsystem (sprint 6/8) |
