//! Test oc-tool (Sprint 5a): read/glob/grep/write + path safety.
//! Fixture folder dibangun per test; deskripsi tool diverifikasi verbatim.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use oc_permission::PermissionService;
use oc_tool::{Context, ToolRegistry};

fn env_lock() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: std::sync::OnceLock<Mutex<()>> = std::sync::OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(())).lock().unwrap()
}

fn temp_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("oc-tool-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn setup_xdg(root: &Path) {
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
}

struct RecordingSink {
    asks: Mutex<Vec<String>>,
}

struct SinkWrap(Arc<RecordingSink>);

impl oc_permission::EventSink for SinkWrap {
    fn asked(&self, info: &oc_permission::Request) {
        self.0.asked(info)
    }
}

impl oc_permission::EventSink for RecordingSink {
    fn asked(&self, info: &oc_permission::Request) {
        self.asks.lock().unwrap().push(info.permission.clone());
    }
}

fn make_ctx(directory: PathBuf, worktree: PathBuf) -> (Context, Arc<RecordingSink>) {
    let sink = Arc::new(RecordingSink {
        asks: Mutex::new(Vec::new()),
    });
    let service: Arc<PermissionService> =
        Arc::new(PermissionService::new(Box::new(SinkWrap(sink.clone()))));
    // auto-approver: setiap ask yang masuk langsung di-reply Always
    // (meniru user menekan "always allow" pada prompt pertama).
    let approver = service.clone();
    std::thread::spawn(move || loop {
        for request in approver.list() {
            let _ = approver.reply(oc_permission::ReplyInput {
                request_id: request.id,
                reply: oc_permission::Reply::Always,
                message: None,
            });
        }
        std::thread::sleep(std::time::Duration::from_millis(2));
    });
    (
        Context {
            session_id: "ses_1".into(),
            message_id: "msg_1".into(),
            agent: "build".into(),
            directory,
            worktree,
            bypass_cwd_check: false,
            permission: service,
        },
        sink,
    )
}

fn fixture(root: &Path) -> PathBuf {
    let project = root.join("proj");
    std::fs::create_dir_all(project.join("src")).unwrap();
    std::fs::write(project.join("README.md"), "# Title\n").unwrap();
    std::fs::write(
        project.join("src/main.rs"),
        "fn main() {\n    println!(\"hello\");\n}\n",
    )
    .unwrap();
    std::fs::write(
        project.join("src/lib.rs"),
        "pub fn add(a: u32, b: u32) -> u32 {\n    a + b\n}\n",
    )
    .unwrap();
    project
}

#[test]
fn descriptions_are_verbatim_from_ts_assets() {
    // deskripsi harus persis isi .txt asli
    let registry = ToolRegistry::builtin();
    let expected_read = include_str!("../assets/read.txt");
    assert_eq!(registry.get("read").unwrap().description, expected_read);
    assert_eq!(
        registry.get("glob").unwrap().description,
        include_str!("../assets/glob.txt")
    );
    assert_eq!(
        registry.get("grep").unwrap().description,
        include_str!("../assets/grep.txt")
    );
    assert_eq!(
        registry.get("write").unwrap().description,
        include_str!("../assets/write.txt")
    );
}

#[test]
fn read_tool_file_output_format() {
    let _guard = env_lock();
    let root = temp_dir("read");
    setup_xdg(&root);
    let project = fixture(&root);

    // worktree == project supaya di dalam boundary
    let (ctx, _sink) = make_ctx(project.clone(), project.clone());
    let registry = ToolRegistry::builtin();

    let result = registry
        .get("read")
        .unwrap()
        .run(
            serde_json::json!({"filePath": project.join("src/main.rs").to_string_lossy()}),
            &ctx,
        )
        .unwrap();
    assert_eq!(result.title, "src/main.rs");
    assert!(result.output.starts_with("<path>"));
    assert!(result.output.contains("<type>file</type>"));
    assert!(result.output.contains("1: fn main() {"));
    assert!(result.output.contains("3: }"));
    assert!(result
        .output
        .ends_with("(End of file - total 3 lines)\n</content>"));

    // offset di luar range
    let error = registry
        .get("read")
        .unwrap()
        .run(serde_json::json!({"filePath": project.join("src/main.rs").to_string_lossy(), "offset": 99}), &ctx)
        .unwrap_err();
    assert!(error.to_string().contains("Offset 99 is out of range"));

    // file tidak ada → saran "Did you mean"
    let error = registry
        .get("read")
        .unwrap()
        .run(
            serde_json::json!({"filePath": project.join("src/main.rss").to_string_lossy()}),
            &ctx,
        )
        .unwrap_err();
    assert!(error.to_string().contains("File not found:"), "ERR={error}");
    assert!(error.to_string().contains("Did you mean one of these?"));

    // direktori listing
    let result = registry
        .get("read")
        .unwrap()
        .run(
            serde_json::json!({"filePath": project.join("src").to_string_lossy()}),
            &ctx,
        )
        .unwrap();
    assert!(result.output.contains("<type>directory</type>"));
    assert!(result.output.contains("(2 entries)"));
}

#[test]
fn write_tool_creates_and_asks_edit_permission() {
    let _guard = env_lock();
    let root = temp_dir("write");
    setup_xdg(&root);
    let project = fixture(&root);

    let (ctx, sink) = make_ctx(project.clone(), project.clone());
    let registry = ToolRegistry::builtin();
    let target = project.join("src/new.rs");

    registry
        .get("write")
        .unwrap()
        .run(
            serde_json::json!({"filePath": target.to_string_lossy(), "content": "pub const X: u8 = 1;\n"}),
            &ctx,
        )
        .unwrap();
    assert_eq!(
        std::fs::read_to_string(&target).unwrap(),
        "pub const X: u8 = 1;\n"
    );
    assert!(sink.asks.lock().unwrap().iter().any(|p| p == "edit"));
}

#[test]
fn glob_tool_matches_and_reports_absolute_paths() {
    let _guard = env_lock();
    let root = temp_dir("glob");
    setup_xdg(&root);
    let project = fixture(&root);

    let (ctx, _sink) = make_ctx(project.clone(), project.clone());
    let registry = ToolRegistry::builtin();

    let params = serde_json::json!({"pattern": "**/*.rs", "path": project.to_string_lossy()});
    let result = registry.get("glob").unwrap().run(params, &ctx).unwrap();
    assert!(result.output.contains("main.rs"), "{}", result.output);
    assert!(result.output.contains("lib.rs"));
    assert!(result.metadata["count"].as_u64().unwrap() >= 2);
    // path absolut dicetak (separator sesuai platform)
    let absolute_main = project.join("src/main.rs").to_string_lossy().into_owned();
    assert!(
        result.output.contains(&absolute_main)
            || result.output.contains(&absolute_main.replace('\\', "/"))
    );

    // tanpa hasil → "No files found"
    let result = registry
        .get("glob")
        .unwrap()
        .run(
            serde_json::json!({"pattern": "**/*.zzz", "path": project.to_string_lossy()}),
            &ctx,
        )
        .unwrap();
    assert_eq!(result.output, "No files found");
}

#[test]
fn grep_tool_output_format() {
    let _guard = env_lock();
    let root = temp_dir("grep");
    setup_xdg(&root);
    let project = fixture(&root);

    let (ctx, _sink) = make_ctx(project.clone(), project.clone());
    let registry = ToolRegistry::builtin();

    let result = registry
        .get("grep")
        .unwrap()
        .run(
            serde_json::json!({"pattern": "fn ", "path": project.to_string_lossy()}),
            &ctx,
        )
        .unwrap();
    assert!(result.output.starts_with("Found "), "{}", result.output);
    assert!(result.output.contains("Line 1: fn main() {"));
    assert!(result.metadata["matches"].as_u64().unwrap() >= 1);

    let result = registry
        .get("grep")
        .unwrap()
        .run(
            serde_json::json!({"pattern": "zzzznotfound", "path": project.to_string_lossy()}),
            &ctx,
        )
        .unwrap();
    assert_eq!(result.output, "No files found");
}

#[test]
fn path_outside_worktree_triggers_external_directory_ask() {
    let _guard = env_lock();
    let root = temp_dir("external");
    setup_xdg(&root);
    let project = fixture(&root);
    let outside = root.join("outside.txt");
    std::fs::write(&outside, "secret").unwrap();

    // worktree berbeda dari lokasi file → butuh approval external_directory
    let (ctx, sink) = make_ctx(project.clone(), project.clone());
    let registry = ToolRegistry::builtin();

    let ctx_for_run = ctx.clone();

    // auto-approver akan menyetujui external_directory (Always)
    let handle = std::thread::spawn(move || {
        registry
            .get("read")
            .unwrap()
            .run(
                serde_json::json!({"filePath": outside.to_string_lossy()}),
                &ctx_for_run,
            )
            .unwrap()
    });
    let result = handle.join().unwrap();
    assert!(result.output.contains("<type>file</type>"));
    // pastikan ask external_directory benar-benar diminta
    assert!(sink
        .asks
        .lock()
        .unwrap()
        .iter()
        .any(|p| p == "external_directory"));
}
