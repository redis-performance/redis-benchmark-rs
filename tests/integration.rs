//! Integration tests — exercise the full benchmark against a live Redis,
//! analogous to memtier's pytest suite. Skips cleanly if no Redis is reachable
//! (set REDIS_HOST / REDIS_PORT; CI provides a redis service container).

use clap::Parser;
use redis_benchmark_rs::cli::{Args, KeyPattern};

fn redis_addr() -> (String, u16) {
    let host = std::env::var("REDIS_HOST").unwrap_or_else(|_| "127.0.0.1".into());
    let port = std::env::var("REDIS_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(6379);
    (host, port)
}

fn reachable(host: &str, port: u16) -> bool {
    use std::net::ToSocketAddrs;
    match (host, port).to_socket_addrs() {
        Ok(mut addrs) => addrs.next().is_some_and(|a| {
            std::net::TcpStream::connect_timeout(&a, std::time::Duration::from_millis(400)).is_ok()
        }),
        Err(_) => false,
    }
}

fn base_args(host: &str, port: u16) -> Args {
    let mut a = Args::parse_from(["redis-benchmark-rs"]);
    a.host = host.to_string();
    a.port = port;
    a.threads = 2;
    a.connections = 4;
    a.test_time = 2;
    a.hide_histogram = true;
    a.key_prefix = "rbench-it:".into();
    a.key_minimum = 1;
    a.key_maximum = 2000;
    a.key_pattern = KeyPattern::Random;
    a
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mixed_get_set_runs_without_errors() {
    let (host, port) = redis_addr();
    if !reachable(&host, port) {
        eprintln!("SKIP: no redis at {host}:{port} (set REDIS_HOST/REDIS_PORT)");
        return;
    }
    let mut a = base_args(&host, port);
    a.ratio = "1:5".into(); // writes + reads

    let agg = redis_benchmark_rs::run(a).await.expect("run ok");

    assert!(agg.gets + agg.sets > 0, "should execute some ops");
    assert!(agg.sets > 0, "ratio 1:5 must produce SETs");
    assert!(agg.gets > 0, "ratio 1:5 must produce GETs");
    assert_eq!(
        agg.get_err + agg.set_err,
        0,
        "no errors against a live redis"
    );
    // hits + misses must equal the GET count (every GET classified)
    assert_eq!(agg.get_hits + agg.get_miss, agg.gets);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn pure_get_does_no_writes() {
    let (host, port) = redis_addr();
    if !reachable(&host, port) {
        eprintln!("SKIP: no redis at {host}:{port}");
        return;
    }
    let mut a = base_args(&host, port);
    a.ratio = "0:1".into(); // pure GET

    let agg = redis_benchmark_rs::run(a).await.expect("run ok");

    assert_eq!(agg.sets, 0, "pure-GET ratio must not SET");
    assert!(agg.gets > 0);
    assert_eq!(agg.get_err, 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn rate_limiting_caps_throughput() {
    let (host, port) = redis_addr();
    if !reachable(&host, port) {
        eprintln!("SKIP: no redis at {host}:{port}");
        return;
    }
    let mut a = base_args(&host, port);
    a.ratio = "0:1".into();
    a.connections = 2;
    a.threads = 1;
    a.rate_limiting = 200; // 200 cmd/s/conn * 2 conns = ~400/s ceiling
    a.test_time = 2;

    let agg = redis_benchmark_rs::run(a).await.expect("run ok");

    let total = agg.gets + agg.sets;
    // ceiling ~= 400/s * 2s = 800; allow generous slack for timer granularity
    assert!(total <= 1100, "rate-limited run exceeded cap: {total} ops");
    assert!(total > 0);
}
