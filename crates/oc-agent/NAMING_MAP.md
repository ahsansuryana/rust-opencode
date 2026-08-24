# Naming Map — oc-agent

Format tabel sesuai `rust-opencode-plan/01_NAMING_CONVENTION.md`.
Sprint yang mengisi tabel ini: Sprint 8.

| TS asli (path:baris) | TS identifier | Rust identifier | Rust lokasi | Catatan |
|---|---|---|---|---|
| agent.ts:35-56 | `Info` (Agent schema) | `Info` | crates/oc-agent/src/lib.rs | mode/permission/model/prompt/options/steps dll |
| agent.ts:38 | `mode` Literals | `Mode` enum + `from_config_str` | crates/oc-agent/src/lib.rs | subagent/primary/all |
| agent.ts:140-265 | agents record (build, plan, general, explore, compaction, title, summary) | `registry::build_agents` | crates/oc-agent/src/registry.rs | permission merge via oc-permission; prompt files include_str! verbatim |
| agent.ts:119-136 | defaults Permission.fromConfig | `registry::default_permission_rules` | crates/oc-agent/src/registry.rs | *.env ask/read pattern ✓ |
| agent.ts:267-294 | config override loop | dalam `build_agents` | crates/oc-agent/src/registry.rs | disable/model/variant/prompt/temp/mode/color/hidden/steps/options/permission ✓ |
| agent.ts:297-310 | Truncate.GLOB allow unless denied | truncation_glob block | crates/oc-agent/src/registry.rs | - |
| agent.ts:312-344 | `get`,`list`,`defaultInfo`,`defaultAgent` | `AgentRegistry::{get,list,default_info,default_agent}` | crates/oc-agent/src/registry.rs | sort by default-first/name-asc ✓; subagent+hidden validation ✓ |
| agent/prompt/*.txt | PROMPT_COMPACTION/EXPLORE/TITLE/SUMMARY/GENERATE | `PROMPT_*` consts (include_str!) | crates/oc-agent/prompts/ | VERBATIM copy dari source asli |
