use crate::cli::{Args, KeyPattern};
use crate::conn::AnyConn;
use crate::keygen::KeyGen;
use crate::realtime::Realtime;
use crate::stats::WorkerStats;
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::time::sleep;

/// Drive one connection until `deadline`, executing GET/SET per `--ratio`,
/// honoring `--pipeline` and per-connection `--rate-limiting`. Feeds both the
/// per-worker final stats and the shared realtime interval stats.
pub async fn run_worker(
    mut conn: AnyConn,
    args: Args,
    seed: u64,
    idx: usize,
    total: usize,
    deadline: Instant,
    rt: Arc<Realtime>,
) -> WorkerStats {
    let (set_w, get_w) = args.parse_ratio().unwrap_or((0, 1));
    let total_w = set_w as u64 + get_w as u64;
    let mut rng = SmallRng::seed_from_u64(seed);
    // Sequential mode partitions the keyspace across workers (memtier-style) so
    // together they cover [min,max] exactly once — essential for preloading.
    let (kmin, kmax) = if args.key_pattern == KeyPattern::Sequential && total > 1 {
        let lo = args.key_minimum.min(args.key_maximum);
        let hi = args.key_minimum.max(args.key_maximum);
        let span = hi - lo + 1;
        let per = span.div_ceil(total as u64);
        let wlo = lo + (idx as u64) * per;
        let whi = (wlo + per - 1).min(hi);
        (wlo.min(hi), whi)
    } else {
        (args.key_minimum, args.key_maximum)
    };
    let mut kg = KeyGen::new(&args.key_prefix, kmin, kmax, args.key_pattern);
    let payload = vec![b'x'; args.data_size];
    let expiry = args.expiry().unwrap_or(None);
    let pipeline = args.pipeline.max(1);
    let mut stats = WorkerStats::new();

    // rate is COMMANDS/sec; a batch carries `pipeline` commands.
    let min_interval = if args.rate_limiting > 0 {
        Some(Duration::from_secs_f64(
            pipeline as f64 / args.rate_limiting as f64,
        ))
    } else {
        None
    };

    let pick_set = |rng: &mut SmallRng| set_w > 0 && rng.gen_range(0..total_w) < set_w as u64;

    // absolute send schedule → drift-corrected rate limiting (see rate-limit block below)
    let mut next_send = Instant::now();

    while Instant::now() < deadline {
        if pipeline == 1 {
            let is_set = pick_set(&mut rng);
            let key = kg.next(&mut rng);
            let mut cmd = redis::cmd(if is_set { "SET" } else { "GET" });
            cmd.arg(&key);
            if is_set {
                cmd.arg(&payload);
                if let Some((lo, hi)) = expiry {
                    cmd.arg("EX").arg(rng.gen_range(lo..=hi));
                }
            }
            let start = Instant::now();
            match conn.run(&cmd).await {
                Ok(v) => {
                    let miss = matches!(v, redis::Value::Nil);
                    let us = start.elapsed();
                    stats.record(is_set, us, miss);
                    rt.record(idx, us.as_micros() as u64, !is_set, miss);
                }
                Err(_) => stats.record_err(is_set),
            }
        } else {
            let mut pipe = redis::pipe();
            let mut kinds = Vec::with_capacity(pipeline);
            for _ in 0..pipeline {
                let is_set = pick_set(&mut rng);
                let key = kg.next(&mut rng);
                if is_set {
                    pipe.cmd("SET").arg(&key).arg(&payload);
                    if let Some((lo, hi)) = expiry {
                        pipe.arg("EX").arg(rng.gen_range(lo..=hi));
                    }
                } else {
                    pipe.cmd("GET").arg(&key);
                }
                kinds.push(is_set);
            }
            let start = Instant::now();
            let res = conn.run_pipe(&pipe).await;
            let elapsed = start.elapsed();
            let us = elapsed.as_micros() as u64;
            match res {
                Ok(v) => {
                    // pipeline reply is an array, one element per command
                    let items: Vec<redis::Value> = match v {
                        redis::Value::Array(a) => a,
                        other => vec![other],
                    };
                    for (i, k) in kinds.iter().enumerate() {
                        let miss = !*k && matches!(items.get(i), Some(redis::Value::Nil));
                        stats.record(*k, elapsed, miss);
                        rt.record(idx, us, !*k, miss);
                    }
                }
                Err(_) => {
                    for k in &kinds {
                        stats.record_err(*k);
                    }
                }
            }
        }

        // rate limit: absolute schedule, drift-corrected, deadline-bounded.
        // Banks sub-300us debt (timer granularity) instead of over-sleeping it.
        if let Some(mi) = min_interval {
            next_send += mi;
            let now = Instant::now();
            if next_send > now {
                let wait = (next_send - now).min(deadline.saturating_duration_since(now));
                if wait >= Duration::from_micros(300) {
                    sleep(wait).await;
                }
            } else if now.saturating_duration_since(next_send) > mi * 4 {
                next_send = now; // cap runaway debt after a long stall
            }
        }
    }
    stats
}
