//! Ported from: packages/opencode/src/storage/storage.ts
//! (dan subset FSUtil yang dipakainya — lihat NAMING_MAP.md).
//!
//! CATATAN: source asli adalah penyimpanan file JSON hierarkis, BUKAN SQLite
//! (asumsi sprint salah — lihat DEVIATIONS.md § technical notes).

pub mod fs_util;
pub mod migrations;

use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock, RwLock};

use oc_global::global;
use serde::Serialize;
use serde_json::Value;

/// Ported from: storage.ts:11-17 (NotFoundError)
#[derive(Debug)]
pub struct NotFoundError {
    pub message: String,
}

impl std::fmt::Display for NotFoundError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[NotFoundError] {}", self.message)
    }
}

impl std::error::Error for NotFoundError {}

/// Ported from: storage.ts:19 (type Error = FSUtil.Error | NotFoundError)
#[derive(Debug)]
pub enum Error {
    Fs(io::Error),
    NotFound(NotFoundError),
}

impl From<io::Error> for Error {
    fn from(value: io::Error) -> Self {
        if value.kind() == io::ErrorKind::NotFound {
            return Error::NotFound(NotFoundError {
                message: "Resource not found".to_string(),
            });
        }
        Error::Fs(value)
    }
}

impl From<NotFoundError> for Error {
    fn from(value: NotFoundError) -> Self {
        Error::NotFound(value)
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Fs(error) => write!(f, "fs error: {error}"),
            Error::NotFound(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for Error {}

/// Ported from: storage.ts:63-65 (file)
fn file_path(dir: &Path, key: &[String]) -> PathBuf {
    let mut path = dir.to_path_buf();
    for part in key {
        path.push(part);
    }
    path.with_extension("json")
}

/// Padanan `missing(err)` pada level io::Error.
fn is_missing_io(error: &io::Error) -> bool {
    error.kind() == io::ErrorKind::NotFound
}

/// Ported from: storage.ts:76-79 (parseMigration)
fn parse_migration(text: &str) -> usize {
    text.trim().parse::<usize>().unwrap_or(0)
}

/// Registry lock per target path (padanan RcMap + TxReentrantLock; simplifikasi
/// reentrant read/write fiber → RwLock per path — didokumentasikan di naming map).
fn lock_for(target: &Path) -> Arc<RwLock<()>> {
    static REGISTRY: OnceLock<Mutex<HashMap<PathBuf, Arc<RwLock<()>>>>> = OnceLock::new();
    let registry = REGISTRY.get_or_init(|| Mutex::new(HashMap::new()));
    let mut guard = registry.lock().unwrap();
    guard
        .entry(target.to_path_buf())
        .or_insert_with(|| Arc::new(RwLock::new(())))
        .clone()
}

/// Ported from: storage.ts:53-61 (Interface) dan 213-243 (layer/state init).
pub struct StorageService {
    dir: PathBuf,
}

impl StorageService {
    /// Konstruksi layer = jalankan migrasi sekali (Effect.cached).
    pub fn new() -> Result<Self, Error> {
        let dir = global::path().data.join("storage");
        ensure_migrated(&dir)?;
        Ok(StorageService { dir })
    }

    pub fn directory(&self) -> &Path {
        &self.dir
    }

    fn target(&self, key: &[String]) -> PathBuf {
        file_path(&self.dir, key)
    }

    fn read_value(&self, target: &Path) -> Result<Value, Error> {
        let lock = lock_for(target);
        let _lock = lock.read().unwrap();
        match read_json(target) {
            Ok(value) => Ok(value),
            Err(error) if is_missing_io(&error) => Err(Error::NotFound(NotFoundError {
                message: format!("Resource not found: {}", target.display()),
            })),
            Err(error) => Err(Error::Fs(error)),
        }
    }

    fn write_json_pretty(&self, target: &Path, content: &Value) -> Result<(), Error> {
        let lock = lock_for(target);
        let _lock = lock.write().unwrap();
        write_json(target, content).map_err(Error::from)
    }

    /// Ported from: storage.ts:266-270 (remove) — missing → void
    pub fn remove(&self, key: &[String]) -> Result<(), Error> {
        let target = self.target(key);
        let lock = lock_for(&target);
        let _lock = lock.write().unwrap();
        match std::fs::remove_file(&target) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(Error::from(error)),
        }
    }

    /// Ported from: storage.ts:272-278 (read<T>)
    pub fn read<T: serde::de::DeserializeOwned>(&self, key: &[String]) -> Result<T, Error> {
        let target = self.target(key);
        let value = self.read_value(&target)?;
        serde_json::from_value(value)
            .map_err(|e| Error::from(io::Error::new(io::ErrorKind::InvalidData, e.to_string())))
    }

    /// Ported from: storage.ts:280-294 (update<T>) — baca, mutasi draft, tulis.
    pub fn update<T, F>(&self, key: &[String], mutate: F) -> Result<T, Error>
    where
        T: serde::de::DeserializeOwned + Serialize,
        F: FnOnce(&mut T),
    {
        let target = self.target(key);
        let lock = lock_for(&target);
        let _lock = lock.write().unwrap();
        let content = match read_json(&target) {
            Ok(content) => content,
            Err(error) if is_missing_io(&error) => {
                return Err(Error::NotFound(NotFoundError {
                    message: format!("Resource not found: {}", target.display()),
                }))
            }
            Err(error) => return Err(Error::from(error)),
        };
        let mut value: T = serde_json::from_value(content)
            .map_err(|e| Error::from(io::Error::new(io::ErrorKind::InvalidData, e.to_string())))?;
        mutate(&mut value);
        let content = serde_json::to_value(&value)
            .map_err(|e| Error::from(io::Error::new(io::ErrorKind::InvalidData, e.to_string())))?;
        write_json(&target, &content).map_err(Error::from)?;
        Ok(value)
    }

    /// Ported from: storage.ts:296-299 (write)
    pub fn write<T: Serialize>(&self, key: &[String], content: &T) -> Result<(), Error> {
        let target = self.target(key);
        let value = serde_json::to_value(content)
            .map_err(|e| Error::from(io::Error::new(io::ErrorKind::InvalidData, e.to_string())))?;
        self.write_json_pretty(&target, &value)
    }

    /// Ported from: storage.ts:301-313 (list)
    pub fn list(&self, prefix: &[String]) -> Result<Vec<Vec<String>>, Error> {
        let cwd = {
            let mut path = self.dir.clone();
            for part in prefix {
                path.push(part);
            }
            path
        };
        let files = fs_util::glob_scan(&cwd, "**/*", true).unwrap_or_default();
        let mut keys: Vec<Vec<String>> = files
            .into_iter()
            .filter_map(|relative| {
                let text = relative.to_string_lossy().replace('\\', "/");
                // x.slice(0, -5): buang ".json"
                let stripped = text.strip_suffix(".json")?;
                let mut key = prefix.to_vec();
                key.extend(stripped.split('/').map(str::to_string));
                Some(key)
            })
            .collect();
        keys.sort_by_key(|key| key.join("/"));
        Ok(keys)
    }
}

fn read_json(path: &Path) -> io::Result<Value> {
    let text = std::fs::read_to_string(path)?;
    serde_json::from_str(&text)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))
}

fn write_json(path: &Path, content: &Value) -> io::Result<()> {
    let text = serde_json::to_string_pretty(content)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;
    fs_util::write_with_dirs(path, &text)
}

/// Ported dari blok state init (storage.ts:222-243):
/// baca marker, jalankan migrasi berurutan, tulis marker tiap sukses,
/// gagal → logError lalu break.
fn ensure_migrated(dir: &Path) -> Result<(), Error> {
    let marker = dir.join("migration");
    let migration = match std::fs::read_to_string(&marker) {
        Ok(text) => parse_migration(&text),
        Err(_) => 0,
    };
    const MIGRATION_COUNT: usize = 2;
    let runners: [fn(&Path) -> io::Result<()>; MIGRATION_COUNT] =
        [migrations::migration_1, migrations::migration_2];
    for (index, runner) in runners.iter().enumerate().skip(migration) {
        tracing::info!(index, "running migration");
        match runner(dir) {
            Ok(()) => {
                fs_util::write_with_dirs(&marker, &(index + 1).to_string())?;
            }
            Err(error) => {
                tracing::error!(index, cause = %error, "failed to run migration");
                break;
            }
        }
    }
    Ok(())
}
