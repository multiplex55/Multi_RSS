mod config;
mod data;
mod net;
mod tui;

use crate::config::Config;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::load()?;
    let groups = data::load_db().unwrap_or_default();
    let mut app = tui::AppState::new(config, groups);
    tui::run_app(&mut app)?;
    Ok(())
}
