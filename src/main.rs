mod config;
mod data;
mod net;
mod tui;

use crate::config::Config;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::net::refresh;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _config = Config::load()?;
    let db = data::load_db().unwrap_or_default();
    let db = Arc::new(Mutex::new(db));

    // Spawn refresh manager; the returned sender can be triggered on F5.
    let _refresh_trigger = refresh::spawn_refresh_manager(db.clone());

    println!("mrss starting up");
    Ok(())
}
