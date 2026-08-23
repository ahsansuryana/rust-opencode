# Naming Map — oc-auth

Format tabel sesuai `rust-opencode-plan/01_NAMING_CONVENTION.md`.
Sprint yang mengisi tabel ini: Sprint 2.

| TS asli (path:baris) | TS identifier | Rust identifier | Rust lokasi | Catatan |
|---|---|---|---|---|
| packages/opencode/src/auth/index.ts:8 | `OAUTH_DUMMY_KEY` | `OAUTH_DUMMY_KEY` | crates/oc-auth/src/lib.rs | - |
| index.ts:10 | `file` (const modul) | `auth_file()` | crates/oc-auth/src/lib.rs | dievaluasi saat akses (Global.Path.data statik) |
| index.ts:12 | `fail` (helper privat) | inline di pemanggilan | crates/oc-auth/src/lib.rs | closure TS tidak perlu padanan langsung |
| index.ts:14-21 | `Oauth` ("OAuth") | `Info::Oauth` (variant) | crates/oc-auth/src/lib.rs | tag `"type"`; `accountId`/`enterpriseUrl` rename camelCase |
| index.ts:23-27 | `Api` ("ApiAuth") | `Info::Api` (variant) | crates/oc-auth/src/lib.rs | `metadata` = OrderedMap<String,String> |
| index.ts:29-33 | `WellKnown` ("WellKnownAuth") | `Info::WellKnown` (variant) | crates/oc-auth/src/lib.rs | - |
| index.ts:35 | `Info` (union discriminator "type") | `Info` (enum, serde tag="type") | crates/oc-auth/src/lib.rs | - |
| index.ts:38-41 | `AuthError` | `AuthError` (thiserror) | crates/oc-auth/src/lib.rs | cause Option<Value> |
| index.ts:43-48 | `Interface` | `AuthService` (struct, method get/all/set/remove) | crates/oc-auth/src/lib.rs | Effect Service → struct stateless |
| index.ts:50 | `Service` (Context.Service) | `AuthService` | crates/oc-auth/src/lib.rs | nama struct memakai nama class TS |
| index.ts:58-67 | `all` | `AuthService::all` | crates/oc-auth/src/lib.rs | OPENCODE_AUTH_CONTENT override (invalid JSON → jatuh ke file); entri gagal decode dibuang diam (Record.filterMap) |
| index.ts:69-71 | `get` | `AuthService::get` | crates/oc-auth/src/lib.rs | - |
| index.ts:73-81 | `set` | `AuthService::set` | crates/oc-auth/src/lib.rs | normalisasi strip `/+`; hapus `key` & `norm+"/"`; tulis pretty JSON 0600 |
| index.ts:83-89 | `remove` | `AuthService::remove` | crates/oc-auth/src/lib.rs | - |
| core/fs-util.ts:102-114 (`readJson`,`writeJson`) | `readJson`, `writeJson` | helper privat `read_json_value`, `write_json_pretty_chmod` | crates/oc-auth/src/lib.rs | subset yang dipakai auth |
