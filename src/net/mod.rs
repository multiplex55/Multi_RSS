#![allow(dead_code)]

//! Networking and feed fetching utilities.

use feed_rs::parser;
use reqwest::{Client, StatusCode, header};

/// Fetch a feed from the network respecting HTTP caching headers.
///
/// `etag` and `last_modified` are previously cached header values. If the
/// remote server returns `304 Not Modified`, `None` will be returned for the
/// feed data. The returned tuple contains the new header values along with the
/// optional parsed feed.
pub async fn fetch_feed(
    url: &str,
    etag: Option<&str>,
    last_modified: Option<&str>,
) -> Result<
    (Option<String>, Option<String>, Option<feed_rs::model::Feed>),
    Box<dyn std::error::Error>,
> {
    let client = Client::builder().build()?;
    let mut req = client.get(url);
    if let Some(et) = etag {
        req = req.header(header::IF_NONE_MATCH, et);
    }
    if let Some(lm) = last_modified {
        req = req.header(header::IF_MODIFIED_SINCE, lm);
    }

    let resp = req.send().await?;

    let new_etag = resp
        .headers()
        .get(header::ETAG)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());
    let new_last = resp
        .headers()
        .get(header::LAST_MODIFIED)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    if resp.status() == StatusCode::NOT_MODIFIED {
        return Ok((
            new_etag.or(etag.map(|s| s.to_string())),
            new_last.or(last_modified.map(|s| s.to_string())),
            None,
        ));
    }

    let bytes = resp.bytes().await?;
    let feed = parser::parse(&bytes[..])?;
    Ok((new_etag, new_last, Some(feed)))
}

pub mod refresh;
