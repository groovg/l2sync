use std::fmt;

use crate::book::{Level, OrderBook};

pub struct DiffEvent {
    pub first_update_id: u64,
    pub final_update_id: u64,
    pub bids: Vec<Level>,
    pub asks: Vec<Level>,
}

pub struct Snapshot {
    pub last_update_id: u64,
    pub bids: Vec<Level>,
    pub asks: Vec<Level>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum SyncError {
    SnapshotTooOld { snapshot: u64, first_event: u64 },
    Gap { expected: u64, got: u64 },
}

impl fmt::Display for SyncError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SyncError::SnapshotTooOld {
                snapshot,
                first_event,
            } => write!(
                f,
                "snapshot {snapshot} predates the stream (first event U={first_event})"
            ),
            SyncError::Gap { expected, got } => {
                write!(f, "sequence gap: expected U={expected}, got U={got}")
            }
        }
    }
}

impl std::error::Error for SyncError {}

#[derive(Default)]
pub struct BookSync {
    book: OrderBook,
    last_update_id: u64,
    synced: bool,
    started: bool,
    resyncs: u64,
}

impl BookSync {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn book(&self) -> &OrderBook {
        &self.book
    }

    pub fn is_synced(&self) -> bool {
        self.synced
    }

    pub fn last_update_id(&self) -> u64 {
        self.last_update_id
    }

    pub fn resyncs(&self) -> u64 {
        self.resyncs
    }

    pub fn init(&mut self, snapshot: &Snapshot) {
        if self.started {
            self.resyncs += 1;
        }
        self.started = true;
        self.book.reset(&snapshot.bids, &snapshot.asks);
        self.last_update_id = snapshot.last_update_id;
        self.synced = false;
    }

    pub fn apply(&mut self, event: &DiffEvent) -> Result<(), SyncError> {
        if event.final_update_id <= self.last_update_id {
            return Ok(());
        }
        if !self.synced {
            if event.first_update_id > self.last_update_id + 1 {
                return Err(SyncError::SnapshotTooOld {
                    snapshot: self.last_update_id,
                    first_event: event.first_update_id,
                });
            }
            self.book.apply(&event.bids, &event.asks);
            self.last_update_id = event.final_update_id;
            self.synced = true;
            return Ok(());
        }
        if event.first_update_id != self.last_update_id + 1 {
            return Err(SyncError::Gap {
                expected: self.last_update_id + 1,
                got: event.first_update_id,
            });
        }
        self.book.apply(&event.bids, &event.asks);
        self.last_update_id = event.final_update_id;
        Ok(())
    }
}
