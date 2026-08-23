//! Golden test Sprint 1 — membandingkan perilaku port Rust dengan output
//! PAKET TS ASLI yang menjadi acuan (jsonc-parser@3.3.1, remeda@2.26.0,
//! xdg-basedir@5.1.0). Fixture dihasilkan oleh tools/golden-gen/ (lihat
//! GENERATE.md).

use std::fs;
use std::path::{Path, PathBuf};

use oc_config::config::merge_deep;
use oc_config::parse::parse_fault_tolerant;

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/golden")
}

fn normalize_separators(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

#[test]
fn golden_jsonc_parser_matches_ts_package() {
    let dir = fixtures_dir().join("jsonc");
    let mut checked = 0;
    for entry in fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let fixture: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        let input = fixture["input"].as_str().unwrap();

        let (value, errors) = parse_fault_tolerant(input);

        let expected_errors = fixture["errors"].as_array().unwrap();
        assert_eq!(
            errors.len(),
            expected_errors.len(),
            "jumlah error beda untuk {path:?}"
        );
        for (actual, expected) in errors.iter().zip(expected_errors) {
            assert_eq!(
                oc_config::parse::print_parse_error_code(actual.0),
                expected["code"].as_str().unwrap(),
                "kode error beda untuk {path:?}"
            );
            assert_eq!(actual.1, expected["offset"].as_u64().unwrap() as usize);
        }

        if expected_errors.is_empty() {
            if fixture["value_undefined"].as_bool().unwrap_or(false) {
                assert_eq!(value, serde_json::Value::Null, "{path:?}");
            } else {
                assert_eq!(value, fixture["value"], "nilai parse beda untuk {path:?}");
            }
        }
        checked += 1;
    }
    assert!(
        checked >= 15,
        "fixture jsonc harus terbaca, dapat {checked}"
    );
}

#[test]
fn golden_merge_deep_matches_remeda() {
    let dir = fixtures_dir().join("merge");
    let mut checked = 0;
    for entry in fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let fixture: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        let merged = merge_deep(&fixture["target"], &fixture["source"]);
        assert_eq!(merged, fixture["merged"], "mergeDeep beda untuk {path:?}");
        checked += 1;
    }
    assert!(checked >= 8, "fixture merge harus terbaca, dapat {checked}");
}

/// Golden xdg-basedir: suffix path relatif home + semantik env kosong.
#[test]
fn golden_xdg_basedir_suffixes() {
    let fixture_path = fixtures_dir().join("xdg.json");
    let fixture: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&fixture_path).unwrap()).unwrap();

    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let _guard = LOCK.lock().unwrap();

    let root = std::env::temp_dir().join(format!("oc-golden-xdg-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();

    let set = |key: &str, value: Option<&str>| match value {
        Some(v) => std::env::set_var(key, v),
        None => std::env::remove_var(key),
    };

    for (scenario, expected) in fixture.as_object().unwrap() {
        let suffixes = &expected["suffixes"];
        set("HOME", Some(root.to_str().unwrap()));
        set("USERPROFILE", Some(root.to_str().unwrap()));
        set("XDG_DATA_HOME", None);
        set("XDG_CACHE_HOME", None);
        set("XDG_STATE_HOME", None);
        match scenario.as_str() {
            "custom_config_home" => {
                set(
                    "XDG_CONFIG_HOME",
                    Some(root.join("custom-config").to_str().unwrap()),
                );
            }
            _ => set("XDG_CONFIG_HOME", None),
        }
        oc_global::reset_for_tests();

        let paths = oc_global::path();
        let home_norm = normalize_separators(&root);
        let strip_app = |p: &Path| -> String {
            // buang komponen terakhir ("opencode") lalu normalisasi
            let base = p.parent().unwrap();
            normalize_separators(base)
                .strip_prefix(&home_norm)
                .unwrap_or(&normalize_separators(base))
                .to_string()
        };
        assert_eq!(strip_app(&paths.data), suffixes["data"], "{scenario} data");
        assert_eq!(
            strip_app(&paths.config),
            suffixes["config"],
            "{scenario} config"
        );
        assert_eq!(
            strip_app(&paths.state),
            suffixes["state"],
            "{scenario} state"
        );
        assert_eq!(
            strip_app(&paths.cache),
            suffixes["cache"],
            "{scenario} cache"
        );
    }

    oc_global::reset_for_tests();
}
