# Known risk areas — redis-performance/redis-benchmark-rs

This repo has no reviewer-caught bugs on record (see `real-history.md`: zero review comments
ever). What it *does* have is a short but real trail of **self-authored bugfixes**, stated in
the author's own PR descriptions and confirmed by reading the current source
(`src/*.rs`) — these are this codebase's own recurring failure modes, evidenced by what has
actually gone wrong here before, not generic Rust-networking advice. Treat these as the real
categories worth tracing by hand on a new PR; everything else, reason about from first
principles and say so.

1. **TLS `CryptoProvider` must actually be installed before the first handshake.**
   PR#2's real fix: rustls 0.23 needs a process-level `CryptoProvider` installed before any
   TLS connection, and the `redis` crate's rustls feature does not install one automatically —
   without this, the *first* TLS handshake panics
   (`Could not automatically determine the process-level CryptoProvider`). This was caught by
   running against a real TLS Redis Enterprise DB, not by review — "nothing TLS worked
   before this" per the PR body. Confirmed in current source: `src/conn.rs` installs it via
   `rustls::crypto::ring::default_provider().install_default()` with a comment explaining why.
   On any PR touching `src/conn.rs`'s TLS setup, connection-establishment ordering, or a
   dependency bump of `rustls`/`redis`, check this install still happens exactly once,
   early enough, and that a second call (guarded or not) can't itself panic.

2. **Cluster mode forces pipeline depth to 1 — verify any PR touching pipelining or cluster
   mode preserves this.** `src/lib.rs` has a real, explicit comment: "Multi-key pipelines
   cross slots in cluster mode → CROSSSLOT. Force depth 1." This is a real, evidenced
   constraint already encoded to avoid a real Redis Cluster error class (CROSSSLOT), not a
   hypothetical. Any PR that changes how `--pipeline` interacts with `--cluster-mode`, adds a
   new command type to the pipeline, or refactors `lib.rs`'s dispatch should be checked for
   whether this guard still holds — silently allowing pipeline depth > 1 in cluster mode
   would reintroduce a real error class this code was written specifically to avoid.

3. **Key-range partitioning across concurrent connections has had a real, shipped bug.**
   PR#2's own description: "sequential `--key-pattern` now partitions `[min,max]` across
   connections (memtier-style) so a concurrent preload covers the whole range exactly once.
   Before, every connection sequenced from `min` and only the low keys got written." This is a
   real, self-caught correctness bug (a preload that silently only wrote part of its intended
   keyspace) — worth tracing by hand on any PR touching `src/keygen.rs`'s `KeyGen::new`/`next`
   or how per-connection key generators are constructed in the workload dispatch, since a
   regression here doesn't crash, it silently produces incomplete coverage.

4. **Rate limiting is drift-corrected against an absolute schedule, not a naive
   sleep-per-batch loop.** `src/workload.rs` computes `min_interval` from
   `pipeline / rate_limiting` and enforces it via "absolute send schedule, drift-corrected
   rate limiting" (real comment in the source). PR#1's description separately calls out
   "rate-limiter accuracy" as one of the things its pre-PR adversarial review targeted, and
   PR#2 reports a real measured validation ("rate-limited GET held 13,100 ops/sec dead-on").
   On any PR touching `src/workload.rs`'s rate-limiting or pipelining logic, check that a
   slow batch (e.g. from a network stall) doesn't cause the *next* batches to compensate by
   bursting, and that the interval math still accounts for `--pipeline` batching commands
   together rather than rate-limiting each command independently.

5. **Fail-fast on total connection failure, and strict input validation, are both stated
   pre-PR review targets with no later evidence they were re-tested.** PR#1's description
   lists "fail-fast on total connection failure" and "strict input validation" as targets of
   its pre-opening 7-way adversarial review, but — per `real-history.md` — nothing in this
   repo's subsequent 3 PRs or zero review comments re-exercised or contradicted that claim.
   Treat it as a real, stated design goal worth checking against on any PR touching
   `src/conn.rs`'s connection-establishment error handling or `src/cli.rs`'s argument
   validation, but don't cite it as independently reviewer-verified — it's the author's own
   account of testing done before the PR existed.

6. **Multi-distro packaging has had a real, shipped fix for a distro-specific package
   conflict.** PR#4: the `el9` RPM job failed bootstrapping because Rocky 9 ships
   `curl-minimal`, which conflicts with plain `curl`; fixed with `yum --allowerasing`
   (a no-op on el8). This is real, evidenced precedent that this project's `el8`/`el9`/
   `jammy`/`noble` packaging matrix (`Makefile`, `.github/workflows/release.yml`) has genuine
   per-distro package-manager quirks, not just per-arch ones. Any PR touching packaging steps
   in the `Makefile` or `release.yml` should be checked against whether it was actually
   validated per-distro, not just on whichever one is fastest to test locally.

## What this list is honestly silent on

- **Async/tokio-specific concurrency bugs** (task cancellation, channel backpressure,
  panics-in-spawned-tasks) — no evidenced real bug in this category yet; reason from
  first principles if a PR touches `tokio::spawn` usage in `src/workload.rs` or `main.rs`.
- **HDR histogram / percentile correctness** (`src/stats.rs`, `src/realtime.rs`) — no
  evidenced real bug; the PR bodies state test counts but no specific percentile-correctness
  bug has been recorded.
- **Memory-safety / `unsafe` review** — grep the diff for `unsafe`; none is known to exist in
  the current codebase, so a PR introducing one would be a first, not a repeat of a known
  pattern, and deserves scrutiny on that basis alone.
