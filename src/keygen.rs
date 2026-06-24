use crate::cli::KeyPattern;
use rand::Rng;
use rand_distr::{Distribution, Normal};

/// Generates keys as `{prefix}{id}` with `id` drawn from [min, max] per the pattern.
pub struct KeyGen {
    prefix: String,
    min: u64,
    max: u64,
    pattern: KeyPattern,
    seq: u64,
    normal: Option<Normal<f64>>,
}

impl KeyGen {
    pub fn new(prefix: &str, min: u64, max: u64, pattern: KeyPattern) -> Self {
        let lo = min.min(max);
        let hi = min.max(max);
        let span = (hi - lo) as f64;
        let normal = if pattern == KeyPattern::Gaussian {
            Normal::new(lo as f64 + span / 2.0, (span / 6.0).max(1.0)).ok()
        } else {
            None
        };
        Self {
            prefix: prefix.to_string(),
            min: lo,
            max: hi,
            pattern,
            seq: 0,
            normal,
        }
    }

    pub fn next<R: Rng>(&mut self, rng: &mut R) -> String {
        let id = match self.pattern {
            KeyPattern::Random => rng.gen_range(self.min..=self.max),
            KeyPattern::Sequential => {
                // u128 span avoids overflow / divide-by-zero when max == u64::MAX
                let span = (self.max - self.min) as u128 + 1;
                let v = self.min + (self.seq as u128 % span) as u64;
                self.seq = self.seq.wrapping_add(1);
                v
            }
            KeyPattern::Gaussian => {
                let n = self.normal.as_ref().expect("gaussian normal");
                let s = n.sample(rng).round() as i64;
                s.clamp(self.min as i64, self.max as i64) as u64
            }
        };
        let mut k = String::with_capacity(self.prefix.len() + 20);
        k.push_str(&self.prefix);
        k.push_str(itoa(id).as_str());
        k
    }
}

#[inline]
fn itoa(n: u64) -> String {
    n.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::KeyPattern;
    use rand::rngs::SmallRng;
    use rand::SeedableRng;

    #[test]
    fn random_in_range_with_prefix() {
        let mut g = KeyGen::new("k:", 10, 20, KeyPattern::Random);
        let mut rng = SmallRng::seed_from_u64(1);
        for _ in 0..2000 {
            let k = g.next(&mut rng);
            assert!(k.starts_with("k:"), "prefix missing: {k}");
            let id: u64 = k[2..].parse().expect("numeric id");
            assert!((10..=20).contains(&id), "id {id} out of range");
        }
    }

    #[test]
    fn sequential_wraps() {
        let mut g = KeyGen::new("", 1, 3, KeyPattern::Sequential);
        let mut rng = SmallRng::seed_from_u64(1);
        let seq: Vec<String> = (0..5).map(|_| g.next(&mut rng)).collect();
        assert_eq!(seq, vec!["1", "2", "3", "1", "2"]);
    }

    #[test]
    fn gaussian_in_range() {
        let mut g = KeyGen::new("p", 100, 200, KeyPattern::Gaussian);
        let mut rng = SmallRng::seed_from_u64(7);
        for _ in 0..2000 {
            let k = g.next(&mut rng);
            let id: u64 = k[1..].parse().unwrap();
            assert!((100..=200).contains(&id));
        }
    }

    #[test]
    fn swapped_min_max_is_tolerated() {
        let mut g = KeyGen::new("x", 20, 10, KeyPattern::Random);
        let mut rng = SmallRng::seed_from_u64(3);
        let id: u64 = g.next(&mut rng)[1..].parse().unwrap();
        assert!((10..=20).contains(&id));
    }

    #[test]
    fn sequential_at_u64_max_does_not_panic() {
        let mut g = KeyGen::new("k", 0, u64::MAX, KeyPattern::Sequential);
        let mut rng = SmallRng::seed_from_u64(1);
        for _ in 0..5 {
            let _ = g.next(&mut rng); // previously div-by-zero / overflow
        }
    }
}
