//! Test oc-session: Part round-trip JSON, ToolState transitions, CRUD.

use oc_session::model::*;
use oc_session::store::SessionStore;
use std::collections::BTreeMap;
use std::sync::Arc;

fn base_ids(session_id: &str, message_id: &str, part_num: u32) -> BaseIds {
    BaseIds {
        id: format!("prt_{part_num}"),
        session_id: session_id.into(),
        message_id: message_id.into(),
    }
}

#[test]
fn part_text_roundtrip() {
    let part = Part::Text {
        ids: base_ids("ses_1", "msg_1", 1),
        text: "hello world".into(),
        synthetic: None,
        ignored: None,
        time: Some(TimeRange {
            start: 100,
            end: Some(200),
        }),
        metadata: None,
    };
    let json = serde_json::to_value(&part).unwrap();
    assert_eq!(json["type"], "text");
    assert_eq!(json["text"], "hello world");
    assert_eq!(json["sessionID"], "ses_1");
    let back: Part = serde_json::from_value(json).unwrap();
    match &back {
        Part::Text { text, .. } => assert_eq!(text, "hello world"),
        _ => panic!("variant salah"),
    }
}

#[test]
fn part_tool_state_transitions() {
    let session_id = "ses_1";
    let msg_id = "msg_1";

    let states = vec![
        ToolState::Pending {
            input: Default::default(),
            raw: "ls".into(),
        },
        ToolState::Running {
            input: Default::default(),
            title: Some("listing".into()),
            metadata: None,
            time: TimeStart { start: 100 },
        },
        ToolState::Completed {
            input: Default::default(),
            output: "file.txt".into(),
            title: "listing files".into(),
            metadata: BTreeMap::new(),
            time: TimeCompleted {
                start: 100,
                end: 200,
                compacted: None,
            },
            attachments: None,
        },
    ];

    for state in states {
        let part = Part::Tool {
            ids: base_ids(session_id, msg_id, 2),
            call_id: "call_1".into(),
            tool: "bash".into(),
            state,
            metadata: None,
        };
        let json = serde_json::to_value(&part).unwrap();
        assert_eq!(json["type"], "tool");
        assert_eq!(json["callID"], "call_1");
        let back: Part = serde_json::from_value(json).unwrap();
        assert!(matches!(back, Part::Tool { .. }));
    }
}

#[test]
fn part_reasoning_roundtrip() {
    let part = Part::Reasoning {
        ids: base_ids("ses_1", "msg_1", 3),
        text: "thinking...".into(),
        metadata: None,
        time: TimeRange {
            start: 50,
            end: None,
        },
    };
    let json = serde_json::to_value(&part).unwrap();
    assert_eq!(json["type"], "reasoning");
    let back: Part = serde_json::from_value(json).unwrap();
    match &back {
        Part::Reasoning { text, .. } => assert_eq!(text, "thinking..."),
        _ => panic!("variant salah"),
    }
}

#[test]
fn message_user_and_assistant_roundtrip() {
    let user_msg = UserOrAssistant::User(UserMessage {
        id: "msg_1".into(),
        session_id: "ses_1".into(),
        time: TimeCreated {
            created: 1700000000,
        },
        format: None,
        summary: None,
        agent: "build".into(),
        model: ModelRefJson {
            provider_id: "anthropic".into(),
            model_id: "claude-sonnet-4".into(),
        },
        system: None,
        tools: None,
    });
    let json = serde_json::to_value(&user_msg).unwrap();
    assert_eq!(json["role"], "user");
    let back: UserOrAssistant = serde_json::from_value(json).unwrap();
    assert_eq!(back.id(), "msg_1");

    let assistant_msg = UserOrAssistant::Assistant(AssistantMessage {
        id: "msg_2".into(),
        session_id: "ses_1".into(),
        time: TimeWithCompletion {
            created: 1700000001,
            completed: Some(1700000010),
        },
        error: None,
        parent_id: "msg_1".into(),
        model_id: "claude-sonnet-4".into(),
        provider_id: "anthropic".into(),
        mode: "primary".into(),
        agent: "build".into(),
        path: SessionPath {
            cwd: "/proj".into(),
            root: "/proj".into(),
        },
        summary: None,
        cost: 0.01,
        tokens: TokenUsage {
            total: Some(100.0),
            input: 50.0,
            output: 40.0,
            reasoning: 10.0,
            cache: CacheReadWrite {
                read: 0.0,
                write: 5.0,
            },
        },
        structured: None,
        variant: None,
        finish: None,
    });
    let json = serde_json::to_value(&assistant_msg).unwrap();
    assert_eq!(json["role"], "assistant");
    assert_eq!(json["parentID"], "msg_1");
    let back: UserOrAssistant = serde_json::from_value(json).unwrap();
    assert_eq!(back.id(), "msg_2");
}

// --- CRUD ---

fn setup_store(tag: &str) -> SessionStore {
    let root = std::env::temp_dir().join(format!("oc-session-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    std::env::set_var("HOME", root.to_str().unwrap());
    std::env::set_var("USERPROFILE", root.to_str().unwrap());
    for key in [
        "XDG_DATA_HOME",
        "XDG_CACHE_HOME",
        "XDG_CONFIG_HOME",
        "XDG_STATE_HOME",
    ] {
        std::env::remove_var(key);
    }
    oc_global::reset_for_tests();
    SessionStore::new().unwrap()
}

#[test]
fn crud_sessions_and_messages() {
    let store = setup_store("crud");

    let session = SessionRow {
        id: "ses_test".into(),
        project_id: "proj_1".into(),
        title: "Test Session".into(),
        version: "1.18.21".into(),
        directory: "/tmp/proj".into(),
        slug: "test-session".into(),
        time_created: 1000,
        time_updated: 1000,
        ..Default::default()
    };
    store.upsert_session(&session).unwrap();

    let fetched = store.get_session("ses_test").unwrap().unwrap();
    assert_eq!(fetched.title, "Test Session");

    let user_msg = UserOrAssistant::User(UserMessage {
        id: "msg_u1".into(),
        session_id: "ses_test".into(),
        time: TimeCreated { created: 1001 },
        format: None,
        summary: None,
        agent: "build".into(),
        model: ModelRefJson {
            provider_id: "test".into(),
            model_id: "m".into(),
        },
        system: None,
        tools: None,
    });
    store.append_message(&user_msg).unwrap();

    let parts = vec![Part::Text {
        ids: BaseIds {
            id: "prt_1".into(),
            session_id: "ses_test".into(),
            message_id: "msg_a1".into(),
        },
        text: "response text".into(),
        synthetic: None,
        ignored: None,
        time: None,
        metadata: None,
    }];
    let assistant_msg = UserOrAssistant::Assistant(AssistantMessage {
        id: "msg_a1".into(),
        session_id: "ses_test".into(),
        time: TimeWithCompletion {
            created: 1002,
            completed: Some(1003),
        },
        error: None,
        parent_id: "msg_u1".into(),
        model_id: "m".into(),
        provider_id: "test".into(),
        mode: "primary".into(),
        agent: "build".into(),
        path: SessionPath {
            cwd: "/tmp".into(),
            root: "/tmp".into(),
        },
        summary: None,
        cost: 0.0,
        tokens: TokenUsage::default(),
        structured: None,
        variant: None,
        finish: None,
    });
    store.append_message(&assistant_msg).unwrap();
    for part in &parts {
        store.write_part("ses_test", "msg_a1", part).unwrap();
    }

    let messages = store.list_messages("ses_test").unwrap();
    assert_eq!(messages.len(), 2);
    match &messages[0].info {
        UserOrAssistant::User(_) => {}
        _ => panic!("pesan pertama harus user"),
    }
    assert!(!messages[1].parts.is_empty());

    store.remove_session("ses_test").unwrap();
    assert!(store.get_session("ses_test").unwrap().is_none());
}

// --- Sprint 10: prompt loop ---
use serde_json::Value;

use oc_session::prompt::{
    run_prompt_loop, PromptLoopInput, ProviderClient, ToolContext, ToolExecutor,
};
use oc_session::tool_result::ToolOutput;

struct MockProvider {
    responses: std::sync::Mutex<std::vec::IntoIter<Value>>,
}

impl ProviderClient for MockProvider {
    fn send(
        &self,
        _model: &str,
        _sys: &str,
        _msgs: &[Value],
        _tools: &[Value],
    ) -> Result<Value, String> {
        let mut guard = self.responses.lock().unwrap();
        guard
            .next()
            .ok_or_else(|| "no more mock responses".to_string())
    }
}

struct MockExecutor;

impl ToolExecutor for MockExecutor {
    fn execute(&self, tool: &str, _args: &Value, _ctx: &ToolContext) -> Result<ToolOutput, String> {
        match tool {
            "read" => Ok(ToolOutput::Text("file content here".into())),
            other => Err(format!("unknown tool: {other}")),
        }
    }
}

#[test]
fn prompt_loop_single_tool_call() {
    let store = setup_store("loop");

    let mock_responses = vec![
        // iterasi 1: tool call
        serde_json::json!({
            "choices": [{"message": {
                "content": null,
                "tool_calls": [{
                    "id": "call_1",
                    "function": {"name": "read", "arguments": "{\"filePath\": \"/tmp/f\"}"}
                }]
            }, "finish_reason": "tool_calls"}],
            "usage": {"prompt_tokens": 50, "completion_tokens": 20}
        }),
        // iterasi 2: final response
        serde_json::json!({
            "choices": [{"message": {
                "content": "Here is the file content.",
                "tool_calls": null
            }, "finish_reason": "stop"}],
            "usage": {"prompt_tokens": 80, "completion_tokens": 15}
        }),
    ];

    let provider = MockProvider {
        responses: std::sync::Mutex::new(mock_responses.into_iter()),
    };
    let executor = MockExecutor;
    let events: std::sync::Arc<std::sync::Mutex<Vec<String>>> = Default::default();
    let events_clone = events.clone();
    let sender: oc_session::prompt::EventSender = Arc::new(move |event| {
        events_clone.lock().unwrap().push(format!("{event:?}"));
    });

    let dir = std::path::PathBuf::from("/tmp/proj");
    let input = PromptLoopInput {
        session_id: "ses_loop",
        parent_message_id: "msg_parent",
        agent: "build",
        model_provider_id: "openai",
        model_id: "gpt-5",
        system: "You are helpful.",
        directory: &dir,
        worktree: &dir,
        max_tokens: 4096,
        max_iterations: 5,
        cancellation: None,
    };

    let messages = vec![serde_json::json!({"role": "user", "content": "read /tmp/f"})];
    let tools = vec![];

    let result = run_prompt_loop(
        &store, &provider, &executor, &sender, &input, &messages, &tools,
    )
    .unwrap();

    assert!(result.output_text.contains("file content"));
    assert_eq!(result.tokens.input, 130); // 50 + 80
    assert_eq!(result.finish_reason.as_deref(), Some("stop"));

    // events dipublish
    let log = events.lock().unwrap();
    assert!(log.iter().any(|e| e.contains("PartUpdated")));
    assert!(log.iter().any(|e| e.contains("ToolExecuted")));
    assert!(log.iter().any(|e| e.contains("MessageCompleted")));

    // pesan tersimpan di store
    let stored = store
        .get_message("ses_loop", &result.assistant_message_id)
        .unwrap();
    assert!(stored.is_some());
}

// --- Sprint 10b: cancellation ---

#[test]
fn prompt_loop_cancellation_stops_early() {
    use oc_session::cancellation::CancellationToken;

    let store = setup_store("cancel");
    let token = CancellationToken::new();

    // Provider yang selalu return tool_calls (loop tak akan berhenti sendiri)
    struct InfiniteProvider;
    impl ProviderClient for InfiniteProvider {
        fn send(&self, _: &str, _: &str, _: &[Value], _: &[Value]) -> Result<Value, String> {
            Ok(serde_json::json!({
                "choices": [{"message": {
                    "content": null,
                    "tool_calls": [{
                        "id": "call_loop",
                        "function": {"name": "read", "arguments": "{}"}
                    }]
                }, "finish_reason": "tool_calls"}],
                "usage": {"prompt_tokens": 1, "completion_tokens": 1}
            }))
        }
    }

    // Cancel SEBELUM loop mulai → langsung break di iterasi pertama
    token.cancel();

    let dir = std::path::PathBuf::from("/tmp/proj");
    let input = PromptLoopInput {
        session_id: "ses_cancel",
        parent_message_id: "msg_p",
        agent: "build",
        model_provider_id: "test",
        model_id: "test",
        system: "",
        directory: &dir,
        worktree: &dir,
        max_tokens: 4096,
        max_iterations: 100, // tinggi — tapi cancelled
        cancellation: Some(token),
    };

    let provider = InfiniteProvider;
    struct NoopExecutor;
    impl ToolExecutor for NoopExecutor {
        fn execute(&self, _: &str, _: &Value, _: &ToolContext) -> Result<ToolOutput, String> {
            Ok(ToolOutput::Text("".into()))
        }
    }
    let sender: oc_session::prompt::EventSender = Arc::new(|_| {});

    let result =
        run_prompt_loop(&store, &provider, &NoopExecutor, &sender, &input, &[], &[]).unwrap();

    assert_eq!(result.finish_reason.as_deref(), Some("aborted"));
}
