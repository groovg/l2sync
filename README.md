# l2sync

[![CI](https://github.com/groovg/l2sync/actions/workflows/ci.yml/badge.svg)](https://github.com/groovg/l2sync/actions/workflows/ci.yml)

Multi-exchange crypto **L2 order book aggregator** in async Rust. Connects to several
exchange depth feeds, maintains a correct local order book per venue with sequence-gap
detection and snapshot resync, and produces a normalized, consolidated cross-venue view.

This is a feed-handler study: the parts that are actually hard in production — out-of-order
updates, sequence gaps, snapshot+diff synchronization, reconnects, and heterogeneous message
formats, all under async backpressure.

> **Status: work in progress.** A single Binance venue is fully synchronized (correct book,
> gap detection, automatic resync). Multiple venues, a normalized model, and the consolidated
> cross-venue view are next. Not production-ready yet.

## Why a depth feed is not "just apply every update"

Exchanges publish the book as a REST **snapshot** plus a stream of **diff** events; each diff
carries only the levels that *changed* since the last one. The naive approach — connect to the
diff stream and treat each event as the book — is silently wrong, because a single diff is not
the book, only a delta against a snapshot you do not have yet.

The difference is stark. Printing the best bid/ask *within each raw diff event* gives nonsense,
because the highest bid present in one delta is not the real top of book:

```
# raw diff top-of-book (wrong)            # synchronized book (correct)
bid 59859.99  | ask 59860.00              bid 59774.01 x 1.31 | ask 59774.02 x 4.13
bid 53874.00  | ask 59863.95              bid 59774.01 x 1.63 | ask 59774.02 x 3.29
bid 59859.99  | ask 79666.01              bid 59777.39 x 5.19 | ask 59777.40 x 0.05
```

The raw column jumps around because deep levels happen to change; the synchronized column holds
a tight, correct one-cent spread. Getting from the left to the right requires the documented
handshake:

1. Open the diff stream and **buffer** events.
2. Fetch the REST depth snapshot (it carries a `lastUpdateId`).
3. Drop buffered diffs whose final id is `<= lastUpdateId`.
4. The first event to apply must bracket the snapshot: `U <= lastUpdateId + 1 <= u`. If the
   stream has already moved past the snapshot (`U > lastUpdateId + 1`), the snapshot is stale —
   refetch it.
5. From then on each event must be contiguous (`U == previous u + 1`). Any **sequence gap**
   discards the book and resyncs from step 2.

Quantities are absolute, not deltas: a level is overwritten, or removed when its quantity is 0.
Sequence semantics differ per venue (Binance `U`/`u`, Bybit `seq`, OKX checksum), so this
handshake lives behind a small core (`book` + `sync`) that the venue adapters feed.

## Correctness & testing

The sync core is pure and synchronous, so it is tested without any network:

- **Sequencing unit tests** (`tests/sequencing.rs`): stale-diff drop, snapshot bracketing,
  rejection of a snapshot older than the stream, sequence-gap detection and recovery, absolute
  quantity / zero-removal semantics, uncrossed-book invariant.
- **Recorded-session replay** (`tests/replay_binance.rs`): a real Binance session (snapshot +
  77 diff events + a second "verify" snapshot) is captured to `tests/fixtures/` by the
  `record` binary and replayed deterministically in CI. The test asserts the handshake
  succeeds, the continuous stream produces **zero gaps**, and the book never crosses.
- **Differential check:** at the update id of the independently-fetched verify snapshot, the
  maintained book's **top 40 levels** are compared against it. On the committed fixture they
  match **exactly (0 mismatches)** — the snapshot+diff pipeline reproduces the exchange's own
  top of book. Levels deeper than the REST snapshot's depth window are deliberately not
  cross-checked: a depth-limited snapshot and the live diff stream naturally diverge in the far
  tail (levels entering or leaving just inside the 1000-level boundary), which is expected, not
  a sync error.

Prices and quantities are parsed into exact scaled integers (no floating point), so levels and
crosses are never corrupted by rounding.

## Build & run

```sh
cargo run --release            # connect to Binance, print the synchronized BBO
cargo run --bin record         # re-capture the test fixture from a live session
cargo test                     # offline; runs against the committed fixture
```

The main binary connects to `btcusdt@depth@100ms`, performs the snapshot handshake, and prints
the top of book whenever it changes. On a gap or disconnect it reconnects and resyncs. Requires
network access to Binance.

## Design notes

- **Book core:** `BTreeMap<Price, Qty>` per side — best bid is the last key, best ask the first.
  Simple and obviously correct; a flat tick-indexed array would be faster at the touch and is a
  later swap.
- **Sync state machine:** a single `BookSync` owns the book, the last applied update id, and the
  synced/resync state; the async shell only does I/O (WebSocket + REST) and feeds it events.
- **Prices:** exchange-native decimal strings parsed to scaled `i64` ticks, exactly. Floats are
  deliberately avoided — rounding produces mismatched levels and phantom crosses. (The shared
  [`fixed-decimal`](https://github.com/groovg/fixed-decimal) type replaces the local scaled int
  once the normalized multi-venue model lands.)
- **Backpressure (planned):** the multi-venue shape is one `tokio` task per venue feeding a
  bounded `mpsc` to a single book engine — bounded buffering instead of unbounded growth under a
  burst. Today there is one venue and one task, so the channel is not wired yet.
- **TLS:** rustls with the `ring` provider, so the build has no system OpenSSL dependency.

## Roadmap

- [x] Single Binance venue: depth-diff stream, snapshot+diff handshake, gap detection, resync.
- [ ] Multiple venues behind a common `Venue` trait (Bybit, OKX) feeding one book engine.
- [ ] Normalized update/trade/snapshot model; adopt the shared [`fixed-decimal`](https://github.com/groovg/fixed-decimal) price type (one scale across venues, replacing the local scaled int).
- [ ] Consolidated cross-venue BBO and crossed-market (arb) detection.
- [ ] Tick-to-book latency percentiles, auto-reconnect with backoff, bounded-channel backpressure.

## Limitations (current)

- Single venue (Binance), single hardcoded symbol.
- Resync on a gap reconnects the socket and refetches the snapshot — correct but heavier than
  necessary (a lighter "keep the socket, refetch only" path is possible).
- Reconnect uses a fixed delay, not yet exponential backoff with jitter.
- No latency measurement yet.

## License

MIT — see [LICENSE](LICENSE).
