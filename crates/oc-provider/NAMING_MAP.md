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

## Sprint 7b

| TS asli (path:baris) | TS identifier | Rust identifier | Rust lokasi | Catatan |
|---|---|---|---|---|
| transform.ts:25-27 | `sanitizeSurrogates` | `transform_messages::sanitize_surrogates` | crates/oc-provider/src/transform_messages.rs | unpaired surrogate → U+FFFD |
| transform.ts:29-39 | `isKimiFamily` | `transform_messages::is_kimi_family` (→ transform.rs) | crates/oc-tool/src/…/transform.rs | host list persis |
| transform.ts:42-98 | `sdkKey` (tabel npm→key) | `transform_messages::sdk_key` | crates/oc-provider/src/transform_messages.rs | 25 mapping verbatim |
| transform.ts:100-357 | `normalizeMessages` (+scrub claude/mistral/deepseek/interleaved) | `transform_messages::normalize_messages` | crates/oc-provider/src/transform_messages.rs | scrub regex direplikasi manual; "Done." sisipan antara tool→user ✓ |
| transform.ts:359-408 | `applyCaching` | DITUNDA ke 7c | - | butuh providerOptions mergeDeep pada Message — struktur ada, wiring menyusul |
| transform.ts:410-446 | `unsupportedParts` | DITUNDA ke 7c | - | butuh capabilities.input modalities array pada Model |
| transform.ts:466-519 | `message()` orchestrator | DITUNDA ke 7c | - | pipeline unsupportedParts → normalizeMessages → applyCaching → sdkKey remap → itemId strip |
| transform.ts:528-572 | `temperature`,`topP`,`topK` | `transform::{temperature,top_p,top_k}` | crates/oc-provider/src/transform.rs | semua branch model-ID persis |
| transform.ts:574-653 | effort constants + `openaiReasoningEfforts`,`openaiCompatibleReasoningEfforts` + gpt5 helpers | fn sama nama (snake_case) | crates/oc-provider/src/transform.rs | tier tier none/minimal/xhigh + release-date gate ✓ |
| transform.ts:655-701 | `anthropicUsesModernAdaptiveThinking`,`anthropicAdaptiveEfforts`,`anthropicOpus45`,`anthropicOmitsThinking`,`googleThinkingLevelEfforts`,`googleThinkingBudgetMax` | pub fn sama nama | crates/oc-provider/src/transform.rs | version-parse claude-(family-)?MAJOR[.MINOR≤2digit] ✓ |
| transform.ts:709-1155 | `variants()` (switch per-npm) | DITUNDA ke 7c (butuh options() juga) | - | ~400 baris switch; port setelah message() selesai |
| transform.ts:1157-1350 | `options()`,`smallOptions` | DITUNDA ke 7c | - | butuh sessionID/providerOptions input type |
| transform.ts:1429-1652 | `sanitizeOpenAISchema`,`schema()` (moonshot/gemini sanitizers) | `transform::{schema,sanitize_openai_schema}` + helper privat | crates/oc-provider/src/transform.rs | boolean→string, const→enum, intent-inference, moonshot $ref+items flatten, gemini enum-to-string/type-array→anyOf/required-filter/non-object-cleanup ✓ |
