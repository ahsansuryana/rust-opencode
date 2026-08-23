//! Test oc-storage: CRUD hierarkis, key→path mapping, dan migration runner
//! (golden: layout legacy → layout baru, marker file, idempotency).

use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock};

use oc_storage::StorageService;

fn env_lock() -> MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(())).lock().unwrap()
}

fn temp_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("oc-storage-{tag}-{}", std::process::id()));
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

#[test]
fn key_to_path_mapping_and_crud() {
    let _guard = env_lock();
    let root = temp_dir("crud");
    setup_xdg(&root);

    let storage = StorageService::new().unwrap();
    let base = root.join(".local/share/opencode/storage");

    // write → path join + ".json", pretty 2-space
    storage
        .write(
            &["session".into(), "abc".into()],
            &serde_json::json!({"id": "abc"}),
        )
        .unwrap();
    let written = std::fs::read_to_string(base.join("session/abc.json")).unwrap();
    assert_eq!(written, "{\n  \"id\": \"abc\"\n}");

    let read: serde_json::Value = storage.read(&["session".into(), "abc".into()]).unwrap();
    assert_eq!(read["id"], "abc");

    // update memutasi draft lalu menulis ulang
    let updated: serde_json::Value = storage
        .update::<serde_json::Value, _>(
            &["session".into(), "abc".into()],
            |draft: &mut serde_json::Value| {
                draft["title"] = serde_json::json!("hello");
            },
        )
        .unwrap();
    assert_eq!(updated["title"], "hello");

    // read missing → NotFoundError dgn pesan target
    match storage.read::<serde_json::Value>(&["nope".into()]) {
        Err(oc_storage::Error::NotFound(not_found)) => {
            assert!(not_found.message.contains("Resource not found"));
            assert!(not_found.message.contains("nope.json"));
        }
        other => panic!("harus NotFound, dapat {other:?}"),
    }

    // remove idempotent
    storage.remove(&["session".into(), "abc".into()]).unwrap();
    storage.remove(&["session".into(), "abc".into()]).unwrap();
    assert!(!base.join("session/abc.json").exists());
}

#[test]
fn list_returns_sorted_keys_without_json_suffix() {
    let _guard = env_lock();
    let root = temp_dir("list");
    setup_xdg(&root);

    let storage = StorageService::new().unwrap();
    for id in ["b", "a"] {
        storage
            .write(&["session".into(), id.into()], &serde_json::json!({}))
            .unwrap();
    }
    storage
        .write(
            &["message".into(), "m1".into(), "p1".into()],
            &serde_json::json!({}),
        )
        .unwrap();

    let keys = storage.list(&["session".into()]).unwrap();
    assert_eq!(
        keys,
        vec![
            vec!["session".to_string(), "a".to_string()],
            vec!["session".to_string(), "b".to_string()],
        ]
    );

    let keys = storage.list(&[]).unwrap();
    assert_eq!(
        keys,
        vec![
            vec!["message".to_string(), "m1".to_string(), "p1".to_string()],
            vec!["session".to_string(), "a".to_string()],
            vec!["session".to_string(), "b".to_string()],
        ]
    );
}

/// Golden migration runner: bangun layout legacy persis seperti yang dibaca
/// migration 1 (project/<dir>/storage/**) dengan worktree berupa repo git
/// sungguhan, jalankan StorageService::new(), bandingkan hasil akhir.
#[test]
fn migrations_run_legacy_layout_to_new_layout_idempotently() {
    let _guard = env_lock();
    let root = temp_dir("migration");
    setup_xdg(&root);

    let data_dir = root.join(".local/share/opencode");
    let worktree = root.join("worktree");
    std::fs::create_dir_all(worktree.join("sub/dir")).unwrap();
    run_git(&worktree, &["init"]);
    run_git(&worktree, &["config", "user.email", "t@t"]);
    run_git(&worktree, &["config", "user.name", "t"]);
    std::fs::write(worktree.join("file.txt"), "x\n").unwrap();
    run_git(&worktree, &["add", "."]);
    run_git(&worktree, &["commit", "-q", "-m", "init"]);
    let expected_root_commit = git_root_commit(&worktree).unwrap();

    // legacy layout: <data>/project/<somedir>/storage/session/{info,message,part}/...
    let legacy = data_dir.join("project/legacy-app/storage/session");
    std::fs::create_dir_all(legacy.join("info")).unwrap();
    std::fs::write(
        legacy.join("info/ses_1.json"),
        r#"{"id":"ses_1","title":"t"}"#,
    )
    .unwrap();
    std::fs::create_dir_all(legacy.join("message/ses_1")).unwrap();
    std::fs::write(
        legacy.join("message/ses_1/msg_1.json"),
        r#"{"id":"msg_1","role":"user"}"#,
    )
    .unwrap();
    std::fs::create_dir_all(legacy.join("part/ses_1/msg_1")).unwrap();
    std::fs::write(
        legacy.join("part/ses_1/msg_1/part_1.json"),
        r#"{"id":"part_1"}"#,
    )
    .unwrap();

    // pesan pertama membawa path.root menuju worktree (dipakai utk rev-list)
    std::fs::create_dir_all(legacy.join("message/ses_1")).unwrap();
    std::fs::write(
        legacy.join("message/ses_1/msg_0.json"),
        format!(
            r#"{{"id":"msg_0","path":{{"root":{}}}}}"#,
            serde_json::to_string(worktree.to_str().unwrap()).unwrap()
        ),
    )
    .unwrap();

    // layout sesuai POST-migration-2 juga disiapkan utk cek migration 2:
    // session/<pid>/<sid>.json dengan summary.diffs
    // (dibuat SETELAH service pertama? Tidak — migration 2 jalan di init yang
    // sama; kita uji lewat instance kedua di bawah.)

    let storage = StorageService::new().unwrap();
    let store = data_dir.join("storage");

    // marker = 2 (semua migrasi sukses)
    assert_eq!(
        std::fs::read_to_string(store.join("migration"))
            .unwrap()
            .trim(),
        "2"
    );

    // project/<root-commit>.json dibuat oleh migration 1
    let project_file = store
        .join("project")
        .join(format!("{expected_root_commit}.json"));
    let project: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&project_file).unwrap()).unwrap();
    assert_eq!(project["vcs"], "git");
    assert_eq!(project["worktree"], worktree.to_string_lossy().as_ref());

    // session/message/part tersalin
    assert!(store
        .join("session")
        .join(&expected_root_commit)
        .join("ses_1.json")
        .exists());
    assert!(store.join("message/ses_1/msg_0.json").exists());
    assert!(store.join("message/ses_1/msg_1.json").exists());
    assert!(store.join("part/msg_1/part_1.json").exists());

    // migration 2 pada state yang sama: seed summary lalu instance BARU
    // (cached state sudah jalan; buat layout post-migration-1 lalu reset)
    drop(storage);

    let session_dir = store.join("session").join(&expected_root_commit);
    std::fs::write(
        session_dir.join("ses_9.json"),
        r#"{"id":"ses_9","projectID":"PID","summary":{"diffs":[{"additions":3,"deletions":1},{"additions":2,"deletions":5}]}}"#,
    )
    .unwrap();
    // turunkan marker ke 1 supaya hanya migration 2 yang dijalankan ulang
    std::fs::write(store.join("migration"), "1").unwrap();

    let storage2 = StorageService::new().unwrap();
    assert_eq!(
        std::fs::read_to_string(store.join("migration"))
            .unwrap()
            .trim(),
        "2"
    );
    let diff: serde_json::Value = storage2
        .read(&["session_diff".into(), "ses_9".into()])
        .unwrap();
    assert_eq!(
        diff,
        serde_json::json!([{"additions":3,"deletions":1},{"additions":2,"deletions":5}])
    );
    let rewritten: serde_json::Value = storage2
        .read(&["session".into(), "PID".into(), "ses_9".into()])
        .unwrap();
    assert_eq!(
        rewritten["summary"],
        serde_json::json!({"additions":5,"deletions":6})
    );

    // idempotency: instance ketiga tidak mengubah apa pun (marker tetap 2)
    let before = walk_checksum(&store);
    let _storage3 = StorageService::new().unwrap();
    let after = walk_checksum(&store);
    assert_eq!(before, after, "run kedua harus no-op");
}

fn run_git(dir: &Path, args: &[&str]) {
    let status = std::process::Command::new("git")
        .args(args)
        .current_dir(dir)
        .status()
        .expect("git harus terpasang untuk test ini");
    assert!(status.success(), "git {args:?} gagal");
}

fn git_root_commit(dir: &Path) -> Option<String> {
    let output = std::process::Command::new("git")
        .args(["rev-list", "--max-parents=0", "--all"])
        .current_dir(dir)
        .output()
        .ok()?;
    let mut lines: Vec<String> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|l| !l.is_empty())
        .map(|l| l.trim().to_string())
        .collect();
    lines.sort();
    lines.into_iter().next()
}

fn walk_checksum(root: &Path) -> Vec<(String, u64)> {
    let mut out = Vec::new();
    fn visit(dir: &Path, out: &mut Vec<(String, u64)>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                visit(&path, out);
            } else if let Ok(meta) = entry.metadata() {
                out.push((path.to_string_lossy().replace('\\', "/"), meta.len()));
            }
        }
    }
    visit(root, &mut out);
    out.sort();
    out
}
