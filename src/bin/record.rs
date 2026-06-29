use std::fs;
use std::path::Path;

use futures_util::StreamExt;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;

use l2sync::binance;

const SYMBOL: &str = "BTCUSDT";
const OUT_DIR: &str = "tests/fixtures";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("install rustls crypto provider");

    let (mut ws, _) = connect_async(binance::stream_url(SYMBOL)).await?;
    let mut diffs: Vec<String> = Vec::new();

    read_frames(&mut ws, &mut diffs, 15).await?;

    let snapshot = http_text(&binance::snapshot_url(SYMBOL, 1000)).await?;
    read_frames(&mut ws, &mut diffs, 60).await?;

    let verify = http_text(&binance::snapshot_url(SYMBOL, 1000)).await?;
    let verify_id = binance::parse_snapshot(&verify)
        .ok_or("verify snapshot parse")?
        .last_update_id;
    read_until(&mut ws, &mut diffs, verify_id).await?;

    let dir = Path::new(OUT_DIR);
    fs::create_dir_all(dir)?;
    fs::write(dir.join("binance_btcusdt_snapshot.json"), &snapshot)?;
    fs::write(dir.join("binance_btcusdt_verify.json"), &verify)?;
    fs::write(dir.join("binance_btcusdt_diffs.ndjson"), diffs.join("\n"))?;

    println!(
        "captured {} diffs; verify lastUpdateId={verify_id}",
        diffs.len()
    );
    Ok(())
}

async fn http_text(url: &str) -> Result<String, reqwest::Error> {
    reqwest::get(url).await?.error_for_status()?.text().await
}

type Ws =
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;

async fn read_frames(
    ws: &mut Ws,
    out: &mut Vec<String>,
    count: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut got = 0;
    while got < count {
        match ws.next().await.ok_or("stream closed")?? {
            Message::Text(t) => {
                out.push(t.as_str().to_owned());
                got += 1;
            }
            Message::Close(_) => return Err("stream closed".into()),
            _ => {}
        }
    }
    Ok(())
}

async fn read_until(
    ws: &mut Ws,
    out: &mut Vec<String>,
    target_id: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    loop {
        match ws.next().await.ok_or("stream closed")?? {
            Message::Text(t) => {
                let text = t.as_str().to_owned();
                let reached =
                    binance::parse_diff(&text).is_some_and(|d| d.final_update_id >= target_id);
                out.push(text);
                if reached {
                    return Ok(());
                }
            }
            Message::Close(_) => return Err("stream closed".into()),
            _ => {}
        }
    }
}
