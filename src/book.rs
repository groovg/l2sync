use std::collections::BTreeMap;

pub type Ticks = i64;

pub const SCALE: i64 = 100_000_000;

pub fn parse_scaled(s: &str) -> Option<Ticks> {
    let (neg, body) = match s.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, s),
    };
    let (int_part, frac_part) = body.split_once('.').unwrap_or((body, ""));
    if int_part.is_empty() || frac_part.len() > 8 {
        return None;
    }
    let mut value = int_part.parse::<i64>().ok()?.checked_mul(SCALE)?;
    let mut place = SCALE / 10;
    for c in frac_part.chars() {
        value = value.checked_add(i64::from(c.to_digit(10)?) * place)?;
        place /= 10;
    }
    Some(if neg { -value } else { value })
}

pub fn format_scaled(t: Ticks) -> String {
    let magnitude = t.unsigned_abs();
    let sign = if t < 0 { "-" } else { "" };
    format!(
        "{sign}{}.{:08}",
        magnitude / SCALE as u64,
        magnitude % SCALE as u64
    )
}

pub type Level = (Ticks, Ticks);

#[derive(Default)]
pub struct OrderBook {
    bids: BTreeMap<Ticks, Ticks>,
    asks: BTreeMap<Ticks, Ticks>,
}

impl OrderBook {
    pub fn clear(&mut self) {
        self.bids.clear();
        self.asks.clear();
    }

    pub fn reset(&mut self, bids: &[Level], asks: &[Level]) {
        self.clear();
        self.apply(bids, asks);
    }

    pub fn apply(&mut self, bids: &[Level], asks: &[Level]) {
        Self::apply_side(&mut self.bids, bids);
        Self::apply_side(&mut self.asks, asks);
    }

    fn apply_side(side: &mut BTreeMap<Ticks, Ticks>, levels: &[Level]) {
        for &(price, qty) in levels {
            if qty == 0 {
                side.remove(&price);
            } else {
                side.insert(price, qty);
            }
        }
    }

    pub fn best_bid(&self) -> Option<Level> {
        self.bids.iter().next_back().map(|(&p, &q)| (p, q))
    }

    pub fn best_ask(&self) -> Option<Level> {
        self.asks.iter().next().map(|(&p, &q)| (p, q))
    }

    pub fn crossed(&self) -> bool {
        matches!((self.best_bid(), self.best_ask()), (Some((b, _)), Some((a, _))) if b >= a)
    }

    pub fn bid_qty(&self, price: Ticks) -> Option<Ticks> {
        self.bids.get(&price).copied()
    }

    pub fn ask_qty(&self, price: Ticks) -> Option<Ticks> {
        self.asks.get(&price).copied()
    }

    pub fn depth(&self) -> (usize, usize) {
        (self.bids.len(), self.asks.len())
    }
}
