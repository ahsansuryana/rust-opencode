# Naming Map — oc-config

Format tabel sesuai `rust-opencode-plan/01_NAMING_CONVENTION.md`.
Sprint yang mengisi tabel ini: Sprint 1.

CATATAN SUMBER: schema `ConfigV1.Info` kini tinggal di package monorepo lain
(`packages/core/src/v1/config/*`, di-import sebagai `@opencode-ai/core/v1/config/*`).
Karena belum ada crate core tersendiri, file tsb dipetakan ke submodule
`v1/` di dalam oc-config (aturan 3 naming convention: pemecahan didaftarkan
di sini). File loader pendamping (`agent.ts`, `command.ts`, `plugin.ts`,
`markdown.ts`, `tui*.ts`) DITUNDA (butuh glob/yaml/plugin subsystem — lihat
DEVIATIONS.md / PROGRESS.md).

## Loader utama — packages/opencode/src/config/config.ts

| TS asli (path:baris) | TS identifier | Rust identifier | Rust lokasi | Catatan |
|---|---|---|---|---|
| config.ts:41-43 | `mergeConfig` (privat) | `merge_config` | crates/oc-config/src/config.rs | privat; perilaku = remeda mergeDeep |
| config.ts:45-51 | `mergeConfigConcatArrays` (privat) | `merge_config_concat_arrays` | crates/oc-config/src/config.rs | - |
| config.ts:53-62 | `normalizeLoadedConfig` (privat) | `normalize_loaded_config` | crates/oc-config/src/config.rs | - |
| config.ts:64-99 | `substituteWellKnownRemoteConfig` (privat) | (ditunda) | - | hanya dipakai jalur well-known remote (butuh Auth) |
| config.ts:101-109 | `resolveLoadedPlugins` (privat) | (ditunda) | - | butuh plugin/shared (sprint plugin/mcp) |
| config.ts:111-115 | `type Info = ConfigV1.Info & {plugin_origins?}` | `Info` | crates/oc-config/src/v1/config.rs | `plugin_origins` ditunda (derived state utk runtime plugin) |
| config.ts:124-135 | `Interface`, `Service`, `use` | `Interface` (struct), service fns bebas | crates/oc-config/src/config.rs | Effect Service → fungsi atas `ConfigState` |
| config.ts:139-147 | `globalConfigFile` (privat) | `global_config_file` | crates/oc-config/src/config.rs | - |
| config.ts:149-161 | `patchJsonc` (privat) | (ditunda) | - | butuh modify/applyEdits jsonc-parser (jalur updateGlobal) |
| config.ts:163-166 | `writable` (privat) | (ditunda) | - | jalur update() |
| config.ts:168-173 | `writableGlobal` (privat) | (ditunda) | - | jalur updateGlobal() |
| config.ts:213-237 | `loadConfig` (internal) | `load_config` | crates/oc-config/src/config.rs | tanpa resolveLoadedPlugins (ditunda); $schema injection + write-back ikut diport |
| config.ts:239-244 | `loadFile` (internal) | `load_file` | crates/oc-config/src/config.rs | - |
| config.ts:246-279 | `loadGlobal` (internal) | `load_global` | crates/oc-config/src/config.rs | incl. migrasi legacy TOML `config` |
| config.ts:281-293 | `cachedGlobal`/`invalidateGlobal`/`getGlobal` | `get_global` (+ cache Mutex) | crates/oc-config/src/config.rs | Effect.cachedInvalidateWithTTL(∞) → memoize |
| config.ts:295-312 | `ensureGitignore` (internal) | `ensure_gitignore` | crates/oc-config/src/config.rs | - |
| config.ts:314-598 | `loadInstanceState` (internal) | `load_instance_state` | crates/oc-config/src/config.rs | PORT SEBAGIAN deterministik: well-known remote, org console, npm install, agent/command/plugin markdown discovery, plugin origins = DITUNDA (lihat DEVIATIONS). Sisanya direplikasi persis |
| config.ts:600-635 | `state`, `get`, `directories`, `invalidate` | `get`, `directories`, `invalidate` | crates/oc-config/src/config.rs | InstanceState multi-directory → cache tunggal (deviasi tercatat) |
| config.ts:618-622 | `waitForDependencies` | (ditunda) | - | butuh npm install |
| config.ts:624-631 | `update` | (ditunda) | - | write-back + patchJsonc |
| config.ts:637-660 | `updateGlobal` | (ditunda) | - | write-back + patchJsonc |
| packages/opencode/src/config/paths.ts:10-21 | `files` | `files` | crates/oc-config/src/paths.rs | incl. `.toReversed()` |
| paths.ts:23-41 | `directories` | `directories` | crates/oc-config/src/paths.rs | incl. remeda `unique` order-preserving |
| paths.ts:43-45 | `fileInDirectory` | `file_in_directory` | crates/oc-config/src/paths.rs | - |
| packages/opencode/src/config/parse.ts:8-33 | `jsonc` | `jsonc` | crates/oc-config/src/parse.rs | parser JSONC = port algoritma vscode jsonc-parser@3.3.1 (scanner+visit+parse), error codes identik |
| parse.ts:35-61 | `schema` | `schema::from_value::<Info>` wrapper `schema_decode` | crates/oc-config/src/parse.rs | onExcessProperty ignore, errors-all TIDAK penuh (serde first-error) — deviasi tercatat |
| packages/opencode/src/config/variable.ts:34-91 | `substitute` | `substitute` | crates/oc-config/src/variable.ts→variable.rs | `{env:}` `{file:}`, skip baris komentar `//`, `~/`, missing error/empty |
| variable.ts:8-27 | `ParseSource`,`SubstituteInput`,`source`,`dir` (privat) | `ParseSource`,`SubstituteInput`,`parse_source_source`,`parse_source_dir` | crates/oc-config/src/variable.rs | nama fn pembantu diberi prefix agar jelas; dicatat |
| packages/opencode/src/config/entry-name.ts:15-19 | `configEntryNameFromPath` | (ditunda) | - | dipakai discovery agent/command markdown |
| packages/opencode/src/config/managed.ts:20-41 | `systemManagedConfigDir`,`managedConfigDir`,`parseManagedPlist` | `system_managed_config_dir`,`managed_config_dir`,`parse_managed_plist` | crates/oc-config/src/managed.rs | - |
| managed.ts:43-69 | `readManagedPreferences` | `read_managed_preferences` | crates/oc-config/src/managed.rs | darwin-only (plutil subprocess), None di platform lain |
| packages/core/src/fs-util.ts:58-67,154-182 | `existsSafe`,`readFileStringSafe`,`findUp`,`up` (helper FSUtil) | helper di `fs_util` modul internal | crates/oc-config/src/fs_util.rs | subset yang dipakai jalur config; FSUtil lengkap milik crate lain nanti |

## Schema — packages/core/src/v1/config/*

| TS asli (path:baris) | TS identifier | Rust identifier | Rust lokasi | Catatan |
|---|---|---|---|---|
| v1/config/config.ts:22-25 | `WellKnown` | (ditunda) | - | jalur remote |
| v1/config/config.ts:32-190 | `Info` (Schema.Struct "Config") | `Info` | crates/oc-config/src/v1/config.rs | semua field top-level direpresentasikan; excess property ignored |
| v1/config/config.ts:27-30 | `LogLevelRef` ("DEBUG"\|"INFO"\|"WARN"\|"ERROR") | `LogLevel` | crates/oc-config/src/v1/config.rs | serde exact uppercase strings |
| v1/config/agent.ts:7-88 | `Color`,`AgentSchema`,`KNOWN_KEYS`,`normalize`,`Info` | `Color`,`AgentConfig`,`agent_normalize`,`AgentConfigInfo`→`Agent` field type | crates/oc-config/src/v1/agent.rs | transform normalize direplikasi persis (rest keys→options, tools→permission, steps??maxSteps) |
| v1/config/command.ts:5-13 | `Info` | `CommandInfo` | crates/oc-config/src/v1/command.rs | - |
| v1/config/error.ts:14-38 | `JsonError`,`InvalidError`,`FrontmatterError`,`DirectoryTypoError`,`RemoteAuthError` | `JsonError`,`InvalidError` (FrontmatterError dsb menyusul sprint masing2) | crates/oc-config/src/v1/error.rs | NamedError → thiserror struct bernama sama |
| v1/config/formatter.ts:5-13 | `Entry`,`Info` | `FormatterEntry`,`FormatterInfo` | crates/oc-config/src/v1/formatter.rs | bool \| record |
| v1/config/layout.ts:5 | `Layout` | `Layout` | crates/oc-config/src/v1/layout.rs | - |
| v1/config/lsp.ts:5-78 | `Disabled`,`Entry`,`builtinServerIds`,`requiresExtensionsForCustomServers`,`Info` | `LspDisabled`,`LspEntry`,`BUILTIN_SERVER_IDS`,`lsp_extensions_check`,`LspInfo` | crates/oc-config/src/v1/lsp.rs | check custom-server extensions direplikasi |
| v1/config/mcp.ts:6-63 | `Local`,`OAuth`,`Remote`,`Info` | `McpLocal`,`McpOAuth`,`McpRemote`,`McpInfo` (+union `{enabled}`) | crates/oc-config/src/v1/mcp.rs | union Record(String, Local\|Remote\|{enabled}) sesuai config.ts:113-115 |
| v1/config/permission.ts:5-48 | `Action`,`Object`,`Rule`,`InputObject`,`InputSchema`,`normalizeInput`,`Info` | `PermissionAction`,`PermissionObjectMap`,`PermissionRule`,`normalize_input`,`PermissionInfo` | crates/oc-config/src/v1/permission.rs | string → {"*": action} |
| v1/config/plugin.ts:5-9 | `Options`,`Spec` | `PluginOptions`,`PluginSpec` | crates/oc-config/src/v1/plugin_spec.rs | string \| [string, options] |
| v1/config/provider.ts:6-126 | `ModelStatus`,`InterleavedField`,`Model`,`Info` | `ModelStatus`,`InterleavedField`,`ProviderModel`,`ProviderInfo` | crates/oc-config/src/v1/provider.rs | StructWithRest options → flatten rest map |
| v1/config/server.ts:6-19 | `Server` | `ServerInfo` | crates/oc-config/src/v1/server.rs | - |
| v1/config/skills.ts:5-13 | `Info` | `SkillsInfo` | crates/oc-config/src/v1/skills.rs | - |
| v1/config/attachment.ts:6-25 | `Image`,`Info` | `AttachmentImage`,`AttachmentInfo` | crates/oc-config/src/v1/attachment.rs | - |
| config/experimental.ts:9-14 + policy.ts:6-13 + catalog.ts:20 | `PolicyAction`,`Policy`(fields action/effect/resource),`Effect` | `PolicyAction`,`PolicyEffect`,`ExperimentalPolicy` | crates/oc-config/src/v1/experimental.rs | PolicyAction literal tunggal "provider.use" |
| config/reference.ts:5-21 | `Git`,`Local`,`Entry`,`Info` | `ReferenceGit`,`ReferenceLocal`,`ReferenceEntry`,`ReferenceInfo` | crates/oc-config/src/v1/reference.rs | union urutan String→Git→Local dipertahankan |
| schema.ts (@opencode-ai/schema) | `PositiveInt`,`NonNegativeInt` | validator `positive_int`,`non_negative_int`,`non_negative_int_opt` | crates/oc-config/src/v1/mod.rs | branded int → custom deserializer u64 dengan constraint |

## Infra pendukung (bukan port identifier TS, didaftarkan agar auditable)

| Asal kebutuhan | Rust identifier | Rust lokasi | Catatan |
|---|---|---|---|
| objek JS order-preserving untuk typed record (`Record(String,X)` di schema) | `v1::OrderedMap<T>` | crates/oc-config/src/v1/mod.rs | serde_json::Map hanya trait-untuk Value; wrapper ini menjaga urutan kunci |
| jsonc-parser parse() level bawah utk golden test | `parse::parse_fault_tolerant`, `parse::print_parse_error_code`, `parse::ParseErrorCode` | crates/oc-config/src/parse.rs | publikasi algoritma yang sudah ada; dipakai tests/golden.rs |
| error agregat lapisan loader | `config::ConfigLoadError`, `config::ConfigState`, `config::ConfigHandle`, `config::InstanceContext` | crates/oc-config/src/config.rs | InstanceContext memirror project/instance-context.ts (subset field); multi-instance cache ditunda |
