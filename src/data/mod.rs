#![allow(dead_code)]

//! Data models and persistence layer.

use std::{fs, io, path::PathBuf};

use directories::BaseDirs;
use log::error;
use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};

/// RSS item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Item {
    pub id: String,
    pub title: String,
    pub link: String,
    #[serde(default)]
    pub desc: String,
    pub timestamp: i64,
    #[serde(default)]
    pub read: bool,
    #[serde(default)]
    pub queued: bool,
}

impl Item {
    /// Generate a stable 16-hex identifier from entry id or link.
    pub fn gen_id(id: Option<&str>, link: &str) -> String {
        let source = id.unwrap_or(link);
        let mut hasher = Sha1::new();
        hasher.update(source.as_bytes());
        let hash = hasher.finalize();
        let hex = format!("{:x}", hash);
        hex[..16].to_string()
    }
}

/// Feed containing multiple items.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Feed {
    pub url: String,
    pub title: String,
    #[serde(default)]
    pub items: Vec<Item>,
}

/// Grouping of feeds.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Group {
    pub name: String,
    #[serde(default)]
    pub feeds: Vec<Feed>,
    #[serde(default)]
    pub unread_count: usize,
}

/// Resolve path to the database json file.
fn db_path() -> Option<PathBuf> {
    BaseDirs::new().map(|b| b.data_dir().join("rssq").join("db.json"))
}

/// Load the database from disk.
pub fn load_db() -> io::Result<Vec<Group>> {
    let path = db_path().ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "data dir"))?;
    match fs::read_to_string(&path) {
        Ok(content) => serde_json::from_str(&content).map_err(|e| {
            error!("Failed to parse {}: {}", path.display(), e);
            io::Error::new(io::ErrorKind::InvalidData, e)
        }),
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            // No database yet.
            Ok(Vec::new())
        }
        Err(e) => {
            error!("Failed to read {}: {}", path.display(), e);
            Err(e)
        }
    }
}

/// Save the database to disk.
pub fn save_db(db: &[Group]) -> io::Result<()> {
    let path = db_path().ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "data dir"))?;
    if let Some(parent) = path.parent() {
        if let Err(e) = fs::create_dir_all(parent) {
            error!("Failed to create {}: {}", parent.display(), e);
            return Err(e);
        }
    }
    match serde_json::to_string_pretty(db) {
        Ok(json) => fs::write(&path, json).map_err(|e| {
            error!("Failed to write {}: {}", path.display(), e);
            e
        }),
        Err(e) => {
            error!("Failed to serialize db: {}", e);
            Err(io::Error::new(io::ErrorKind::InvalidData, e))
        }
    }
}
