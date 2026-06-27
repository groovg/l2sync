# l2sync

[![CI](https://github.com/groovg/l2sync/actions/workflows/ci.yml/badge.svg)](https://github.com/groovg/l2sync/actions/workflows/ci.yml)

Multi-exchange crypto **L2 order book aggregator** in async Rust. Connects to several
exchange depth feeds, maintains a correct local order book per venue with sequence-gap
detection and snapshot resync, and produces a normalized, consolidated cross-venue view.

This is a feed-handler study: the parts that are actually hard in production — out-of-order
updates, sequence gaps, snapshot+diff synchronization, reconnects, and heterogeneous message
formats, all under async backpressure.

> **Status: early / work in progress.** The single-venue raw stream works; the book
> synchronization that makes the data *correct* is next. Nothing here is production-ready yet.

## Why a depth feed is not "just apply every update"

Exchanges publish the book as a REST **snapshot** plus a stream of **diff** events; each diff
carries only the levels that *changed* since the last one. The naive approach — connect to the
diff stream and treat each event as the book — is silently wrong, because a single diff is not
the book, only a delta against a snapshot you do not have yet.

You can see it directly from the raw stream. Printing the best bid/ask *within each diff
event* gives nonsense, because the highest bid present in one delta is not the real top of book:

```
bid 59859.99 x 0.94701  |  ask 59860.00 x 1.65166     <- plausible
bid 53874.00 x 0.58268  |  ask 59863.95 x 0.00009     <- a deep level happened to change
bid 59859.99 x 0.96713  |  ask 79666.01 x 0.00133     <- an ask far from the touch changed
```

A correct book requires the documented handshake:

1. Open the diff stream and **buffer** events.
2. Fetch the REST depth snapshot (it carries a `lastUpdateId`).
3. Drop buffered diffs whose final id is `<= lastUpdateId`.
4. Validate that the first kept diff brackets the snapshot id, then apply diffs in order.
5. On any **sequence gap** (a diff whose first id is not contiguous with the last applied),
   discard the book and resync from step 2.

Sequence semantics differ per venue (Binance `U`/`u`, Bybit `seq`, OKX checksum), so the
handshake is abstracted behind a `Venue` trait rather than special-cased.

## Roadmap

- [x] Single Binance venue: depth-diff stream, `serde` parse, raw top of book.
- [ ] Correct book sync: snapshot+diff handshake, sequence-gap detection, resync.
- [ ] Multiple venues behind a common `Venue` trait (Bybit, OKX).
- [ ] Normalized update/trade/snapshot model; exact fixed-point prices
      ([`fixed-decimal`](https://github.com/groovg/fixed-decimal)), never floats.
- [ ] Consolidated cross-venue BBO and crossed-market (arb) detection.
- [ ] Tick-to-book latency percentiles, auto-reconnect with backoff, bounded-channel backpressure.

## Build & run

```sh
cargo run --release
```

Connects to `btcusdt@depth@100ms` and streams the raw best bid/ask. Requires network access
to Binance.

## Design notes

- **Async model:** one `tokio` task per venue connection, feeding a bounded `mpsc` to a single
  book engine — backpressure instead of unbounded memory growth under a market burst.
- **Book representation:** `BTreeMap<Price, Qty>` per side to start (simple and obviously
  correct). A flat tick-indexed array is faster at the touch and is noted as a later swap.
- **Prices:** kept as exchange-native strings for now; exact fixed-point once the normalized
  model lands. Floats are deliberately avoided in the book — rounding produces mismatched levels
  and phantom crosses.
- **TLS:** rustls with the `ring` provider, so the build has no system OpenSSL dependency.

## Limitations (current)

- Single venue (Binance), single hardcoded symbol.
- No book synchronization yet — the current binary prints the *raw diff* top of book, which is
  intentionally not the synchronized BBO (see above).
- No reconnect / ping-pong handling yet.
- No tests yet; deterministic replay-from-recording tests land with the sync work.

## License

MIT — see [LICENSE](LICENSE).
