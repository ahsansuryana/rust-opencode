# Naming Map — oc-provider

Format tabel sesuai `rust-opencode-plan/01_NAMING_CONVENTION.md`.
Sprint yang mengisi tabel ini: Sprint 7a (fondasi). 7b transform.ts + 7c HTTP
client menyusul.

## Provider coverage (wajib dilaporkan per instruksi sprint)

| Provider | Status | Alasan |
|---|---|---|
| Anthropic, OpenAI, Google | ditunda ke 7c | butuh HTTP client + SSE streaming; trait didesain di 7a agar tinggal implement |
| 70+ provider lain (models.dev) | stub terstruktur | data model tetap dimuat dari snapshot models.dev; eksekusi return error "not yet supported" |
| npm-installed/custom loader | tidak direplikasi | Rust tidak punya dynamic npm install — semua provider diperlakukan "bundled" sesuai catatan sprint |

## Identifier (Sprint 7a)

| TS asli (path:baris) | TS identifier | Rust identifier | Rust lokasi | Catatan |
|---|---|---|---|---|
| provider.ts:1053-1068 | `Model` (+Cost/Limit/Capabilities/ApiInfo) | `Model`,`Cost`,`Limit`,`Capabilities` | crates/oc-provider/src/lib.rs | `api` = JSON passthrough (npm metadata tak direplikasi — DEVIATIONS) |
| provider.ts:1070-1079 | `Info` (+Source) | `Info`,`Source` | crates/oc-provider/src/lib.rs | - |
| provider.ts:1083-1094 | `ListResult` | `ListResult` | crates/oc-provider/src/lib.rs | - |
| provider.ts:1112-1115 | `defaultModelIDs` | `default_model_ids` | crates/oc-provider/src/lib.rs | - |
| provider.ts:2018-2029 | `priority`, `sort` | `PRIORITY`, `sort_models` | crates/oc-provider/src/lib.rs | sortBy desc(priorityIndex)/asc(latest)/desc(id) persis |
| provider.ts:2033-2040 | `parseModel` | `parse_model` | crates/oc-provider/src/lib.rs | split pertama "/" saja |
| provider.ts:1116-1168 | `ModelNotFoundError`,`InitError`,`NoProvidersError`,`NoModelsError`,`type Error` | `error::{...}` enum `Error` | crates/oc-provider/src/error.rs | pesan message-getter identik |
| provider/auth.ts:11-45,50-54 | `When/TextPrompt/SelectPrompt/Prompt/Method/Methods/Authorization` | `auth::{When,Prompt,Method,Methods,Authorization}` | crates/oc-provider/src/auth.rs | serde tag "type" utk union Prompt |
| auth.ts:68-86 | `OauthMissing`,`OauthCodeMissing`,`OauthCallbackFailed`,`ValidationFailed`,`Error` | `auth::ProviderAuthError` | crates/oc-provider/src/auth.rs | ValidationFailed menunggu prompt.validate hook |
| auth.ts:100-224 | `State/Service` (`methods`,`authorize`,`callback`) | `ProviderAuthService` | crates/oc-provider/src/auth.rs | plugin hooks DITUNDA (methods kosong); callback → oc_auth.set api/oauth ✓ |
