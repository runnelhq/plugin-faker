//! Random numbers that are the same on every platform: SplitMix64, seeded from a hash of
//! the seed, the record and the field. So a value depends only on these, not on the order
//! of generation or on other fields.

/// SplitMix64 (Steele, Lea, Flood 2014).
#[derive(Debug, Clone)]
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed)
    }

    /// A generator for one value: seed, a number (e.g. the record) and a name (e.g. the
    /// field) together.
    pub fn for_key(seed: u64, n: u64, name: &str) -> Rng {
        Rng(mix(seed
            ^ mix(n.wrapping_add(0x9E37_79B9_7F4A_7C15))
            ^ fnv(name.as_bytes())))
    }

    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        mix(self.0)
    }

    /// A number in 0..n (n > 0).
    pub fn below(&mut self, n: u64) -> u64 {
        ((u128::from(self.next_u64()) * u128::from(n.max(1))) >> 64) as u64
    }

    /// A number in min..=max.
    pub fn range_i64(&mut self, min: i64, max: i64) -> i64 {
        let span = (max as i128 - min as i128 + 1) as u128;
        if span > u128::from(u64::MAX) {
            return self.next_u64() as i64;
        }
        (min as i128 + i128::from(self.below(span as u64))) as i64
    }

    /// A number in 0.0..1.0.
    pub fn unit(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }

    pub fn chance(&mut self, rate: f64) -> bool {
        self.unit() < rate
    }

    pub fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[self.below(items.len() as u64) as usize]
    }
}

fn mix(mut z: u64) -> u64 {
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// FNV-1a, 64 bit.
pub fn fnv(data: &[u8]) -> u64 {
    data.iter().fold(0xCBF2_9CE4_8422_2325, |h, b| {
        (h ^ u64::from(*b)).wrapping_mul(0x0100_0000_01B3)
    })
}
