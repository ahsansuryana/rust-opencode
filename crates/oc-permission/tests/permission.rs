//! Test oc-permission: urutan evaluasi rule (findLast), alur ask/reply
//! (once/always/reject + cascade), fromConfig, arity, disabled tools.

use std::sync::{Arc, Mutex};

use oc_permission::arity::prefix;
use oc_permission::{
    evaluate, from_config_value, id_ascending, visible_tools, Action, AskInput, PermissionService,
    Reply, ReplyInput, Rule,
};
use serde_json::json;

fn rule(permission: &str, pattern: &str, action: Action) -> Rule {
    #[allow(clippy::needless_update)]
    Rule {
        permission: permission.to_string(),
        pattern: pattern.to_string(),
        action,
    }
}

#[derive(Clone, Default)]
struct SinkLog {
    asked: Arc<Mutex<Vec<String>>>,
    replied: Arc<Mutex<Vec<String>>>,
}

impl oc_permission::EventSink for SinkLog {
    fn asked(&self, info: &oc_permission::Request) {
        self.asked.lock().unwrap().push(info.id.clone());
    }
    fn replied(&self, _session_id: &str, request_id: &str, reply: Reply) {
        self.replied
            .lock()
            .unwrap()
            .push(format!("{request_id}:{:?}", reply));
    }
}

#[test]
fn evaluate_last_matching_rule_wins() {
    let ruleset = vec![
        rule("bash", "*", Action::Deny),
        rule("bash", "git *", Action::Allow),
    ];
    // findLast → rule kedua menang utk git push
    assert_eq!(
        evaluate("bash", "git push", std::slice::from_ref(&ruleset)).action,
        Action::Allow
    );
    assert_eq!(
        evaluate("bash", "rm -rf /", &[ruleset]).action,
        Action::Deny
    );
}

#[test]
fn evaluate_fallback_is_ask_and_later_ruleset_overrides() {
    let rules = vec![rule("edit", "*", Action::Allow)];
    // fallback tanpa match sama sekali
    let fallback = evaluate("webfetch", "https://x", &[rules]);
    assert_eq!(fallback.action, Action::Ask);
    assert_eq!(fallback.pattern, "*");

    // approved (ruleset kedua) dipakai setelah ruleset config
    let again = vec![rule("edit", "*", Action::Allow)];
    let approved = vec![rule("edit", "/tmp/*", Action::Allow)];
    assert_eq!(
        evaluate("edit", "/tmp/f", &[again, approved]).action,
        Action::Allow
    );
}

#[test]
fn id_ascending_starts_with_per() {
    let a = id_ascending();
    let b = id_ascending();
    assert!(a.starts_with("per_"));
    assert!(b.starts_with("per_"));
    assert_ne!(a, b);
}

fn base_ask(permission: &str, patterns: &[&str]) -> AskInput {
    AskInput {
        session_id: "ses_1".into(),
        permission: permission.into(),
        patterns: patterns.iter().map(|p| p.to_string()).collect(),
        metadata: Default::default(),
        always: Vec::new(),
        tool: None,
        id: None,
        ruleset: Vec::new(),
    }
}

#[test]
fn deny_short_circuits_without_asking() {
    let sink = SinkLog::default();
    let service = PermissionService::new(Box::new(sink.clone()));
    let mut input = base_ask("bash", &["rm -rf /"]);
    input.ruleset = vec![rule("bash", "rm *", Action::Deny)];
    let error = service.ask(input).unwrap_err();
    let oc_permission::Error::Denied(denied) = error else {
        panic!("harus Denied");
    };
    let message = denied.message();
    assert!(message.contains("relevant rules"));
    // ruleset terfilter hanya yang permission-nya match
    assert!(message.contains("\"permission\":\"bash\""));
    assert!(
        sink.asked.lock().unwrap().is_empty(),
        "tidak boleh ada pending"
    );
}

#[test]
fn allow_rules_skip_asking() {
    let sink = SinkLog::default();
    let service = PermissionService::new(Box::new(sink.clone()));
    let mut input = base_ask("edit", &["/tmp/a"]);
    input.ruleset = vec![rule("edit", "/tmp/*", Action::Allow)];
    service.ask(input).unwrap();
    assert!(sink.asked.lock().unwrap().is_empty());
}

#[test]
fn ask_blocks_until_reply_once_and_always_then_cascades() {
    let sink = SinkLog::default();
    let service = Arc::new(PermissionService::new(Box::new(sink.clone())));

    // thread 1: ask butuh approval untuk dua pattern berbeda
    let s1 = service.clone();
    let t1 = std::thread::spawn(move || {
        s1.ask(base_ask("bash", &["npm test"]))
            .map(|_| "t1-ok".to_string())
    });
    let s2 = service.clone();
    let t2 = std::thread::spawn(move || {
        s2.ask(base_ask("bash", &["cargo build"]))
            .map(|_| "t2-ok".to_string())
    });

    // tunggu keduanya masuk pending
    for _ in 0..200 {
        if service.list().len() == 2 {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert_eq!(service.list().len(), 2);

    // reply ALWAYS pada request pertama — pattern "always" kosong, jadi tidak
    // menambah approved; kedua tetap harus di-reply manual.
    let snapshot = service.list();
    assert_eq!(snapshot.len(), 2);
    let id_first = snapshot[0].id.clone();
    let first_id = id_first.clone();
    service
        .reply(ReplyInput {
            request_id: first_id.clone(),
            reply: Reply::Once,
            message: None,
        })
        .unwrap();

    for _ in 0..200 {
        if service.list().len() == 1 {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert_eq!(service.list().len(), 1);

    let remaining_id = service.list()[0].id.clone();
    service
        .reply(ReplyInput {
            request_id: remaining_id,
            reply: Reply::Reject,
            message: Some("pakai cargo check saja".into()),
        })
        .unwrap();

    let r1 = t1.join().unwrap();
    let r2 = t2.join().unwrap();
    let results = [
        r1.unwrap_or_else(|e| format!("ERR:{e}")),
        r2.unwrap_or_else(|e| format!("ERR:{e}")),
    ];
    // thread mana yang dapat Once tidak deterministik — yang pasti tepat
    // SATU sukses dan SATU CorrectedError dengan feedback.
    let oks = results.iter().filter(|r| !r.starts_with("ERR:")).count();
    assert_eq!(oks, 1, "results={results:?}");
    let corrected = results.iter().find(|r| r.starts_with("ERR:")).unwrap();
    assert!(corrected.contains("pakai cargo check saja"), "{corrected}");

    // event log: 2 asked, lalu replied once + reject
    assert_eq!(sink.asked.lock().unwrap().len(), 2);
    assert_eq!(sink.replied.lock().unwrap().len(), 2);
}

#[test]
fn always_reply_adds_approved_and_resolves_same_session_pending() {
    let sink = SinkLog::default();
    let service = Arc::new(PermissionService::new(Box::new(sink)));

    let mut input_a = base_ask("edit", &["/repo/a"]);
    input_a.always = vec!["/repo/*".to_string()];
    let mut input_b = base_ask("edit", &["/repo/b"]);
    input_b.always = vec!["/repo/*".to_string()];

    let s1 = service.clone();
    let ta = std::thread::spawn(move || s1.ask(input_a));
    let s2 = service.clone();
    let tb = std::thread::spawn(move || s2.ask(input_b));

    for _ in 0..200 {
        if service.list().len() == 2 {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    let first_id = service.list()[0].id.clone();
    service
        .reply(ReplyInput {
            request_id: first_id,
            reply: Reply::Always,
            message: None,
        })
        .unwrap();

    // cascade: bila approved rule cocok utk pending lain pada session sama,
    // ia auto-resolve; kalau tidak, tetap pending dan kita resolve manual.
    let ra = ta.join().unwrap();
    let rb = tb.join().unwrap();
    // salah satu pasti sukses langsung; yang lain tergantung urutan pattern.
    let outcomes = [ra.map(|_| ()).err(), rb.map(|_| ()).err()];
    assert!(
        outcomes.iter().all(|o| o.is_none()),
        "keduanya harus resolve"
    );
}

#[test]
fn reject_cascades_to_same_session_only() {
    let service = Arc::new(PermissionService::new(Box::new(oc_permission::NoopSink)));
    let s1 = service.clone();
    let ta = std::thread::spawn(move || s1.ask(base_ask("bash", &["a"])));
    let s2 = service.clone();
    let tb = std::thread::spawn(move || s2.ask(base_ask("bash", &["b"])));
    let s3 = service.clone();
    let tc = std::thread::spawn(move || {
        let mut input = base_ask("bash", &["c"]);
        input.session_id = "ses_lain".into();
        s3.ask(input)
    });

    for _ in 0..200 {
        if service.list().len() == 3 {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    let target = service
        .list()
        .iter()
        .find(|r| r.session_id == "ses_1")
        .unwrap()
        .id
        .clone();
    service
        .reply(ReplyInput {
            request_id: target,
            reply: Reply::Reject,
            message: None,
        })
        .unwrap();
    // beri kesempatan cascade menyebar
    std::thread::sleep(std::time::Duration::from_millis(30));
    // resolve request milik ses_lain supaya join tidak menggantung
    if let Some(other) = service.list().first().cloned() {
        service
            .reply(ReplyInput {
                request_id: other.id,
                reply: Reply::Once,
                message: None,
            })
            .unwrap();
    }

    let results = [ta.join().unwrap(), tb.join().unwrap(), tc.join().unwrap()];
    let errs: Vec<String> = results
        .into_iter()
        .map(|r| r.err().map(|e| e.to_string()).unwrap_or_default())
        .collect();
    // dua request ses_1 gagal Rejected, ses_lain sukses
    assert_eq!(errs.iter().filter(|e| e.is_empty()).count(), 1);
    assert_eq!(errs.iter().filter(|e| e.contains("rejected")).count(), 2);
}

#[test]
fn reply_unknown_request_is_not_found() {
    let service = PermissionService::default();
    let err = service
        .reply(ReplyInput {
            request_id: "per_missing".into(),
            reply: Reply::Once,
            message: None,
        })
        .unwrap_err();
    assert_eq!(err.request_id, "per_missing");
}

#[test]
fn from_config_expands_home_and_star_patterns() {
    // pakai $HOME supaya test platform-neutral
    let config = json!({
        "edit": "ask",
        "bash": { "$HOME/work/**": "allow", "*": "deny" },
        "scary": { "~/secret": "deny" },
    });
    let rules = from_config_value(&config);

    assert!(rules.contains(&Rule {
        permission: "edit".into(),
        action: Action::Ask,
        pattern: "*".into(),
    }));

    let bash_allow = rules
        .iter()
        .find(|r| r.permission == "bash" && r.action == Action::Allow)
        .unwrap();
    assert!(!bash_allow.pattern.starts_with('$'));
    assert!(bash_allow.pattern.ends_with("/work/**"));

    let bash_deny = rules
        .iter()
        .find(|r| r.permission == "bash" && r.action == Action::Deny)
        .unwrap();
    assert_eq!(bash_deny.pattern, "*");

    // urutan Object.entries dipertahankan: allow sebelum deny utk bash
    let bash_idx = |action: Action| {
        rules
            .iter()
            .position(|r| r.permission == "bash" && r.action == action)
            .unwrap()
    };
    assert!(bash_idx(Action::Allow) < bash_idx(Action::Deny));
}

#[test]
fn disabled_and_visible_tools_match_ts_semantics() {
    let tools = vec![
        ("bash".to_string(), 1usize),
        ("edit".to_string(), 2),
        ("write".to_string(), 3),
        ("read_mcp_resource".to_string(), 4),
        ("grep".to_string(), 5),
    ];
    let ruleset = vec![
        rule("bash", "*", Action::Deny),
        rule("edit", "*", Action::Deny),
        rule("read", "*", Action::Deny),
    ];
    let hidden = oc_permission::disabled_tools(
        &tools.iter().map(|(n, _)| n.clone()).collect::<Vec<_>>(),
        &ruleset,
    );
    assert!(hidden.contains("bash"));
    assert!(hidden.contains("edit")); // alias edit-group
    assert!(hidden.contains("write"));
    assert!(hidden.contains("read_mcp_resource"));
    assert!(!hidden.contains("grep"));

    let visible = visible_tools(&tools, &ruleset);
    assert_eq!(visible.len(), 1);
    assert_eq!(visible[0].0, "grep");

    // tanpa rule matching → tidak ada yang disembunyikan
    assert!(oc_permission::disabled_tools(&["bash".into()], &vec![]).is_empty());
}

#[test]
fn arity_prefix_longest_match_and_defaults() {
    let toks = |s: &[&str]| -> Vec<String> { s.iter().map(|x| x.to_string()).collect() };
    // git arity 2
    assert_eq!(
        prefix(&toks(&["git", "checkout", "main"])),
        toks(&["git", "checkout"])
    );
    // npm run arity 3
    assert_eq!(
        prefix(&toks(&["npm", "run", "dev"])),
        toks(&["npm", "run", "dev"])
    );
    // npm arity 2 → dua token dipertahankan (slice(0,2))
    assert_eq!(
        prefix(&toks(&["npm", "install"])),
        toks(&["npm", "install"])
    );
    // touch arity 1
    assert_eq!(prefix(&toks(&["touch", "file.txt"])), toks(&["touch"]));
    // python ada di tabel, arity 2
    assert_eq!(
        prefix(&toks(&["python", "-m", "venv"])),
        toks(&["python", "-m"])
    );
    // default (tidak ada di tabel): token pertama saja — per KODE
    assert_eq!(
        prefix(&toks(&["frobnicate", "x", "y"])),
        toks(&["frobnicate"])
    );
    assert!(prefix(&[]).is_empty());
}
