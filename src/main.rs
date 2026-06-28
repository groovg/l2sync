use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;

use l2sync::binance;
use l2sync::book::{Level, format_scaled};
use l2sync::sync::{BookSync, DiffEvent};

const SYMBOL: &str = "BTCUSDT";
const SNAPSHOT_DEPTH: u32 = 1000;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("install rustls crypto provider");

    let mut sessions: u64 = 0;
    loop {
        if let Err(e) = run(SYMBOL).await {
            sessions += 1;
            eprintln!("session ended ({e}); reconnecting and resyncing [#{sessions}]");
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
}

async fn run(symbol: &str) -> Result<(), Box<dyn std::error::Error>> {
    let (mut ws, _) = connect_async(binance::stream_url(symbol)).await?;
    println!("connected {symbol}; fetching snapshot");

    let snapshot = binance::fetch_snapshot(symbol, SNAPSHOT_DEPTH);
    tokio::pin!(snapshot);

    let mut state = BookSync::new();
    let mut buffered: Vec<DiffEvent> = Vec::new();
    let mut have_snapshot = false;
    let mut last_top: Option<(Level, Level)> = None;

    loop {
        tokio::select! {
            biased;
            snap = &mut snapshot, if !have_snapshot => {
                state.init(&snap?);
                let pending = std::mem::take(&mut buffered);
                for event in &pending {
                    state.apply(event)?;
                }
                have_snapshot = true;
                println!("snapshot applied (buffered diffs: {})", pending.len());
                check_consistent(&state)?;
                report(&state, &mut last_top);
            }
            frame = ws.next() => {
                let Some(frame) = frame else {
                    eprintln!("stream ended; reconnecting");
                    return Ok(());
                };
                match frame? {
                    Message::Text(text) => {
                        let Some(event) = binance::parse_diff(text.as_str()) else { continue };
                        if have_snapshot {
                            state.apply(&event)?;
                            check_consistent(&state)?;
                            report(&state, &mut last_top);
                        } else {
                            buffered.push(event);
                        }
                    }
                    Message::Ping(payload) => ws.send(Message::Pong(payload)).await?,
                    Message::Close(_) => {
                        eprintln!("server closed the stream; reconnecting");
                        return Ok(());
                    }
                    _ => {}
                }
            }
        }
    }
}

fn check_consistent(state: &BookSync) -> Result<(), Box<dyn std::error::Error>> {
    if state.is_synced() && state.book().crossed() {
        return Err("maintained book crossed; resyncing".into());
    }
    Ok(())
}

fn report(state: &BookSync, last_top: &mut Option<(Level, Level)>) {
    if !state.is_synced() {
        return;
    }
    let (Some(bid), Some(ask)) = (state.book().best_bid(), state.book().best_ask()) else {
        return;
    };
    if *last_top == Some((bid, ask)) {
        return;
    }
    *last_top = Some((bid, ask));
    let (bids, asks) = state.book().depth();
    println!(
        "bid {} x {}  |  ask {} x {}   [{bids}x{asks} levels]",
        format_scaled(bid.0),
        format_scaled(bid.1),
        format_scaled(ask.0),
        format_scaled(ask.1),
    );
}
