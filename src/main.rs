mod config;
mod data;
mod net;
mod tui;

use crate::config::Config;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _config = Config::load()?;
    println!("mrss starting up");
    Ok(())
}
