# redis-benchmark-rs

A memtier-style Redis benchmark written in Rust. GET-focused, with the memtier flags
people actually reach for: OSS-cluster **or** single-endpoint, TLS, per-connection
rate-limiting, `--ratio`, `--key-pattern`, and `--key-prefix`/`--key-minimum`/`--key-maximum`.

Built on `tokio` + the `redis` crate (async, cluster-async, rustls). TLS uses **rustls**
(pure Rust) so the release ships a single static binary that runs on every supported distro.

## Usage

```
redis-benchmark-rs -h <host> -p <port> [flags]
```

| flag | meaning |
|---|---|
| `-h, --host` / `-p, --port` | server endpoint (or a cluster seed) |
| `--cluster-mode` | use the OSS cluster API (slot routing); omit for a single endpoint |
| `--tls` / `--tls-skip-verify` | enable TLS / skip cert verification |
| `--cacert`/`--cert`/`--key` | CA / client cert / key (verified TLS — planned) |
| `--user` / `-a, --password` | AUTH username / password |
| `-t, --threads` / `-c, --connections` | worker threads × connections-per-thread |
| `--ratio` | `set:get` (default `0:1` = pure GET; `1:10` = 1 SET / 10 GET) |
| `--key-pattern` | `R` random · `S` sequential · `G` gaussian |
| `--key-prefix` / `--key-minimum` / `--key-maximum` | key namespace and id range |
| `--data-size` | SET payload size in bytes (write side only) |
| `--pipeline` | in-flight commands per connection (forced to 1 in `--cluster-mode`) |
| `--rate-limiting` | per-connection **commands/sec** (0 = unlimited); aggregate ≈ `rate × threads × connections` |
| `--test-time` | duration (seconds) |
| `--print-percentiles` | e.g. `50,99,99.9,99.99,99.999` |
| `--json-out-file` / `--hide-histogram` / `--realtime-latencies` | output control |

### Examples

Pure GET, single endpoint, TLS, rate-limited:
```
redis-benchmark-rs -h 10.0.0.1 -p 12000 --tls --tls-skip-verify \
  -t 16 -c 26 --ratio 0:1 --key-pattern R \
  --key-prefix 'k:' --key-minimum 1 --key-maximum 1700000 \
  --rate-limiting 700 --test-time 600 \
  --print-percentiles 50,99,99.9,99.99,99.999 --json-out-file out.json
```

Cluster mode:
```
redis-benchmark-rs -h 10.0.0.1 -p 12000 --cluster-mode --tls --tls-skip-verify -t 16 -c 26
```

## Install

Download the static binary from the latest [release](../../releases) (works on RHEL 8/9,
Ubuntu 22.04/24.04, …):

```
curl -sfLO https://github.com/redis-performance/redis-benchmark-rs/releases/latest/download/redis-benchmark-rs-x86_64-musl
chmod +x redis-benchmark-rs-x86_64-musl && ./redis-benchmark-rs-x86_64-musl --help
```

Or install the native package: `*.el8.rpm`, `*.el9.rpm`, `*.jammy.deb`, `*.noble.deb`.

## Build

```
cargo build --release
# static (portable) build:
cargo build --release --target x86_64-unknown-linux-musl
```

## Status / roadmap

- [x] single-endpoint + cluster-mode, GET + `--ratio` SET
- [x] `--key-pattern` (R/S/G), key-prefix/min/max, rate-limiting, pipeline
- [x] insecure TLS; HDR percentiles + JSON out; multi-distro release
- [ ] verified TLS / mTLS (`--cacert`/`--cert`/`--key`)
- [ ] hash-tag key templates, multi-key & transaction modes
