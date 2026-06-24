pub mod cli;
pub mod conn;
pub mod keygen;
pub mod realtime;
pub mod stats;
pub mod workload;

use realtime::Realtime;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Run the benchmark to completion and emit the report. Returns the aggregate
/// stats so callers (tests) can assert on them.
pub async fn run(mut args: cli::Args) -> anyhow::Result<stats::WorkerStats> {
    // Multi-key pipelines cross slots in cluster mode → CROSSSLOT. Force depth 1.
    if args.cluster_mode && args.pipeline > 1 {
        eprintln!(
            "warning: --pipeline {} forced to 1 in cluster-mode (pipelines would cross slots)",
            args.pipeline
        );
        args.pipeline = 1;
    }
    let total = args.total_connections();
    eprintln!(
        "==> {} connections ({} threads x {} per thread), {}, {}s @ {}:{}",
        total,
        args.threads,
        args.connections,
        if args.cluster_mode {
            "cluster-mode"
        } else {
            "single-endpoint"
        },
        args.test_time,
        args.host,
        args.port,
    );

    let run_start = Instant::now();
    let deadline = run_start + Duration::from_secs(args.test_time);
    let realtime = Arc::new(Realtime::new(total, args.realtime_latencies));
    let conn_ok = Arc::new(AtomicUsize::new(0));

    let ticker = if args.realtime_latencies {
        let rt = realtime.clone();
        let pcts = args.percentiles();
        let interval = args.realtime_interval.max(0.05);
        let test_time = args.test_time;
        Some(tokio::spawn(async move {
            let start = Instant::now();
            let (mut p_ops, mut p_hits, mut p_miss) = (0u64, 0u64, 0u64);
            loop {
                tokio::time::sleep(Duration::from_secs_f64(interval)).await;
                let (ops, hits, miss) = rt.snapshot();
                let (d_ops, d_hits, d_miss) = (ops - p_ops, hits - p_hits, miss - p_miss);
                p_ops = ops;
                p_hits = hits;
                p_miss = miss;
                let h = rt.drain_hist();
                let secs = start.elapsed().as_secs_f64();
                let tput = d_ops as f64 / interval;
                let gets = d_hits + d_miss;
                let miss_pct = if gets > 0 {
                    100.0 * d_miss as f64 / gets as f64
                } else {
                    0.0
                };
                let mut line =
                    format!("[{secs:>5.1}s] {tput:>9.0} ops/sec | miss {miss_pct:>5.1}% |");
                for p in &pcts {
                    let v = h.value_at_quantile(p / 100.0) as f64 / 1000.0;
                    line.push_str(&format!(" {}={:.3}", stats::fmt_pct(*p), v));
                }
                line.push_str(" ms");
                eprintln!("{line}");
                if secs >= test_time as f64 {
                    break;
                }
            }
        }))
    } else {
        None
    };

    let mut handles = Vec::with_capacity(total);
    let args = Arc::new(args);
    for i in 0..total {
        let args = args.clone();
        let realtime = realtime.clone();
        let conn_ok = conn_ok.clone();
        handles.push(tokio::spawn(async move {
            let seed = 0x9E3779B97F4A7C15u64.wrapping_mul(i as u64 + 1);
            match conn::connect(&args).await {
                Ok(c) => {
                    conn_ok.fetch_add(1, Ordering::Relaxed);
                    workload::run_worker(c, (*args).clone(), seed, i, total, deadline, realtime)
                        .await
                }
                Err(e) => {
                    eprintln!("connection {i} failed: {e:#}");
                    stats::WorkerStats::new()
                }
            }
        }));
    }

    let mut agg = stats::WorkerStats::new();
    for h in handles {
        if let Ok(s) = h.await {
            agg.merge(&s);
        }
    }
    if let Some(t) = ticker {
        let _ = t.await;
    }

    if conn_ok.load(Ordering::Relaxed) == 0 {
        anyhow::bail!("all {total} connection(s) failed — check host/port/TLS/auth");
    }
    let actual_secs = run_start.elapsed().as_secs_f64();
    stats::report(&agg, args.as_ref(), actual_secs)?;
    Ok(agg)
}
