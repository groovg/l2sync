use futures_util::StreamExt;
use serde::Deserialize;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;

const WS_URL: &str = "wss://stream.binance.com:9443/ws/btcusdt@depth@100ms";

#[derive(Debug, Deserialize)]
struct DepthDiff {
    #[serde(rename = "E")]
    event_time: u64,
    #[serde(rename = "U")]
    first_update_id: u64,
    #[serde(rename = "u")]
    final_update_id: u64,
    #[serde(rename = "b")]
    bids: Vec<[String; 2]>,
    #[serde(rename = "a")]
    asks: Vec<[String; 2]>,
}

fn best(levels: &[[String; 2]], want_max: bool) -> Option<&[String; 2]> {
    let mut chosen: Option<(&[String; 2], f64)> = None;
    for lvl in levels {
        let Ok(qty) = lvl[1].parse::<f64>() else {
            continue;
        };
        if qty <= 0.0 {
            continue;
        }
        let Ok(px) = lvl[0].parse::<f64>() else {
            continue;
        };
        let take = match chosen {
            Some((_, cur)) => {
                if want_max {
                    px > cur
                } else {
                    px < cur
                }
            }
            None => true,
        };
        if take {
            chosen = Some((lvl, px));
        }
    }
    chosen.map(|(lvl, _)| lvl)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("install rustls crypto provider");

    let (mut ws, _) = connect_async(WS_URL).await?;
    println!("connected {WS_URL}");

    while let Some(frame) = ws.next().await {
        let text = match frame? {
            Message::Text(t) => t,
            Message::Close(_) => break,
            _ => continue,
        };
        let Ok(diff) = serde_json::from_str::<DepthDiff>(text.as_str()) else {
            continue;
        };
        let (Some(bid), Some(ask)) = (best(&diff.bids, true), best(&diff.asks, false)) else {
            continue;
        };
        println!(
            "E={} U={} u={}  bid {} x {}  |  ask {} x {}",
            diff.event_time,
            diff.first_update_id,
            diff.final_update_id,
            bid[0],
            bid[1],
            ask[0],
            ask[1],
        );
    }
    Ok(())
}
