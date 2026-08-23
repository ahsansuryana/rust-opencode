//! Ported from: packages/opencode/src/config/managed.ts

use std::path::PathBuf;

use serde_json::{Map, Value};

/// Ported from: packages/opencode/src/config/managed.ts:8 (MANAGED_PLIST_DOMAIN)
#[allow(dead_code)] // hanya dipakai jalur darwin, meniru TS
const MANAGED_PLIST_DOMAIN: &str = "ai.opencode.managed";

/// Ported from: packages/opencode/src/config/managed.ts:11-18 (PLIST_META)
const PLIST_META: &[&str] = &[
    "PayloadDisplayName",
    "PayloadIdentifier",
    "PayloadType",
    "PayloadUUID",
    "PayloadVersion",
    "_manualProfile",
];

/// Ported from: packages/opencode/src/config/managed.ts:20-29 (systemManagedConfigDir)
fn system_managed_config_dir() -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        PathBuf::from("/Library/Application Support/opencode")
    }
    #[cfg(windows)]
    {
        let program_data =
            std::env::var("ProgramData").unwrap_or_else(|_| "C:\\ProgramData".to_string());
        PathBuf::from(program_data).join("opencode")
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        PathBuf::from("/etc/opencode")
    }
}

/// Ported from: packages/opencode/src/config/managed.ts:31-33 (managedConfigDir)
pub fn managed_config_dir() -> PathBuf {
    match std::env::var("OPENCODE_TEST_MANAGED_CONFIG_DIR") {
        Ok(dir) => PathBuf::from(dir),
        Err(_) => system_managed_config_dir(),
    }
}

/// Ported from: packages/opencode/src/config/managed.ts:35-41 (parseManagedPlist)
pub fn parse_managed_plist(json: &str) -> Result<String, serde_json::Error> {
    let raw: Map<String, Value> = serde_json::from_str(json)?;
    let mut filtered = Map::new();
    for (key, value) in raw {
        if !PLIST_META.contains(&key.as_str()) {
            filtered.insert(key, value);
        }
    }
    serde_json::to_string(&Value::Object(filtered))
}

/// Hasil readManagedPreferences.
pub struct ManagedPreferences {
    pub source: String,
    pub text: String,
}

/// Ported from: packages/opencode/src/config/managed.ts:43-69 (readManagedPreferences)
/// darwin-only; platform lain mengembalikan None persis seperti source asli.
pub fn read_managed_preferences() -> Option<ManagedPreferences> {
    if !cfg!(target_os = "macos") {
        return None;
    }

    #[cfg(target_os = "macos")]
    {
        let user = whoami::username();
        let username = if user.is_empty() {
            "user".to_string()
        } else {
            user
        };
        let paths = [
            PathBuf::from("/Library/Managed Preferences")
                .join(&username)
                .join(format!("{MANAGED_PLIST_DOMAIN}.plist")),
            PathBuf::from("/Library/Managed Preferences")
                .join(format!("{MANAGED_PLIST_DOMAIN}.plist")),
        ];
        for plist in paths {
            if !plist.exists() {
                continue;
            }
            // Process.run(["plutil","-convert","json","-o","-",plist], nothrow)
            let result = std::process::Command::new("plutil")
                .args(["-convert", "json", "-o", "-"])
                .arg(&plist)
                .output();
            let Ok(output) = result else { continue };
            if !output.status.success() {
                continue;
            }
            let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
            let text = parse_managed_plist(&stdout).ok()?;
            return Some(ManagedPreferences {
                source: format!("mobileconfig:{}", plist.display()),
                text,
            });
        }
        None
    }

    #[cfg(not(target_os = "macos"))]
    {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_managed_plist_strips_meta_keys_and_keeps_order() {
        let json = r#"{"PayloadType":"x","shell":"zsh","model":"m/m"}"#;
        let out = parse_managed_plist(json).unwrap();
        assert_eq!(out, r#"{"shell":"zsh","model":"m/m"}"#);
    }
}
