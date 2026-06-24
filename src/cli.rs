use clap::Parser;

/// Key access pattern (memtier-compatible subset).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyPattern {
    /// Random uniform over [min, max]
    Random,
    /// Sequential, wrapping
    Sequential,
    /// Gaussian centered in the range
    Gaussian,
}

impl std::str::FromStr for KeyPattern {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, String> {
        match s {
            "R" | "r" => Ok(KeyPattern::Random),
            "S" | "s" => Ok(KeyPattern::Sequential),
            "G" | "g" => Ok(KeyPattern::Gaussian),
            other => Err(format!("invalid --key-pattern '{other}' (use R|S|G)")),
        }
    }
}

/// memtier-style Redis benchmark, GET-focused.
#[derive(Parser, Debug, Clone)]
#[command(name = "redis-benchmark-rs", version, about, disable_help_flag = true)]
pub struct Args {
    /// Print help (the short -h is used for --host, as in memtier/redis-cli)
    #[arg(long, action = clap::ArgAction::Help)]
    help: Option<bool>,

    /// Server host (single-endpoint) or one cluster seed
    #[arg(short = 'h', long, default_value = "127.0.0.1")]
    pub host: String,

    /// Server port
    #[arg(short = 'p', long, default_value_t = 6379)]
    pub port: u16,

    /// Use the OSS cluster API (slot routing). Omit for a single endpoint.
    #[arg(long)]
    pub cluster_mode: bool,

    /// Enable TLS
    #[arg(long)]
    pub tls: bool,

    /// Skip TLS certificate verification
    #[arg(long)]
    pub tls_skip_verify: bool,

    /// CA certificate (PEM) for TLS verification
    #[arg(long)]
    pub cacert: Option<String>,

    /// Client certificate (PEM) for mTLS
    #[arg(long)]
    pub cert: Option<String>,

    /// Client key (PEM) for mTLS
    #[arg(long)]
    pub key: Option<String>,

    /// Username for AUTH (ACL); omit for the default user
    #[arg(long)]
    pub user: Option<String>,

    /// Password for AUTH
    #[arg(short = 'a', long)]
    pub password: Option<String>,

    /// RESP protocol version: 2 or 3 (memtier -P parity)
    #[arg(long, default_value_t = 2)]
    pub resp: u8,

    /// Number of worker threads
    #[arg(short = 't', long, default_value_t = 4)]
    pub threads: usize,

    /// Connections per thread
    #[arg(short = 'c', long, default_value_t = 50)]
    pub connections: usize,

    /// set:get ratio (e.g. 0:1 = pure GET, 1:10 = 1 SET per 10 GET)
    #[arg(long, default_value = "0:1")]
    pub ratio: String,

    /// Key access pattern: R(andom) | S(equential) | G(aussian)
    #[arg(long, default_value = "R")]
    pub key_pattern: KeyPattern,

    /// Key prefix (the literal text prepended to the numeric id)
    #[arg(long, default_value = "memtier-")]
    pub key_prefix: String,

    /// Minimum key id
    #[arg(long, default_value_t = 1)]
    pub key_minimum: u64,

    /// Maximum key id
    #[arg(long, default_value_t = 10_000_000)]
    pub key_maximum: u64,

    /// SET payload size in bytes (only used when --ratio has writes)
    #[arg(long, default_value_t = 32)]
    pub data_size: usize,

    /// Set keys with a random expiry (seconds) from RANGE = min-max (e.g. 100-3600), like memtier
    #[arg(long, value_name = "RANGE")]
    pub expiry_range: Option<String>,

    /// Pipeline depth (in-flight commands per connection)
    #[arg(long, default_value_t = 1)]
    pub pipeline: usize,

    /// Per-connection rate limit in COMMANDS/sec (0 = unlimited)
    #[arg(long, default_value_t = 0)]
    pub rate_limiting: u64,

    /// Test duration in seconds
    #[arg(long, default_value_t = 10)]
    pub test_time: u64,

    /// Percentiles to print (comma-separated)
    #[arg(long, default_value = "50,99,99.9,99.99,99.999")]
    pub print_percentiles: String,

    /// Write a JSON results file
    #[arg(long)]
    pub json_out_file: Option<String>,

    /// Suppress the histogram dump
    #[arg(long)]
    pub hide_histogram: bool,

    /// Print a per-interval realtime line (throughput + miss ratio + latency percentiles) to stderr
    #[arg(long)]
    pub realtime_latencies: bool,

    /// Realtime report interval in seconds (with --realtime-latencies)
    #[arg(long, default_value_t = 1.0)]
    pub realtime_interval: f64,
}

impl Args {
    pub fn total_connections(&self) -> usize {
        self.threads * self.connections
    }

    /// Parse "set:get" into (set_weight, get_weight).
    pub fn parse_ratio(&self) -> anyhow::Result<(u32, u32)> {
        let parts: Vec<&str> = self.ratio.split(':').collect();
        anyhow::ensure!(parts.len() == 2, "--ratio must be set:get (e.g. 1:10)");
        let s: u32 = parts[0].parse()?;
        let g: u32 = parts[1].parse()?;
        anyhow::ensure!(s as u64 + g as u64 > 0, "--ratio set+get must be > 0");
        Ok((s, g))
    }

    /// Parse `--expiry-range=min-max` into (min, max) seconds, if set.
    pub fn expiry(&self) -> anyhow::Result<Option<(u64, u64)>> {
        let Some(s) = &self.expiry_range else {
            return Ok(None);
        };
        let (a, b) = s
            .split_once('-')
            .ok_or_else(|| anyhow::anyhow!("--expiry-range must be min-max, e.g. 100-3600"))?;
        let lo: u64 = a.trim().parse()?;
        let hi: u64 = b.trim().parse()?;
        anyhow::ensure!(lo >= 1 && hi >= lo, "--expiry-range needs 1 <= min <= max");
        Ok(Some((lo, hi)))
    }

    /// Validate all inputs up front; returns a clear error for bad values/combos.
    pub fn validate(&self) -> anyhow::Result<()> {
        self.parse_ratio()?;
        self.expiry()?;
        anyhow::ensure!(self.threads > 0, "--threads must be > 0");
        anyhow::ensure!(self.connections > 0, "--connections must be > 0");
        anyhow::ensure!(self.test_time > 0, "--test-time must be > 0");
        anyhow::ensure!(self.resp == 2 || self.resp == 3, "--resp must be 2 or 3");
        for tok in self.print_percentiles.split(',') {
            let tok = tok.trim();
            if tok.is_empty() {
                continue;
            }
            let p: f64 = tok
                .parse()
                .map_err(|_| anyhow::anyhow!("invalid --print-percentiles entry '{tok}'"))?;
            anyhow::ensure!(p > 0.0 && p < 100.0, "percentile {p} must be in (0, 100)");
        }
        Ok(())
    }

    pub fn percentiles(&self) -> Vec<f64> {
        self.print_percentiles
            .split(',')
            .filter_map(|p| p.trim().parse().ok())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn defaults() -> Args {
        Args::parse_from(["redis-benchmark-rs"])
    }

    #[test]
    fn ratio_parsing() {
        let mut a = defaults();
        a.ratio = "1:10".into();
        assert_eq!(a.parse_ratio().unwrap(), (1, 10));
        a.ratio = "0:1".into();
        assert_eq!(a.parse_ratio().unwrap(), (0, 1));
        a.ratio = "0:0".into();
        assert!(a.parse_ratio().is_err());
        a.ratio = "abc".into();
        assert!(a.parse_ratio().is_err());
    }

    #[test]
    fn percentiles_parse() {
        let mut a = defaults();
        a.print_percentiles = "50,99,99.9".into();
        assert_eq!(a.percentiles(), vec![50.0, 99.0, 99.9]);
    }

    #[test]
    fn key_pattern_from_str() {
        assert_eq!("R".parse::<KeyPattern>().unwrap(), KeyPattern::Random);
        assert_eq!("S".parse::<KeyPattern>().unwrap(), KeyPattern::Sequential);
        assert_eq!("G".parse::<KeyPattern>().unwrap(), KeyPattern::Gaussian);
        assert!("Z".parse::<KeyPattern>().is_err());
    }

    #[test]
    fn total_connections_multiplies() {
        let mut a = defaults();
        a.threads = 4;
        a.connections = 25;
        assert_eq!(a.total_connections(), 100);
    }

    #[test]
    fn validate_rejects_zero_inputs() {
        let setters: [fn(&mut Args); 3] = [
            |a| a.connections = 0,
            |a| a.threads = 0,
            |a| a.test_time = 0,
        ];
        for set in setters {
            let mut a = defaults();
            set(&mut a);
            assert!(a.validate().is_err());
        }
    }

    #[test]
    fn validate_rejects_bad_percentiles() {
        let mut a = defaults();
        a.print_percentiles = "50,150".into();
        assert!(a.validate().is_err());
        a.print_percentiles = "50,foo".into();
        assert!(a.validate().is_err());
    }

    #[test]
    fn ratio_overflow_does_not_panic() {
        let mut a = defaults();
        a.ratio = "4000000000:4000000000".into(); // s+g overflows u32
        let _ = a.parse_ratio(); // must not panic
    }

    #[test]
    fn expiry_range_parse() {
        let mut a = defaults();
        assert_eq!(a.expiry().unwrap(), None);
        a.expiry_range = Some("100-3600".into());
        assert_eq!(a.expiry().unwrap(), Some((100, 3600)));
        a.expiry_range = Some("3600-100".into()); // min > max
        assert!(a.expiry().is_err());
        a.expiry_range = Some("nope".into());
        assert!(a.expiry().is_err());
    }

    #[test]
    fn validate_resp_version() {
        let mut a = defaults();
        a.resp = 3;
        assert!(a.validate().is_ok());
        a.resp = 4;
        assert!(a.validate().is_err());
    }
}
