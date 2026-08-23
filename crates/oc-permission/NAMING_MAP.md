# Naming Map — oc-permission

Format tabel sesuai `rust-opencode-plan/01_NAMING_CONVENTION.md`.
Sprint yang mengisi tabel ini: Sprint 4.

| TS asli (path:baris) | TS identifier | Rust identifier | Rust lokasi | Catatan |
|---|---|---|---|---|
| core/util/wildcard.ts:3-13 | `Wildcard.match` | `wildcard::match` | crates/oc-permission/src/wildcard.rs | glob matcher DP (*, ?, literal; backslash→slash; trailing " *" opsional; case-fold di win32) |
| schema/v1/permission.ts:8-14 | `PermissionV1.ID` + `ascending` | `Id`, `id_ascending()` | crates/oc-permission/src/lib.rs | prefix "per_"; token monotonik (approx ULID, dicatat) |
| v1/permission.ts:16-19 | `Action` | `Action` | crates/oc-permission/src/lib.rs | allow/deny/ask |
| v1/permission.ts:21-25 | `Rule`, `Ruleset` | `Rule`, `Ruleset` | crates/oc-permission/src/lib.rs | - |
| v1/permission.ts:27-38 | `Request` | `Request` | crates/oc-permission/src/lib.rs | sessionID/metadata/tool sesuai shape |
| v1/permission.ts:40-42 | `Reply` | `Reply` | crates/oc-permission/src/lib.rs | once/always/reject |
| v1/permission.ts:48-51 | `AskInput` | `AskInput` | crates/oc-permission/src/lib.rs | id opsional + ruleset |
| v1/permission.ts:53-56 | `ReplyInput` | `ReplyInput` | crates/oc-permission/src/lib.rs | - |
| core/v1/permission.ts:4-24 | `RejectedError`,`CorrectedError`,`DeniedError`,`NotFoundError`,`Error` | enum `Error` + struct error bernama sama | crates/oc-permission/src/lib.rs | pesan DeniedError memakai JSON.stringify(ruleset) |
| permission/index.ts:28-38 | `evaluate` | `evaluate` | crates/oc-permission/src/lib.rs | flat + findLast; fallback ask |
| index.ts:12-16,40 | `Interface`, `Service "@opencode/Permission"` | `PermissionService` | crates/oc-permission/src/lib.rs | ask memblokir menunggu reply (Deferred → Mutex+Condvar); EventV2Bridge → trait `EventSink` (Noop default) |
| index.ts:67-107 | `ask` | `PermissionService::ask` | crates/oc-permission/src/lib.rs | deny short-circuit + ruleset terfilter |
| index.ts:109-167 | `reply` | `PermissionService::reply` | crates/oc-permission/src/lib.rs | reject cascade per-session; always → approved.push + cascade auto-succeed |
| index.ts:169-172 | `list` | `list` | crates/oc-permission/src/lib.rs | snapshot pending |
| index.ts:178-184 | `expand` (privat) | `expand_pattern` | crates/oc-permission/src/lib.rs | ~/, ~, $HOME/, $HOME |
| index.ts:186-198 | `fromConfig` | `from_config_value` (Value, urutan kunci asli) + `from_config_info` (typed) | crates/oc-permission/src/lib.rs | versi Value menjaga urutan; versi typed punya caveat urutan field |
| index.ts:200-202 | `merge` | `merge_rulesets` | crates/oc-permission/src/lib.rs | flatten |
| index.ts:204-214 | `disabled` | `disabled_tools` | crates/oc-permission/src/lib.rs | edits/reads alias list persis |
| index.ts:216-219 | `visibleTools` | `visible_tools` | crates/oc-permission/src/lib.rs | filter record |
| permission/arity.ts:1-9 | `prefix` | `arity::prefix` | crates/oc-permission/src/arity.rs | longest-prefix; default token pertama (SESUAI KODE, bukan komentar) |
| arity.ts:24-161 | `ARITY` | `ARITY` (const map) | crates/oc-permission/src/arity.rs | tabel disalin verbatim |

Trait dependency injection (permintaan sprint, bukan identifier TS):

| Tujuan | Rust identifier | Catatan |
|---|---|---|
| pengganti EventV2Bridge.publish | trait `EventSink` (+ `NoopSink`) | asked/replied callback |
| prompter interaktif (untuk CLI sprint 15) | trait `Prompter` (+ `AutoDeny`) | CLI nantinya meng-inject implementasi nyata; service saat ini menerima `reply()` eksternal seperti source asli |
