use serde::Deserialize;

use crate::book::{Level, parse_scaled};
use crate::sync::{DiffEvent, Snapshot};

const WS_BASE: &str = "wss://stream.binance.com:9443/ws";
const REST_DEPTH: &str = "https://api.binance.com/api/v3/depth";

#[derive(Deserialize)]
struct RawDiff {
    #[serde(rename = "U")]
    first: u64,
    #[serde(rename = "u")]
    last: u64,
    #[serde(rename = "b")]
    bids: Vec<[String; 2]>,
    #[serde(rename = "a")]
    asks: Vec<[String; 2]>,
}

#[derive(Deserialize)]
struct RawSnapshot {
    #[serde(rename = "lastUpdateId")]
    last_update_id: u64,
    bids: Vec<[String; 2]>,
    asks: Vec<[String; 2]>,
}

fn levels(raw: &[[String; 2]]) -> Vec<Level> {
    raw.iter()
        .filter_map(|l| Some((parse_scaled(&l[0])?, parse_scaled(&l[1])?)))
        .collect()
}

pub fn stream_url(symbol: &str) -> String {
    format!("{WS_BASE}/{}@depth@100ms", symbol.to_lowercase())
}

pub fn snapshot_url(symbol: &str, limit: u32) -> String {
    format!(
        "{REST_DEPTH}?symbol={}&limit={limit}",
        symbol.to_uppercase()
    )
}

pub fn parse_diff(text: &str) -> Option<DiffEvent> {
    let raw: RawDiff = serde_json::from_str(text).ok()?;
    Some(DiffEvent {
        first_update_id: raw.first,
        final_update_id: raw.last,
        bids: levels(&raw.bids),
        asks: levels(&raw.asks),
    })
}

pub fn parse_snapshot(text: &str) -> Option<Snapshot> {
    let raw: RawSnapshot = serde_json::from_str(text).ok()?;
    Some(Snapshot {
        last_update_id: raw.last_update_id,
        bids: levels(&raw.bids),
        asks: levels(&raw.asks),
    })
}

pub async fn fetch_snapshot(symbol: &str, limit: u32) -> Result<Snapshot, reqwest::Error> {
    let raw: RawSnapshot = reqwest::get(snapshot_url(symbol, limit))
        .await?
        .error_for_status()?
        .json()
        .await?;
    Ok(Snapshot {
        last_update_id: raw.last_update_id,
        bids: levels(&raw.bids),
        asks: levels(&raw.asks),
    })
}
