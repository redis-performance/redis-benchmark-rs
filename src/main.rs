#[cfg(all(feature = "mimalloc", not(target_env = "msvc")))]
use mimalloc::MiMalloc;
#[cfg(all(feature = "mimalloc", not(target_env = "msvc")))]
#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

use clap::Parser;
use redis_benchmark_rs::{cli, run};

fn main() -> anyhow::Result<()> {
    let args = cli::Args::parse();
    args.validate()?; // validate all inputs before spinning up the runtime

    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(args.threads.max(1))
        .enable_all()
        .build()?;
    rt.block_on(async { run(args).await.map(|_| ()) })
}
