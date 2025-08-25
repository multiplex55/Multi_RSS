#![allow(dead_code)]

//! Application configuration handling.

use directories::ProjectDirs;

/// Load configuration from the default location.
pub fn load() -> Option<toml::Value> {
    let dirs = ProjectDirs::from("com", "example", "mrss")?;
    let config_path = dirs.config_dir().join("config.toml");
    std::fs::read_to_string(config_path)
        .ok()
        .and_then(|s| toml::from_str(&s).ok())
}
