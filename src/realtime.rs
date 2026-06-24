use hdrhistogram::Histogram;
use std::sync::atomic::{AtomicU64, Ordering::Relaxed};
use std::sync::Mutex;

/// Shared, low-overhead interval stats for the `--realtime-latencies` ticker.
/// Counters are lock-free atomics; latency goes into a per-connection histogram
/// shard (one writer each → uncontended lock), which the ticker drains+clears
/// every interval to compute per-second percentiles.
pub struct Realtime {
    pub ops: AtomicU64,
    pub hits: AtomicU64,
    pub misses: AtomicU64,
    pub enabled: bool,
    shards: Vec<Mutex<Histogram<u64>>>,
}

impl Realtime {
    pub fn new(conns: usize, enabled: bool) -> Self {
        let n = if enabled { conns } else { 0 };
        let shards = (0..n)
            .map(|_| Mutex::new(Histogram::new(3).expect("hdr")))
            .collect();
        Self {
            ops: AtomicU64::new(0),
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
            enabled,
            shards,
        }
    }

    #[inline]
    pub fn record(&self, idx: usize, micros: u64, is_get: bool, miss: bool) {
        self.ops.fetch_add(1, Relaxed);
        if is_get {
            if miss {
                self.misses.fetch_add(1, Relaxed);
            } else {
                self.hits.fetch_add(1, Relaxed);
            }
        }
        if self.enabled {
            if let Ok(mut h) = self.shards[idx].lock() {
                let _ = h.record(micros);
            }
        }
    }

    /// Merge all shard histograms into one and clear them (per-interval snapshot).
    pub fn drain_hist(&self) -> Histogram<u64> {
        let mut merged = Histogram::new(3).expect("hdr");
        for s in &self.shards {
            if let Ok(mut h) = s.lock() {
                let _ = merged.add(&*h);
                h.clear();
            }
        }
        merged
    }

    pub fn snapshot(&self) -> (u64, u64, u64) {
        (
            self.ops.load(Relaxed),
            self.hits.load(Relaxed),
            self.misses.load(Relaxed),
        )
    }
}
