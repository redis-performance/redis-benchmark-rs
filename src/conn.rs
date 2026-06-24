use crate::cli::Args;
use anyhow::{Context, Result};
use redis::aio::MultiplexedConnection;
use redis::cluster_async::ClusterConnection;

/// One live connection — either a single-endpoint multiplexed connection or an
/// async cluster connection. Both implement `ConnectionLike`, so command
/// execution is uniform.
pub enum AnyConn {
    Single(MultiplexedConnection),
    Cluster(ClusterConnection),
}

impl AnyConn {
    pub async fn run(&mut self, cmd: &redis::Cmd) -> redis::RedisResult<redis::Value> {
        match self {
            AnyConn::Single(c) => cmd.query_async(c).await,
            AnyConn::Cluster(c) => cmd.query_async(c).await,
        }
    }

    pub async fn run_pipe(&mut self, pipe: &redis::Pipeline) -> redis::RedisResult<redis::Value> {
        match self {
            AnyConn::Single(c) => pipe.query_async(c).await,
            AnyConn::Cluster(c) => pipe.query_async(c).await,
        }
    }
}

/// Percent-encode the URL-reserved ASCII chars so passwords/usernames survive
/// being placed in the userinfo of a connection URL.
fn enc(s: &str) -> String {
    let mut o = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '@' | ':' | '/' | '#' | '%' | '?' | '&' => {
                for b in c.to_string().bytes() {
                    o.push_str(&format!("%{b:02X}"));
                }
            }
            _ => o.push(c),
        }
    }
    o
}

/// Build a connection URL. `rediss://` for TLS; the `#insecure` fragment skips
/// certificate verification (redis-rs convention); userinfo carries AUTH.
fn url(args: &Args) -> String {
    let scheme = if args.tls { "rediss" } else { "redis" };
    let auth = match (&args.user, &args.password) {
        (Some(u), Some(p)) => format!("{}:{}@", enc(u), enc(p)),
        (None, Some(p)) => format!(":{}@", enc(p)),
        (Some(u), None) => format!("{}@", enc(u)),
        (None, None) => String::new(),
    };
    let mut u = format!("{scheme}://{auth}{}:{}", args.host, args.port);
    let frag = if args.tls && args.tls_skip_verify {
        "#insecure"
    } else {
        ""
    };
    if args.resp == 3 {
        u.push_str("/?protocol=resp3");
        u.push_str(frag);
    } else if !frag.is_empty() {
        u.push('/');
        u.push_str(frag);
    }
    u
}

/// rustls 0.23 needs a process-level CryptoProvider installed before any TLS
/// handshake; the redis crate's rustls feature doesn't install one for us.
fn ensure_crypto_provider() {
    use std::sync::Once;
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let _ = rustls::crypto::ring::default_provider().install_default();
    });
}

/// Open one connection according to the CLI (single endpoint or cluster).
pub async fn connect(args: &Args) -> Result<AnyConn> {
    if args.tls {
        ensure_crypto_provider();
    }
    if args.tls && (args.cacert.is_some() || args.cert.is_some()) && !args.tls_skip_verify {
        // mTLS / custom-CA path is a follow-up milestone; insecure TLS is supported now.
        anyhow::bail!(
            "--cacert/--cert/--key (verified TLS) not yet implemented; use --tls --tls-skip-verify"
        );
    }
    let u = url(args);
    if args.cluster_mode {
        let client = redis::cluster::ClusterClient::builder(vec![u])
            .build()
            .context("building cluster client")?;
        let conn = client
            .get_async_connection()
            .await
            .context("cluster connect")?;
        Ok(AnyConn::Cluster(conn))
    } else {
        let client = redis::Client::open(u).context("opening client")?;
        let conn = client
            .get_multiplexed_async_connection()
            .await
            .context("single connect")?;
        Ok(AnyConn::Single(conn))
    }
}
