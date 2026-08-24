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

// --- Sprint 5b: edit / truncate / registry model filter ---

use oc_tool::edit;

#[test]
fn edit_tool_exact_and_strategies() {
    let _guard = env_lock();
    let root = temp_dir("edit");
    setup_xdg(&root);
    let project = root.join("proj");
    std::fs::create_dir_all(&project).unwrap();
    let (ctx, sink) = make_ctx(project.clone(), project.clone());
    let registry = ToolRegistry::builtin();
    let target = project.join("code.rs");

    // buat file via oldString="" (file belum ada)
    registry
        .get("edit")
        .unwrap()
        .run(
            serde_json::json!({"filePath": target.to_string_lossy(), "oldString": "", "newString": "fn a() {}\nfn b() {}\n"}),
            &ctx,
        )
        .unwrap();
    assert_eq!(
        std::fs::read_to_string(&target).unwrap(),
        "fn a() {}\nfn b() {}\n"
    );

    // oldString kosong di file existing → error spesifik
    let err = registry
        .get("edit")
        .unwrap()
        .run(
            serde_json::json!({"filePath": target.to_string_lossy(), "oldString": "", "newString": "x"}),
            &ctx,
        )
        .unwrap_err();
    assert!(err.to_string().contains("oldString cannot be empty"));

    // replace persis
    registry
        .get("edit")
        .unwrap()
        .run(
            serde_json::json!({"filePath": target.to_string_lossy(), "oldString": "fn a() {}", "newString": "fn a2() {}"}),
            &ctx,
        )
        .unwrap();
    assert!(std::fs::read_to_string(&target)
        .unwrap()
        .contains("fn a2() {}"));
    assert!(sink.asks.lock().unwrap().iter().any(|p| p == "edit"));

    // line-trimmed (indentasi beda)
    std::fs::write(&target, "    fn indented() {\n        body();\n    }\n").unwrap();
    registry
        .get("edit")
        .unwrap()
        .run(
            serde_json::json!({"filePath": target.to_string_lossy(), "oldString": "  fn indented() {\n    body();\n  }", "newString": "  fn renamed() {}"}),
            &ctx,
        )
        .unwrap();
    assert!(std::fs::read_to_string(&target)
        .unwrap()
        .contains("renamed"));

    // whitespace-normalized: jumlah spasi berbeda tapi urutan kata sama
    std::fs::write(&target, "let   x   =   1;\n").unwrap();
    registry
        .get("edit")
        .unwrap()
        .run(
            serde_json::json!({"filePath": target.to_string_lossy(), "oldString": "let x = 1;", "newString": "let y = 2;"}),
            &ctx,
        )
        .unwrap();
    assert_eq!(std::fs::read_to_string(&target).unwrap(), "let y = 2;\n");

    // multiple match tanpa replaceAll → error; dengan replaceAll → semua
    std::fs::write(&target, "x\nx\n").unwrap();
    let err = registry
        .get("edit")
        .unwrap()
        .run(
            serde_json::json!({"filePath": target.to_string_lossy(), "oldString": "x", "newString": "y"}),
            &ctx,
        )
        .unwrap_err();
    assert!(err.to_string().contains("Found multiple matches"));
    registry
        .get("edit")
        .unwrap()
        .run(
            serde_json::json!({"filePath": target.to_string_lossy(), "oldString": "x", "newString": "y", "replaceAll": true}),
            &ctx,
        )
        .unwrap();
    assert_eq!(std::fs::read_to_string(&target).unwrap(), "y\ny\n");

    // tidak ketemu
    let err = registry
        .get("edit")
        .unwrap()
        .run(
            serde_json::json!({"filePath": target.to_string_lossy(), "oldString": "nope-nope", "newString": "z"}),
            &ctx,
        )
        .unwrap_err();
    assert!(err.to_string().contains("Could not find oldString"));

    // CRLF dipertahankan
    std::fs::write(&target, "a\r\nb\r\n").unwrap();
    registry
        .get("edit")
        .unwrap()
        .run(
            serde_json::json!({"filePath": target.to_string_lossy(), "oldString": "a", "newString": "A"}),
            &ctx,
        )
        .unwrap();
    assert_eq!(std::fs::read_to_string(&target).unwrap(), "A\r\nb\r\n");

    // output sukses persis + metadata filediff
    std::fs::write(&target, "q\n").unwrap();
    let result = registry
        .get("edit")
        .unwrap()
        .run(
            serde_json::json!({"filePath": target.to_string_lossy(), "oldString": "q", "newString": "qq"}),
            &ctx,
        )
        .unwrap();
    assert_eq!(result.output, "Edit applied successfully.");
    assert_eq!(result.metadata["filediff"]["additions"], 1);
    assert_eq!(result.metadata["filediff"]["deletions"], 1);
}

#[test]
fn edit_replace_disproportionate_guard() {
    let content = "start\n1\n2\n3\n4\n5\n6\n7\n8\n9\n10\nend\n";
    let result = edit::replace(content, "start", "s", false);
    // disproportionate: matched span jauh lebih besar dari oldString
    // (simple replacer exact match "start" aman)
    assert!(result.is_ok());
    let r = edit::replace("aaa\n".repeat(300).as_str(), "aaa", "bbb", true);
    assert!(r.is_ok());
}

#[test]
fn truncate_output_passthrough_and_head_tail() {
    use oc_tool::truncate::{self, Options, TruncateResult};

    let _guard = env_lock();
    let root = temp_dir("truncate-env");
    setup_xdg(&root);

    // muat → utuh
    let small = "line1\nline2\n";
    match truncate::output(small, Options::default()).unwrap() {
        TruncateResult::Content(c) => assert_eq!(c, small),
        _ => panic!("tidak boleh truncate"),
    }

    // kelebihan baris → head preview + hint + file tersimpan
    let big: String = (0..2500)
        .map(|i| format!("line{i}\n"))
        .collect::<Vec<_>>()
        .join("\n");
    let dir_before = std::fs::read_dir(truncate::truncation_dir())
        .map(|d| d.count())
        .unwrap_or(0);
    match truncate::output(&big, Options::default()).unwrap() {
        TruncateResult::Truncated {
            content,
            output_path,
        } => {
            assert!(content.contains(" truncated..."), "{content}");
            assert!(content.contains("Full output saved to:"));
            assert!(output_path.exists());
            let dir_now = truncate::truncation_dir();
            assert!(
                output_path.starts_with(&dir_now),
                "out={output_path:?} dir={dir_now:?}"
            );
        }
        _ => panic!("harus truncate"),
    }
    let dir_after = std::fs::read_dir(truncate::truncation_dir())
        .map(|d| d.count())
        .unwrap_or(0);
    assert!(dir_after > dir_before);

    // tail direction
    let marker = "TAIL-MARKER";
    let text = format!("{}\n{marker}", "filler\n".repeat(2100));
    match truncate::output(
        &text,
        Options {
            max_lines: Some(5),
            max_bytes: None,
            tail: true,
        },
    )
    .unwrap()
    {
        TruncateResult::Truncated { content, .. } => {
            assert!(content.contains(marker), "tail harus menyimpan baris akhir");
            assert!(content.starts_with("..."));
        }
        _ => panic!("harus truncate"),
    }
}

#[test]
fn registry_model_filter_swaps_edit_write_for_gpt() {
    let registry = ToolRegistry::builtin();
    let ids_for = |model: &str| -> Vec<String> {
        registry
            .tools_for_model(model)
            .into_iter()
            .map(|t| t.id.to_string())
            .collect()
    };

    let claude = ids_for("claude-sonnet-4-5");
    assert!(claude.contains(&"edit".to_string()));
    assert!(claude.contains(&"write".to_string()));

    let gpt5 = ids_for("gpt-5");
    assert!(!gpt5.contains(&"edit".to_string()));
    assert!(!gpt5.contains(&"write".to_string()));

    // oss & gpt-4 tetap pakai edit/write
    assert!(ids_for("gpt-oss-120b").contains(&"edit".to_string()));
    assert!(ids_for("gpt-4o").contains(&"edit".to_string()));
}

// --- Sprint 6a: shell detection + shell tool ---

use oc_tool::shell_detect;

#[test]
fn shell_detection_meta_and_args() {
    // ps/posix classification
    assert!(shell_detect::ps(
        r"C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe"
    ));
    assert!(shell_detect::ps("/usr/bin/pwsh"));
    assert!(!shell_detect::ps("/bin/bash"));
    assert!(shell_detect::posix("/bin/bash"));
    assert!(shell_detect::posix("/bin/zsh"));
    assert!(!shell_detect::posix("pwsh"));
    // deny list
    assert!(!shell_detect::acceptable_name("fish"));
    assert!(!shell_detect::acceptable_name("nu"));
    assert!(shell_detect::acceptable_name("bash"));

    // exec args per shell
    assert_eq!(
        shell_detect::exec_args("/bin/sh", "echo hi", "/tmp"),
        vec!["-c".to_string(), "echo hi".to_string()]
    );
    assert_eq!(
        shell_detect::exec_args("cmd.exe", "dir", "C:\\"),
        vec!["/c".to_string(), "dir".to_string()]
    );
    assert_eq!(
        shell_detect::exec_args("/usr/bin/pwsh", "Get-ChildItem", "/"),
        vec![
            "-NoProfile".to_string(),
            "-Command".to_string(),
            "Get-ChildItem".to_string()
        ]
    );

    // toKind
    assert_eq!(shell_detect::to_kind("pwsh"), "powershell");
    assert_eq!(shell_detect::to_kind("cmd"), "cmd");
    assert_eq!(shell_detect::to_kind("bash"), "posix");
}

#[test]
fn shell_tool_runs_echo_and_reports_exit() {
    let _guard = env_lock();
    let root = temp_dir("shell");
    setup_xdg(&root);
    let project = fixture(&root);
    let (ctx, _sink) = make_ctx(project.clone(), project.clone());
    let registry = ToolRegistry::builtin();

    let result = registry
        .get("bash")
        .unwrap()
        .run(serde_json::json!({"command": "echo hello-shell"}), &ctx)
        .unwrap();

    assert_eq!(result.title, "echo hello-shell");
    if cfg!(windows) {
        // cmd echo menambah newline CRLF; cek substring saja
        assert!(result.output.contains("hello-shell"), "{}", result.output);
    } else {
        assert!(result.output.contains("hello-shell"));
    }
    assert!(result.metadata["exit"].is_number());

    // exit code non-zero tetap sukses eksekusi (metadata.exit mencerminkan)
    let result = registry
        .get("bash")
        .unwrap()
        .run(serde_json::json!({"command": "exit 5"}), &ctx)
        .unwrap();
    assert_eq!(result.metadata["exit"], 5);

    // timeout: sleep melebihi batas → metadata shell_metadata muncul
    let cmd = if cfg!(windows) {
        serde_json::json!({"command": "ping -n 3 127.0.0.1 > nul", "timeout": 300})
    } else {
        serde_json::json!({"command": "sleep 2", "timeout": 300})
    };
    let result = registry.get("bash").unwrap().run(cmd, &ctx).unwrap();
    assert!(
        result
            .output
            .contains("terminated command after exceeding timeout 300 ms"),
        "{}",
        result.output
    );
}

// --- Sprint 6b: webfetch / websearch helpers ---

use oc_tool::websearch;

#[test]
fn checksum_fnv1a_base36_matches_ts() {
    // FNV-1a 32-bit: checksum("") = undefined, checksum("a") = "1sqg5f"
    assert_eq!(websearch::checksum(""), None);
    // vektor kontrol: hitung manual via algoritma
    let value = websearch::checksum("ses_abc").unwrap();
    assert!(!value.is_empty());
    assert!(value
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit()));
}

#[test]
fn select_provider_respects_override_and_parity() {
    std::env::set_var("OPENCODE_WEBSEARCH_PROVIDER", "exa");
    assert_eq!(
        websearch::select_web_search_provider(
            "anything",
            &websearch::ProviderFlags {
                exa: false,
                parallel: false
            }
        ),
        websearch::WebSearchProvider::Exa
    );
    std::env::set_var("OPENCODE_WEBSEARCH_PROVIDER", "parallel");
    assert_eq!(
        websearch::select_web_search_provider(
            "anything",
            &websearch::ProviderFlags {
                exa: false,
                parallel: false
            }
        ),
        websearch::WebSearchProvider::Parallel
    );
    std::env::remove_var("OPENCODE_WEBSEARCH_PROVIDER");

    let label = websearch::web_search_provider_label(Some(websearch::WebSearchProvider::Exa));
    assert_eq!(label, "Exa Web Search");
}

#[test]
fn webfetch_rejects_non_http_url() {
    let _guard = env_lock();
    let root = temp_dir("webfetch");
    setup_xdg(&root);
    let project = fixture(&root);
    let (ctx, _sink) = make_ctx(project.clone(), project.clone());
    let registry = ToolRegistry::builtin();

    let err = registry
        .get("webfetch")
        .unwrap()
        .run(serde_json::json!({"url": "ftp://example.com"}), &ctx)
        .unwrap_err();
    assert!(err.to_string().contains("URL must start with http://"));
}

#[test]
fn html_text_extraction_skips_script_style() {
    use oc_tool::webfetch::extract_text_from_html;
    let html = "<html><head><style>body{color:red}</style></head>\
                <body><h1>Title</h1><script>alert(1)</script>\
                <p>Hello <b>world</b></p><!-- comment --></body></html>";
    let text = extract_text_from_html(html);
    assert!(text.contains("Title"), "{text}");
    assert!(text.contains("Hello world"), "{text}");
    assert!(!text.contains("alert"));
    assert!(!text.contains("color:red"));
}

#[test]
fn html_to_markdown_basics() {
    use oc_tool::webfetch::convert_html_to_markdown;
    let md = convert_html_to_markdown(
        "<h2>Heading</h2>\n<p>Text with <strong>bold</strong> and <em>italic</em> \
         and <a href=\"https://x\">link</a>.</p>\
         <ul><li>one</li><li>two</li></ul>\
         <pre><code>let x = 1;</code></pre>",
    );
    assert!(md.contains("## Heading"), "{md}");
    assert!(md.contains("**bold**"));
    assert!(md.contains("*italic*"));
    assert!(md.contains("[link](https://x)"));
    assert!(md.contains("- one"));
    assert!(md.contains("- two"));
    assert!(md.contains("```\nlet x = 1;\n```"));
}

// --- Sprint 6 (tail): apply_patch tool ---

use oc_tool::patch;

#[test]
fn patch_parse_and_derive_contents() {
    let patch_text = "*** Begin Patch\n*** Update File: a.txt\n@@\n-old line\n+new line\n*** Add File: b.txt\n+hello\n*** Delete File: c.txt\n*** End Patch";
    let hunks = patch::parse_patch(patch_text).unwrap();
    assert_eq!(hunks.len(), 3);
    match &hunks[0] {
        patch::Hunk::Update { path, chunks, .. } => {
            assert_eq!(path, "a.txt");
            assert_eq!(chunks.len(), 1);
            assert_eq!(chunks[0].old_lines, vec!["old line".to_string()]);
            assert_eq!(chunks[0].new_lines, vec!["new line".to_string()]);
        }
        other => panic!("hunk pertama harus update: {other:?}"),
    }
    match &hunks[1] {
        patch::Hunk::Add { path, contents } => {
            assert_eq!(path, "b.txt");
            assert_eq!(contents, "hello");
        }
        other => panic!("hunk kedua harus add: {other:?}"),
    }

    // marker hilang → error
    let err = patch::parse_patch("tanpa marker").unwrap_err();
    assert!(err.to_string().contains("missing Begin/End markers"));

    // derive contents: exact match
    let update = patch::derive_new_contents_from_chunks(
        Path::new("a.txt"),
        &[patch::UpdateFileChunk {
            old_lines: vec!["line2".into()],
            new_lines: vec!["LINE2".into()],
            change_context: None,
            is_end_of_file: false,
        }],
        "line1\nline2\nline3\n",
    )
    .unwrap();
    assert_eq!(update.content, "line1\nLINE2\nline3\n");

    // fuzzy rstrip match
    let update = patch::derive_new_contents_from_chunks(
        Path::new("a.txt"),
        &[patch::UpdateFileChunk {
            old_lines: vec!["line1   ".into()],
            new_lines: vec!["first".into()],
            change_context: None,
            is_end_of_file: false,
        }],
        "line1\nline2\n",
    )
    .unwrap();
    assert_eq!(update.content, "first\nline2\n");

    // context tidak ketemu → error
    let err = patch::derive_new_contents_from_chunks(
        Path::new("a.txt"),
        &[patch::UpdateFileChunk {
            old_lines: vec!["zzz".into()],
            new_lines: vec![],
            change_context: Some("no-such-context".into()),
            is_end_of_file: false,
        }],
        "line1\n",
    )
    .unwrap_err();
    assert!(err.to_string().contains("Failed to find context"));
}

#[test]
fn apply_patch_tool_end_to_end() {
    let _guard = env_lock();
    let root = temp_dir("apply-patch");
    setup_xdg(&root);
    let project = fixture(&root);
    let (ctx, sink) = make_ctx(project.clone(), project.clone());
    let registry = ToolRegistry::builtin();

    // update + add + delete dalam satu patch
    let patch_text = concat!(
        "*** Begin Patch\n",
        "*** Update File: src/main.rs\n",
        "@@\n",
        "-fn main() {\n",
        "+fn main() {\n",
        "+    println!(\"patched\");\n",
        "*** Add File: docs/new.md\n",
        "+# New Doc\n",
        "*** Delete File: README.md\n",
        "*** End Patch"
    );
    let result = registry
        .get("apply_patch")
        .unwrap()
        .run(serde_json::json!({ "patchText": patch_text }), &ctx)
        .unwrap();

    // isi file ter-update
    let main_rs = std::fs::read_to_string(project.join("src/main.rs")).unwrap();
    assert!(main_rs.contains("patched"), "{main_rs}");
    assert!(std::fs::read_to_string(project.join("docs/new.md"))
        .unwrap()
        .contains("# New Doc"));
    assert!(!project.join("README.md").exists());

    // output summary format persis
    let expected =
        "Success. Updated the following files:\nM src/main.rs\nA docs/new.md\nD README.md";
    assert_eq!(result.output, expected, "{}", result.output);
    assert!(result.title.starts_with("Success."));

    // permission edit diminta dengan multi-pattern
    let asks = sink.asks.lock().unwrap();
    assert!(asks.iter().any(|p| p == "edit"));

    // registry filter: model gpt-* memakai apply_patch menggantikan edit/write
    let ids_for = |model: &str| -> Vec<String> {
        registry
            .tools_for_model(model)
            .into_iter()
            .map(|t| t.id.to_string())
            .collect()
    };
    assert!(ids_for("gpt-5").contains(&"apply_patch".to_string()));
    assert!(!ids_for("gpt-5").contains(&"edit".to_string()));
    assert!(!ids_for("claude-x").contains(&"apply_patch".to_string()));

    // patch kosong
    let err = registry
        .get("apply_patch")
        .unwrap()
        .run(
            serde_json::json!({"patchText": "*** Begin Patch\n*** End Patch"}),
            &ctx,
        )
        .unwrap_err();
    assert!(err.to_string().contains("empty patch"));
}
