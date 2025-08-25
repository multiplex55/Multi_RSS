#![allow(dead_code)]

//! Networking and feed fetching utilities.

use feed_rs::parser;
use reqwest::Client;

/// Fetch and parse a feed from a URL.
pub async fn fetch_feed(url: &str) -> Result<feed_rs::model::Feed, Box<dyn std::error::Error>> {
    let bytes = Client::new().get(url).send().await?.bytes().await?;
    let feed = parser::parse(&bytes[..])?;
    Ok(feed)
}
