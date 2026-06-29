use std::fs;

use l2sync::binance;
use l2sync::sync::{BookSync, SyncError};

const DIR: &str = "tests/fixtures";

fn read(name: &str) -> String {
    fs::read_to_string(format!("{DIR}/{name}")).expect("fixture")
}

#[test]
fn replay_recorded_binance_session() {
    let snapshot =
        binance::parse_snapshot(&read("binance_btcusdt_snapshot.json")).expect("snapshot");
    let verify = binance::parse_snapshot(&read("binance_btcusdt_verify.json")).expect("verify");
    let diffs = read("binance_btcusdt_diffs.ndjson");

    let mut state = BookSync::new();
    state.init(&snapshot);

    let mut applied = 0u64;
    let mut diff_at_verify = None;
    for line in diffs.lines() {
        let event = binance::parse_diff(line).expect("diff parse");
        match state.apply(&event) {
            Ok(()) => {}
            Err(SyncError::SnapshotTooOld {
                snapshot,
                first_event,
            }) => {
                panic!(
                    "snapshot {snapshot} predates a continuous recording (first event {first_event})"
                )
            }
            Err(SyncError::Gap { expected, got }) => {
                panic!("unexpected gap in continuous recording: expected {expected}, got {got}")
            }
        }
        if state.is_synced() {
            assert!(
                !state.book().crossed(),
                "book crossed after a synced update"
            );
            applied += 1;
            if diff_at_verify.is_none() && state.last_update_id() >= verify.last_update_id {
                diff_at_verify = Some(differential(&state, &verify));
            }
        }
    }

    assert!(
        state.is_synced(),
        "stream never synced against the snapshot"
    );
    assert!(applied > 0, "no updates applied after sync");

    let mismatches = diff_at_verify.expect("recording did not reach the verify snapshot");
    assert_eq!(
        mismatches, 0,
        "maintained book diverged from the REST snapshot on the top 40 levels"
    );
}

fn differential(state: &BookSync, verify: &l2sync::sync::Snapshot) -> usize {
    let mut mismatches = 0;
    for &(price, qty) in verify.bids.iter().take(20) {
        if state.book().bid_qty(price) != Some(qty) {
            mismatches += 1;
        }
    }
    for &(price, qty) in verify.asks.iter().take(20) {
        if state.book().ask_qty(price) != Some(qty) {
            mismatches += 1;
        }
    }
    mismatches
}
