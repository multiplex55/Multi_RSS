mod config;
mod data;
mod net;
mod tui;

use crate::config::Config;
use chrono::Utc;
use std::{
    sync::{Arc, Mutex, mpsc},
    thread,
    time::Duration,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::load()?;
    let groups = Arc::new(Mutex::new(data::load_db().unwrap_or_default()));
    let (tx, rx) = mpsc::channel();
    let interval = config.refresh.interval_secs;
    let groups_clone = Arc::clone(&groups);
    thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        loop {
            let mut new_items = 0;
            rt.block_on(async {
                let mut guard = groups_clone.lock().unwrap();
                for group in guard.iter_mut() {
                    for feed in group.feeds.iter_mut() {
                        let prev = feed.items.len();
                        if let Ok((etag, last, Some(parsed))) = net::fetch_feed(
                            &feed.url,
                            feed.etag.as_deref(),
                            feed.last_modified.as_deref(),
                        )
                        .await
                        {
                            feed.etag = etag;
                            feed.last_modified = last;
                            feed.merge_items(parsed);
                            if feed.items.len() > prev {
                                new_items += feed.items.len() - prev;
                            }
                        }
                    }
                    group.update_unread();
                }
            });
            let _ = tx.send((Utc::now(), new_items));
            thread::sleep(Duration::from_secs(interval));
        }
    });

    let mut app = tui::AppState::new(config, groups, rx);
    tui::run_app(&mut app)?;
    Ok(())
}
