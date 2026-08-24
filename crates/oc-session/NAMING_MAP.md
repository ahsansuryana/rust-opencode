# Naming Map — oc-session

Format tabel sesuai `rust-opencode-plan/01_NAMING_CONVENTION.md`.
Sprint yang mengisi tabel ini: Sprint 9.

## Part types (schema/v1/session.ts:87-449 — 12 varian)

| TS asli | TS type literal | Rust variant | Catatan |
|---|---|---|---|
| TextPart | "text" | `Part::Text` | text/synthetic/ignored/time/metadata |
| ReasoningPart | "reasoning" | `Part::Reasoning` | text/metadata/time |
| FilePart | "file" | `Part::File` | mime/filename/url/source(FilePartSource union) |
| ToolPart | "tool" | `Part::Tool` | callID/tool/state(ToolState)/metadata |
| StepStartPart | "step-start" | `Part::StepStart` | snapshot? |
| StepFinishPart | "step-finish" | `Part::StepFinish` | reason/snapshot/cost/tokens(TokenUsage) |
| SnapshotPart | "snapshot" | `Part::Snapshot` | snapshot string |
| PatchPart | "patch" | `Part::Patch` | hash/files[] |
| AgentPart | "agent" | `Part::Agent` | name/source? |
| RetryPart | "retry" | `Part::Retry` | attempt/error(APIError)/time |
| CompactionPart | "compaction" | `Part::Compaction` | auto/overflow?/tail_start_id? |
| SubtaskPart | "subtask" | `Part::Subtask` | prompt/description/agent/model?/command? |

## ToolState (session.ts:259-314)

| TS state | Discriminator | Rust variant | Fields |
|---|---|---|---|
| ToolStatePending | "pending" | `ToolState::Pending` | input/raw |
| ToolStateRunning | "running" | `ToolState::Running` | input/title?/metadata?/time.start |
| ToolStateCompleted | "completed" | `ToolState::Completed` | input/output/title/metadata/time.{start,end,compacted?}/attachments? |
| ToolStateError | "error" | `ToolState::Error` | input/error/metadata?/time.{start,end} |

## Messages

| TS asli (path) | TS identifier | Rust identifier | Catatan |
|---|---|---|---|
| session.ts:399-451 | `User` ("UserMessage") | `UserOrAssistant::User(UserMessage)` | tag="role", serde snake_case |
| session.ts:453-497 | `Assistant` ("AssistantMessage") | `UserOrAssistant::Assistant(AssistantMessage)` | parentID/modelID/providerID/path/cost/tokens |
| message-v2.ts:98-123 | `hydrate`, `WithParts` | `WithParts` | info + parts[] |

## Session

| TS asli (path:baris) | TS identifier | Rust identifier | Catatan |
|---|---|---|---|
| core/session/sql.ts:22-70 | SessionTable row shape | `SessionRow` (type alias `Session`) | id/project_id/title/tokens/revert/permission/agent/model/time_* dll |

## Store (CRUD dasar)

| Fungsi | Rust method | Catatan |
|---|---|---|
| upsert session | `SessionStore::upsert_session` | write ke storage key session/info/<id> |
| get session | `get_session` | read, NotFound → None |
| remove session | `remove_session` | - |
| list sessions | `list_sessions` | sorted desc by time_created |
| append message | `append_message` | write ke message/<session_id>/<msg_id> |
| get message | `get_message` | - |
| list messages+parts | `list_messages` | hydrate pattern dari message-v2.ts:98-123 |
| write part | `write_part` | part/<session_id>/<msg_id>/<part_id> |
| list parts | `list_parts` | sorted by part_id |
