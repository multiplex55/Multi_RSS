//! Background refresh manager for feeds.

use std::{sync::Arc, time::Duration};

use tokio::{
    sync::{Mutex, mpsc},
    time,
};

use crate::data::Group;

use super::fetch_feed;

/// Spawn the refresh manager. The returned sender can be used to trigger a
/// manual refresh (e.g. when the user presses F5).
pub fn spawn_refresh_manager(db: Arc<Mutex<Vec<Group>>>) -> mpsc::Sender<()> {
    let (tx, mut rx) = mpsc::channel::<()>(1);

    tokio::spawn(async move {
        let mut ticker = time::interval(Duration::from_secs(900)); // 15min
        loop {
            tokio::select! {
                _ = ticker.tick() => {
                    refresh_all(&db).await;
                }
                Some(_) = rx.recv() => {
                    refresh_all(&db).await;
                }
            }
        }
    });

    tx
}

async fn refresh_all(db: &Arc<Mutex<Vec<Group>>>) {
    let mut guard = db.lock().await;
    for group in guard.iter_mut() {
        for feed in group.feeds.iter_mut() {
            if let Ok((etag, last, Some(parsed))) = fetch_feed(
                &feed.url,
                feed.etag.as_deref(),
                feed.last_modified.as_deref(),
            )
            .await
            {
                feed.etag = etag;
                feed.last_modified = last;
                feed.merge_items(parsed);
            }
        }
        group.update_unread();
    }
}
