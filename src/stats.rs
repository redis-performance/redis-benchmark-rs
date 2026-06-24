use crate::cli::Args;
use hdrhistogram::Histogram;
use serde::Serialize;
use std::time::Duration;

/// Per-worker (and, after merge, aggregate) counters + latency histograms.
pub struct WorkerStats {
    pub gets: u64,
    pub get_hits: u64,
    pub get_miss: u64,
    pub get_err: u64,
    pub sets: u64,
    pub set_err: u64,
    pub get_h: Histogram<u64>, // microseconds
    pub set_h: Histogram<u64>,
}

impl WorkerStats {
    pub fn new() -> Self {
        Self {
            gets: 0,
            get_hits: 0,
            get_miss: 0,
            get_err: 0,
            sets: 0,
            set_err: 0,
            get_h: Histogram::new(3).expect("hdr get"),
            set_h: Histogram::new(3).expect("hdr set"),
        }
    }

    pub fn record(&mut self, is_set: bool, d: Duration, miss: bool) {
        let us = d.as_micros() as u64;
        if is_set {
            self.sets += 1;
            let _ = self.set_h.record(us);
        } else {
            self.gets += 1;
            if miss {
                self.get_miss += 1;
            } else {
                self.get_hits += 1;
            }
            let _ = self.get_h.record(us);
        }
    }

    pub fn record_err(&mut self, is_set: bool) {
        if is_set {
            self.set_err += 1;
        } else {
            self.get_err += 1;
        }
    }

    pub fn merge(&mut self, o: &WorkerStats) {
        self.gets += o.gets;
        self.get_hits += o.get_hits;
        self.get_miss += o.get_miss;
        self.get_err += o.get_err;
        self.sets += o.sets;
        self.set_err += o.set_err;
        let _ = self.get_h.add(&o.get_h);
        let _ = self.set_h.add(&o.set_h);
    }
}

impl Default for WorkerStats {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Serialize)]
struct CmdReport {
    ops: u64,
    ops_per_sec: f64,
    hits_per_sec: f64,
    misses_per_sec: f64,
    hit_ratio_pct: f64,
    miss_ratio_pct: f64,
    avg_latency_ms: f64,
    percentiles_ms: std::collections::BTreeMap<String, f64>,
}

#[derive(Serialize)]
struct Report {
    runtime_secs: f64,
    total_ops: u64,
    total_ops_per_sec: f64,
    total_errors: u64,
    gets: CmdReport,
    sets: CmdReport,
}

fn cmd_report(
    h: &Histogram<u64>,
    ops: u64,
    hits: u64,
    miss: u64,
    secs: f64,
    pcts: &[f64],
) -> CmdReport {
    let percentiles_ms = pcts
        .iter()
        .map(|p| (fmt_pct(*p), h.value_at_quantile(p / 100.0) as f64 / 1000.0))
        .collect();
    let lookups = hits + miss;
    let (hit_ratio_pct, miss_ratio_pct) = if lookups > 0 {
        (
            100.0 * hits as f64 / lookups as f64,
            100.0 * miss as f64 / lookups as f64,
        )
    } else {
        (0.0, 0.0)
    };
    CmdReport {
        ops,
        ops_per_sec: ops as f64 / secs,
        hits_per_sec: hits as f64 / secs,
        misses_per_sec: miss as f64 / secs,
        hit_ratio_pct,
        miss_ratio_pct,
        avg_latency_ms: h.mean() / 1000.0,
        percentiles_ms,
    }
}

pub fn fmt_pct(p: f64) -> String {
    if (p.fract()).abs() < f64::EPSILON {
        format!("p{}", p as u64)
    } else {
        format!("p{}", p)
    }
}

pub fn report(agg: &WorkerStats, args: &Args, actual_secs: f64) -> anyhow::Result<()> {
    let secs = if actual_secs > 0.0 {
        actual_secs
    } else {
        args.test_time.max(1) as f64
    };
    let pcts = args.percentiles();
    let gets = cmd_report(
        &agg.get_h,
        agg.gets,
        agg.get_hits,
        agg.get_miss,
        secs,
        &pcts,
    );
    let sets = cmd_report(&agg.set_h, agg.sets, 0, 0, secs, &pcts);
    let total_ops = agg.gets + agg.sets;

    let errs = agg.get_err + agg.set_err;
    if !args.hide_histogram {
        println!("\n=== RESULTS ({secs:.1} s) ===");
        print_row("GET", &gets, agg.get_err, true);
        if agg.sets > 0 {
            print_row("SET", &sets, agg.set_err, false);
        }
        println!(
            "TOTAL   {:>10.0} ops/sec   ({} ops, {} get-misses, {} errors)",
            total_ops as f64 / secs,
            total_ops,
            agg.get_miss,
            errs
        );
        if errs > 0 {
            let attempts = (total_ops + errs).max(1);
            println!(
                "WARNING: {errs} command errors ({:.2}% of attempts) — results may be invalid",
                100.0 * errs as f64 / attempts as f64
            );
        }
    }

    if let Some(path) = &args.json_out_file {
        let rep = Report {
            runtime_secs: secs,
            total_ops,
            total_ops_per_sec: total_ops as f64 / secs,
            total_errors: errs,
            gets,
            sets,
        };
        std::fs::write(path, serde_json::to_string_pretty(&rep)?)?;
        eprintln!("wrote {path}");
    }
    Ok(())
}

fn print_row(name: &str, r: &CmdReport, errs: u64, show_hitmiss: bool) {
    print!(
        "{name:5} {:>10.0} ops/sec  avg {:>7.3} ms  ",
        r.ops_per_sec, r.avg_latency_ms
    );
    if show_hitmiss {
        print!(
            "hit {:>5.1}% miss {:>5.1}%  ",
            r.hit_ratio_pct, r.miss_ratio_pct
        );
    }
    print!("errs {errs:>5} | ");
    for (p, v) in &r.percentiles_ms {
        print!("{p} {:.3}  ", v);
    }
    println!();
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn counts_hits_misses_and_merges() {
        let mut a = WorkerStats::new();
        a.record(false, Duration::from_micros(100), false); // get hit
        a.record(false, Duration::from_micros(100), true); // get miss
        a.record(true, Duration::from_micros(100), false); // set
        assert_eq!((a.gets, a.get_hits, a.get_miss, a.sets), (2, 1, 1, 1));
        let mut b = WorkerStats::new();
        b.record(false, Duration::from_micros(50), true);
        a.merge(&b);
        assert_eq!((a.gets, a.get_miss), (3, 2));
    }

    #[test]
    fn percentile_labels() {
        assert_eq!(fmt_pct(50.0), "p50");
        assert_eq!(fmt_pct(99.9), "p99.9");
        assert_eq!(fmt_pct(99.999), "p99.999");
    }
}
