#![allow(dead_code)]

//! Application configuration handling.

use directories::BaseDirs;
use serde::{Deserialize, Serialize};

/// Global application configuration.
#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub ui: Ui,
    pub opener: Opener,
    pub keys: Keys,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Ui {
    pub theme: Theme,
    pub unread_only: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Opener {
    pub command: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Keys {
    pub quit: String,
    pub open: String,
    pub refresh: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    Dark,
    Light,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            ui: Ui::default(),
            opener: Opener::default(),
            keys: Keys::default(),
        }
    }
}

impl Default for Ui {
    fn default() -> Self {
        Self {
            theme: Theme::Dark,
            unread_only: true,
        }
    }
}

impl Default for Opener {
    fn default() -> Self {
        #[cfg(target_os = "windows")]
        {
            Self {
                command: "start".into(),
            }
        }
        #[cfg(target_os = "macos")]
        {
            Self {
                command: "open".into(),
            }
        }
        #[cfg(not(any(target_os = "windows", target_os = "macos")))]
        {
            Self {
                command: "xdg-open".into(),
            }
        }
    }
}

impl Default for Keys {
    fn default() -> Self {
        Self {
            quit: "q".into(),
            open: "o".into(),
            refresh: "r".into(),
        }
    }
}

impl Config {
    fn path() -> std::path::PathBuf {
        BaseDirs::new()
            .map(|d| d.config_dir().join("rssq").join("config.toml"))
            .unwrap_or_else(|| std::path::PathBuf::from("config.toml"))
    }

    /// Load configuration from disk, creating it with defaults if missing.
    pub fn load() -> std::io::Result<Self> {
        let path = Self::path();
        if let Ok(data) = std::fs::read_to_string(&path) {
            toml::from_str(&data)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
        } else {
            let cfg = Self::default();
            cfg.save()?;
            Ok(cfg)
        }
    }

    /// Persist configuration to disk.
    pub fn save(&self) -> std::io::Result<()> {
        let path = Self::path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let data = toml::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        std::fs::write(path, data)
    }
}
