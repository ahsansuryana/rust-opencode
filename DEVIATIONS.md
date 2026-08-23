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
