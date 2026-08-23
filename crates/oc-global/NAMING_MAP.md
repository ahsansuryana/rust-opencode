# Naming Map — oc-global

Format tabel sesuai `rust-opencode-plan/01_NAMING_CONVENTION.md`.
Sprint yang mengisi tabel ini: Sprint 1.

CATATAN SUMBER: file `packages/opencode/src/global/index.ts` dari MASTER_PLAN
tidak ada lagi di repo saat ini — modul global pindah ke
`packages/core/src/global.ts` (di-import sebagai `@opencode-ai/core/global`).
Port mengikuti lokasi baru tersebut. `Flag` (`packages/core/src/flag/flag.ts`)
ditempatkan di crate ini sebagai `flag.rs` karena tidak ada crate core
tersendiri dan oc-global adalah fondasi; didokumentasikan di sini sesuai
aturan 3 naming convention.

| TS asli (path:baris) | TS identifier | Rust identifier | Rust lokasi | Catatan |
|---|---|---|---|---|
| packages/core/src/global.ts:10 | `app` (const, privat) | `APP` | crates/oc-global/src/global.rs | privat di TS; dipertahankan nama |
| packages/core/src/global.ts:11-15 | `data`,`cache`,`config`,`state`,`tmp` (const modul) | field statis `Paths` dihitung sekali via `init_paths` | crates/oc-global/src/global.rs | evaluasi sekali saat load modul TS ⇒ OnceLock |
| packages/core/src/global.ts:17-29 | `paths` (objek dgn getter `home`) | `Paths` (struct) + `Paths::home()` | crates/oc-global/src/global.rs | `home` getter dievaluasi tiap akses ⇒ method |
| packages/core/src/global.ts:31 | `Path` (export const) | `path()` | crates/oc-global/src/global.rs | akses singleton hasil init |
| packages/core/src/global.ts:35-43 | efek samping import `fs.mkdir(...)` | `ensure_dirs()` | crates/oc-global/src/global.rs | Rust tak punya efek samping import; fungsi eksplisit, dipanggil entry point |
| packages/core/src/global.ts:45 | `Service` (Effect Context.Service) | (tidak diport) | - | infra Effect; diganti pemakaian langsung struct `Interface`; revisit sprint project/session |
| packages/core/src/global.ts:47-57 | `Interface` | `Interface` (struct) | crates/oc-global/src/global.rs | field readonly → field biasa |
| packages/core/src/global.ts:59-72 | `make` | `make` | crates/oc-global/src/global.rs | `Partial<Interface>` input → `InterfaceOverride` struct dgn Option field |
| packages/core/src/global.ts:64 | `Flag.OPENCODE_CONFIG_DIR` | `flag::open_code_config_dir()` | crates/oc-global/src/flag.rs | getter TS ⇒ fn per-call |
| packages/core/src/global.ts:74-85 | `layer`, `node`, `layerWith` | (tidak diport) | - | infra Effect Layer |
| packages/core/src/global.ts:87 | `export * as Global` | crate `oc-global` / module `global` | crates/oc-global/src/lib.rs | namespace projection → module |
| packages/core/src/flag/flag.ts:3-6 | `truthy` | `truthy` | crates/oc-global/src/flag.rs | - |
| packages/core/src/flag/flag.ts:15-78 | `Flag.*` | `flag::OPENCODE_*` (subset) | crates/oc-global/src/flag.rs | PORT SEBAGIAN: hanya flag yg dipakai oc-global/oc-config saat ini (CONFIG, CONFIG_CONTENT, CONFIG_DIR, DISABLE_PROJECT_CONFIG, PERMISSION, DISABLE_AUTOCOMPACT, DISABLE_PRUNE); sisanya ditambahkan sprint berikutnya. Non-getter dievaluasi sekali (OnceLock) sesuai semantik module-load TS; getter dievaluasi per panggilan |
| (infra test) | - | `flag::reset_for_tests`, `global::reset_for_tests` | crates/oc-global/src/{flag.rs,global.rs} | infra khusus test (module JS tak bisa di-reload dgn env baru); bukan port identifier TS |
