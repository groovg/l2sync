# l2sync

Multi-exchange crypto **L2 order book aggregator** in async Rust. Connects to several
exchange depth feeds, maintains a correct local order book per venue with sequence-gap
detection and snapshot resync, and produces a normalized, consolidated cross-venue view.

The interesting parts are the ones feed handlers actually struggle with in production:
out-of-order updates, sequence gaps, snapshot+diff synchronization, reconnects, and
heterogeneous message formats — all under async backpressure.

> **Status: early / work in progress.** Scaffolding only so far; see the roadmap.

## Roadmap

- [ ] Single Binance venue: depth-diff stream, parse, raw top of book.
- [ ] Correct book sync: snapshot+diff handshake, sequence-gap detection, resync.
- [ ] Multiple venues behind a common trait (Bybit, OKX).
- [ ] Normalized update/trade/snapshot model with exact fixed-point prices.
- [ ] Consolidated cross-venue BBO and crossed-market detection.
- [ ] Tick-to-book latency, auto-reconnect with backoff, bounded-channel backpressure.

## License

MIT — see [LICENSE](LICENSE).
