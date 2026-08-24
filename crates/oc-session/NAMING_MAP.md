# Naming Map — oc-session

Format tabel sesuai `rust-opencode-plan/01_NAMING_CONVENTION.md`.
Sprint yang mengisi tabel ini: Sprint 9 + 10a.

## Sprint 9 (data model) — lihat model.rs
Lihat NAMING_MAP di bawah untuk Part types, ToolState, Messages, SessionRow.

## Sprint 10a (prompt loop)

| TS asli (path:baris) | TS identifier | Rust identifier | Rust lokasi | Catatan |
|---|---|---|---|---|
| prompt.ts | prompt processing loop | `prompt::run_prompt_loop` | crates/oc-session/src/prompt.rs | resolve→call→parse→execute→feed-back→repeat |
| (trait design) | - | `ProviderClient` trait | crates/oc-session/src/prompt.rs | mock-able provider boundary |
| (trait design) | - | `ToolExecutor` trait | crates/oc-session/src/prompt.rs | mock-able tool execution |
| prompt.ts:349-460 | ToolPart state events | `LoopEvent::PartUpdated/ToolExecuted/MessageCompleted` | crates/oc-session/src/prompt.rs | EventSender callback |
| prompt.ts:563-621 | `resolveTools`+execute | dalam `run_prompt_loop` | crates/oc-session/src/prompt.rs | permission check via ctx.ask |
| prompt.ts:588-610 | provider call point | dalam loop iterasi | crates/oc-session/src/prompt.rs | transform → send → parse |
| message-v2.ts | token usage tracking | `TokenUsageResult` accumulation | crates/oc-session/src/prompt.rs | dari response.usage per iterasi |
| tool/task.ts | TaskTool subagent spawning | DITUNDA ke 10b | - | butuh session-child creation |
| (cancellation) | abort/interrupt | DITUNDA ke 10b | - | tokio::CancellationToken menyusul |

## Sprint 11

| TS asli (path:baris) | TS identifier | Rust identifier | Rust lokasi | Catatan |
|---|---|---|---|---|
| overflow.ts | `usable(cfg,model,outputMax)` | `overflow::usable` | crates/oc-session/src/overflow.rs | COMPACTION_BUFFER=20k; input_limit vs context-maxOutput |
| overflow.ts | `isOverflow(cfg,tokens,model)` | `overflow::is_overflow` | crates/oc-session/src/overflow.rs | auto=false → false; total ?? sum; >= usable |
| compaction.ts:28-29 | `PRUNE_MINIMUM`,`PRUNE_PROTECT` | `PRUNE_MINIMUM`,`PRUNE_PROTECT` | crates/oc-session/src/overflow.rs | 20k/40k |
| compaction.ts | shouldPrune logic | `should_prune` | crates/oc-session/src/overflow.rs | > PRUNE_PROTECT |
| compaction.ts:166-189 | `Service.isOverflow` | `is_overflow` (free fn) | crates/oc-session/src/overflow.rs | Effect service → free fn |
