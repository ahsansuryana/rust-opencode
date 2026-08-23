# Naming Map — oc-storage

Format tabel sesuai `rust-opencode-plan/01_NAMING_CONVENTION.md`.
Sprint yang mengisi tabel ini: Sprint 3.

CATATAN SUMBER: sprint mengasumsikan SQLite; source aktual
(`packages/opencode/src/storage/storage.ts`) adalah penyimpanan file JSON
hierarkis + 2 data-migration + marker file `migration`. Port mengikuti source
aktual (keputusan tercatat di DEVIATIONS.md § technical notes).

## Daftar migrasi (urutan asli)

| # | TS lokasi | Fungsi | Catatan |
|---|---|---|---|
| 1 | storage.ts:82-181 | `Storage.migration.1` — migrasi layout legacy `<data>/project/<dir>/storage/**` ke layout baru `<data>/storage/**` (project/session/message/part), butuh `git rev-list --max-parents=0 --all` untuk menentukan projectID | butuh helper git runner |
| 2 | storage.ts:182-210 | `Storage.migration.2` — pisah `summary.diffs` dari `session/*/*.json` menjadi `session_diff/<id>.json` + ringkasan additions/deletions | - |

## Identifier

| TS asli (path:baris) | TS identifier | Rust identifier | Rust lokasi | Catatan |
|---|---|---|---|---|
| storage.ts:11-17 | `NotFoundError` (+isInstance) | `NotFoundError` | crates/oc-storage/src/lib.rs | pesan `Resource not found: <target>` |
| storage.ts:19 | `type Error` | `Error` (enum Fs / NotFound) | crates/oc-storage/src/lib.rs | FSUtil.Error \| NotFoundError |
| storage.ts:21-51 | `RootFile`,`SessionFile`,`MessageFile`,`DiffFile`,`SummaryFile` + decoder | `RootFile`,`SessionFile`,`MessageFile`,`DiffEntry`,`SummaryFile` (serde) | crates/oc-storage/src/lib.rs | onExcessProperty preserve → serde default ignore extra; None bila gagal |
| storage.ts:53-61 | `Interface`, `Service "@opencode/Storage"` | `StorageService` (struct) | crates/oc-storage/src/lib.rs | method remove/read/update/write/list |
| storage.ts:63-65 | `file(dir,key)` | `file_path(dir,key)` | crates/oc-storage/src/lib.rs | join + ".json" |
| storage.ts:67-74 | `missing(err)` | `is_missing_error` | crates/oc-storage/src/lib.rs | ENOENT/NotFound |
| storage.ts:76-79 | `parseMigration` | `parse_migration` | crates/oc-storage/src/lib.rs | NaN→0 |
| storage.ts:81-211 | `MIGRATIONS` | `run_migrations` + `migration_1`, `migration_2` | crates/oc-storage/src/migrations.rs | urutan & isi dipertahankan step-per-step; marker ditulis tiap sukses |
| storage.ts:213-243 (layer state) | state `{dir}` cached + marker read/run loop | `StorageService::new()` init + `ensure_migrated()` | crates/oc-storage/src/lib.rs | gagal migrasi → logError + break, marker tetap |
| storage.ts:245-249 | `fail`, `wrap` | `not_found`, `wrap_missing` | crates/oc-storage/src/lib.rs | - |
| storage.ts:251-253 | `writeJson` | `write_json_pretty` | crates/oc-storage/src/lib.rs | stringify null,2 + writeWithDirs |
| storage.ts:255-264 | `withResolved` + RcMap/TxReentrantLock | registry `RwLock` per target path | crates/oc-storage/src/lib.rs | simplifikasi konkurensi fiber→thread; didokumentasikan |
| storage.ts:266-299 | `remove`,`read`,`update`,`write` | method sama nama (snake_case) | crates/oc-storage/src/lib.rs | read/update generic T: DeserializeOwned/Serialize |
| storage.ts:301-313 | `list` | `list` | crates/oc-storage/src/lib.rs | glob `**/*` file; sort by join("/") |
| core/fs-util.ts:127-145 | `ensureDir`, `writeWithDirs` | `ensure_dir`, `write_with_dirs` | crates/oc-storage/src/fs_util.rs | tulis langsung BUKAN atomic temp+rename (sesuai source; asumsi sprint salah) |
| core/fs-util.ts:69-77,147-152 | `isDir`, `glob` | `is_dir`, `glob_scan` (mini-matcher) | crates/oc-storage/src/glob_util.rs | pattern `*`/`**` segmen; cukup utk pola yg dipakai migrasi & list |
| @/git `git.run(["rev-list",...])` (dipakai migration 1) | `Git.Interface.run` | helper `git_rev_list_roots(cwd)` | crates/oc-storage/src/migrations.rs | subset minimal; crate git penuh menyusul (folder git/ belum termap sprint) |
