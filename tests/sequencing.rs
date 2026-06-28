use l2sync::book::Level;
use l2sync::sync::{BookSync, DiffEvent, Snapshot, SyncError};

fn snap(last_update_id: u64, bids: &[Level], asks: &[Level]) -> Snapshot {
    Snapshot {
        last_update_id,
        bids: bids.to_vec(),
        asks: asks.to_vec(),
    }
}

fn diff(first: u64, last: u64, bids: &[Level], asks: &[Level]) -> DiffEvent {
    DiffEvent {
        first_update_id: first,
        final_update_id: last,
        bids: bids.to_vec(),
        asks: asks.to_vec(),
    }
}

#[test]
fn drops_stale_diffs_then_brackets_snapshot() {
    let mut s = BookSync::new();
    s.init(&snap(100, &[(50, 5), (49, 3)], &[(51, 2), (52, 4)]));

    s.apply(&diff(90, 99, &[(50, 9)], &[])).unwrap();
    assert!(!s.is_synced());
    assert_eq!(s.book().bid_qty(50), Some(5));

    s.apply(&diff(100, 101, &[(50, 7)], &[(51, 0)])).unwrap();
    assert!(s.is_synced());
    assert_eq!(s.last_update_id(), 101);
    assert_eq!(s.book().bid_qty(50), Some(7));
    assert_eq!(s.book().ask_qty(51), None);
}

#[test]
fn applies_contiguous_updates() {
    let mut s = BookSync::new();
    s.init(&snap(100, &[(50, 5)], &[(51, 2)]));
    s.apply(&diff(100, 101, &[(50, 6)], &[])).unwrap();
    s.apply(&diff(102, 103, &[(48, 1)], &[])).unwrap();
    s.apply(&diff(104, 110, &[], &[(53, 9)])).unwrap();
    assert_eq!(s.last_update_id(), 110);
    assert_eq!(s.book().bid_qty(48), Some(1));
    assert_eq!(s.book().ask_qty(53), Some(9));
}

#[test]
fn rejects_snapshot_older_than_stream() {
    let mut s = BookSync::new();
    s.init(&snap(100, &[(50, 5)], &[(51, 2)]));
    let err = s.apply(&diff(103, 105, &[(50, 6)], &[])).unwrap_err();
    assert_eq!(
        err,
        SyncError::SnapshotTooOld {
            snapshot: 100,
            first_event: 103
        }
    );
    assert!(!s.is_synced());
}

#[test]
fn detects_sequence_gap_and_recovers() {
    let mut s = BookSync::new();
    s.init(&snap(100, &[(50, 5), (49, 8)], &[(51, 2)]));
    s.apply(&diff(100, 101, &[(50, 6)], &[])).unwrap();
    s.apply(&diff(102, 103, &[(50, 7)], &[])).unwrap();
    assert_eq!(s.book().bid_qty(49), Some(8));

    let err = s.apply(&diff(110, 112, &[(50, 8)], &[])).unwrap_err();
    assert_eq!(
        err,
        SyncError::Gap {
            expected: 104,
            got: 110
        }
    );
    assert_eq!(s.resyncs(), 0);

    s.init(&snap(111, &[(50, 50)], &[(51, 3)]));
    assert_eq!(s.resyncs(), 1);
    assert!(!s.is_synced());
    assert_eq!(s.book().bid_qty(49), None);

    s.apply(&diff(111, 113, &[(50, 51)], &[])).unwrap();
    assert!(s.is_synced());
    assert_eq!(s.book().bid_qty(50), Some(51));
}

#[test]
fn recovers_from_repeated_gaps() {
    let mut s = BookSync::new();
    s.init(&snap(10, &[(50, 5)], &[(51, 2)]));
    s.apply(&diff(10, 11, &[(50, 6)], &[])).unwrap();

    assert!(s.apply(&diff(20, 21, &[(50, 7)], &[])).is_err());
    s.init(&snap(21, &[(50, 7)], &[(51, 2)]));
    assert_eq!(s.resyncs(), 1);
    s.apply(&diff(21, 22, &[(50, 8)], &[])).unwrap();
    assert!(s.is_synced());

    assert!(s.apply(&diff(40, 41, &[(50, 9)], &[])).is_err());
    s.init(&snap(41, &[(50, 9)], &[(51, 2)]));
    assert_eq!(s.resyncs(), 2);
    s.apply(&diff(41, 42, &[(50, 10)], &[])).unwrap();
    assert!(s.is_synced());
    assert_eq!(s.last_update_id(), 42);
    assert_eq!(s.book().bid_qty(50), Some(10));
}

#[test]
fn zero_quantity_removes_level() {
    let mut s = BookSync::new();
    s.init(&snap(10, &[(50, 5)], &[(51, 2)]));
    s.apply(&diff(10, 11, &[(50, 0)], &[])).unwrap();
    assert_eq!(s.book().bid_qty(50), None);
    s.apply(&diff(12, 12, &[(50, 4)], &[])).unwrap();
    assert_eq!(s.book().bid_qty(50), Some(4));
}

#[test]
fn book_stays_uncrossed_and_tracks_top() {
    let mut s = BookSync::new();
    s.init(&snap(0, &[(100, 5), (99, 4)], &[(101, 3), (102, 6)]));
    s.apply(&diff(0, 1, &[(100, 0)], &[(101, 0)])).unwrap();
    assert_eq!(s.book().best_bid(), Some((99, 4)));
    assert_eq!(s.book().best_ask(), Some((102, 6)));
    assert!(!s.book().crossed());
}
