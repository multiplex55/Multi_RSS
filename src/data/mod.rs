#![allow(dead_code)]

//! Data models and persistence layer.

use std::{fs, io, path::PathBuf};

use directories::BaseDirs;
use feed_rs::model as feedmodel;
use log::error;
use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};
use std::collections::HashMap;

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
    #[serde(default)]
    pub etag: Option<String>,
    #[serde(default)]
    pub last_modified: Option<String>,
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

impl Feed {
    /// Merge parsed feed data into this feed, preserving read/queued flags.
    pub fn merge_items(&mut self, parsed: feedmodel::Feed) {
        // Update title if present
        if let Some(title) = parsed.title {
            self.title = title.content;
        }

        // Map existing items by id to preserve state
        let existing: HashMap<String, Item> = self
            .items
            .iter()
            .cloned()
            .map(|i| (i.id.clone(), i))
            .collect();

        let mut new_items = Vec::new();
        for entry in parsed.entries {
            let link = entry
                .links
                .first()
                .map(|l| l.href.clone())
                .unwrap_or_default();
            let id = Item::gen_id(Some(&entry.id), &link);

            let mut item = Item {
                id: id.clone(),
                title: entry
                    .title
                    .as_ref()
                    .map(|t| t.content.clone())
                    .unwrap_or_default(),
                link,
                desc: entry
                    .summary
                    .as_ref()
                    .map(|s| s.content.clone())
                    .unwrap_or_default(),
                timestamp: entry
                    .published
                    .or(entry.updated)
                    .map(|d| d.timestamp())
                    .unwrap_or_default(),
                read: false,
                queued: false,
            };

            if let Some(old) = existing.get(&id) {
                item.read = old.read;
                item.queued = old.queued;
            }

            new_items.push(item);
        }

        // Newest first by timestamp
        new_items.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        self.items = new_items;
    }
}

impl Group {
    /// Recalculate unread count for the group.
    pub fn update_unread(&mut self) {
        self.unread_count = self
            .feeds
            .iter()
            .map(|f| f.items.iter().filter(|i| !i.read).count())
            .sum();
    }
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
    if let Some(parent) = path.parent()
        && let Err(e) = fs::create_dir_all(parent)
    {
        error!("Failed to create {}: {}", parent.display(), e);
        return Err(e);
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
