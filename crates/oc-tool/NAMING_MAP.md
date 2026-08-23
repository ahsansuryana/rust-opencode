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

## Sprint 5b

| TS asli (path:baris) | TS identifier | Rust identifier | Rust lokasi | Catatan |
|---|---|---|---|---|
| tool/edit.ts:22-33 | `normalizeLineEndings`,`detectLineEnding`,`convertToLineEnding` | fn sama nama (snake_case) | crates/oc-tool/src/edit.rs | - |
| edit.ts:47-56 | `Parameters` | validasi manual di `execute` | crates/oc-tool/src/edit.rs | filePath/oldString/newString/replaceAll |
| edit.ts:244-644 | 9 replacer strategies (`SimpleReplacer`…`MultiOccurrenceReplacer`) + `levenshtein` | `simple_replacer` … `multi_occurrence_replacer`, `levenshtein`, `literal_span` | crates/oc-tool/src/edit.rs | WhitespaceNormalized word-pattern `\s+` direplikasi tanpa regex; threshold similarity identik (0.65) |
| edit.ts:682-737 | `replace`,`isDisproportionateMatch` | `edit::replace`,`is_disproportionate_match` | crates/oc-tool/src/edit.rs | urutan replacer + guard pesan persis |
| edit.ts:646-680 | `trimDiff` | `edit::trim_diff` | crates/oc-tool/src/edit.rs | - |
| edit.ts:58-215 (+write.ts) diff metadata | `createTwoFilesPatch`,`diffLines` | `two_file_patch` (header minimal), `count_changes` (LCS) | crates/oc-tool/src/edit.rs | DEVIASI: patch text bentuk jsdiff penuh tidak direplikasi (metadata saja); angka additions/deletions LCS akurat |
| util/bom.ts | `Bom.readFile/split/join` | `bom_split`,`bom_join` | crates/oc-tool/src/edit.rs | BOM UTF-8 dipertahankan sumber/baru |
| tool/truncate.ts | `Truncate.Service` (`output`,`cleanup`,`write`,`limits`), `MAX_LINES/MAX_BYTES` | `truncate::{output,cleanup,MAX_LINES,MAX_BYTES,truncation_dir}` | crates/oc-tool/src/truncate.rs | hint Task-tool menunggu agent permission (sprint 8) — varian plain dipakai; cleanup scheduler hourly menyusul di CLI runtime |
| truncation-dir.ts | `TRUNCATION_DIR` | `truncation_dir()` | crates/oc-tool/src/truncate.rs | data/tool-output |
| registry.ts:291-303 | filter `tools(model)` gpt-*/oss/gpt-4 | `ToolRegistry::tools_for_model` | crates/oc-tool/src/lib.rs | aturan apply_patch vs edit+write; tool apply_patch sendiri menyusul sprint 6 |

## Sprint 6a

| TS asli (path:baris) | TS identifier | Rust identifier | Rust lokasi | Catatan |
|---|---|---|---|---|
| core/shell.ts:7-19,131-224 | META table, `name`,`ps`,`posix`,`ok`,`acceptable`,`gitbash`,`args` | `shell_detect::{shell_name,ps,posix,acceptable_name,acceptable,gitbash,exec_args,to_kind}` | crates/oc-tool/src/shell_detect.rs | login-script bash/zsh (source rc + cd $1 + eval) direplikasi persis |
| tool/shell.ts:27-66 | `CWD`,`FILES`,`CMD_FILES`,`FLAGS`,`SWITCHES` | konstanta sama nama | crates/oc-tool/src/shell.rs | disalin verbatim |
| shell.ts:220-255 | `preview`,`tail` | `preview`,`tail_text` | crates/oc-tool/src/shell.rs | byte-cap UTF-8 boundary replikasi |
| shell.ts:263-291 | `ask` (scan→permission) | `scan_and_ask` | crates/oc-tool/src/shell.rs | DEVIASI: tokenizer sederhana pengganti tree-sitter bash/powershell; external_directory + pattern + arity prefix tetap |
| shell.ts:293-310,428-595 | `cmd`,`run` | inline di `shell::execute` | crates/oc-tool/src/shell.rs | ring buffer keep=maxBytes*2, overflow→truncation file, tail final, "(no output)", `<shell_metadata>` timeout text persis; abort-signal arm menunggu session sprint 10 |
| shell/prompt.ts | `ShellPrompt.render`, shell.txt | deskripsi = include_str assets/shell.txt | crates/oc-tool/assets/shell.txt | render substitusi ${key} penuh ditunda (sprint-6b) |
| tool/webfetch.ts, websearch.ts | WebFetchTool/WebSearchTool | DITUNDA ke sprint-6b | - | keduanya built-in ✓ (ada di source); butuh provider/exa flags |
