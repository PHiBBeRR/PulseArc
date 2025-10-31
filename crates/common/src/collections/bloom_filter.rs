//! Bloom filter for probabilistic membership testing.
//!
//! Provides a hardened, space-efficient data structure suitable for enterprise
//! workloads.

use std::convert::TryInto;
use std::fmt;
use std::hash::{Hash, Hasher};

use rand::rngs::OsRng;
use rand::RngCore;

/// Errors returned by [`BloomFilter::new`] / [`BloomFilter::try_new`].
#[derive(Debug, Clone, PartialEq)]
pub enum BloomError {
    /// Parameters are invalid (e.g., `expected_items == 0` or
    /// `false_positive_rate` not in (0,1)).
    InvalidParameters { expected_items: usize, false_positive_rate: f64 },
    /// The computed bitset size is too large and exceeds the configured
    /// maximum.
    AllocationTooLarge { bits: usize },
}

impl fmt::Display for BloomError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BloomError::InvalidParameters { expected_items, false_positive_rate } => write!(
                f,
                "Invalid bloom filter parameters: expected_items={}, false_positive_rate={}",
                expected_items, false_positive_rate
            ),
            BloomError::AllocationTooLarge { bits } => {
                write!(f, "Requested bloom filter size too large: {} bits", bits)
            }
        }
    }
}

impl std::error::Error for BloomError {}

/// A bloom filter for probabilistic membership testing.
///
/// ```rust
/// use pulsearc_common::collections::BloomFilter;
///
/// let mut filter = BloomFilter::new(1000, 0.01).unwrap();
/// filter.insert(&"hello");
/// filter.insert(&"world");
///
/// assert!(filter.contains(&"hello"));
/// assert!(filter.contains(&"world"));
/// ```
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BloomFilter {
    bits: Vec<u64>,
    bit_len: usize,
    num_hashes: usize,
    inserted: usize,
    keys: HashKeys,
}

impl BloomFilter {
    /// Maximum number of bits to allocate for the filter (~128 MiB).
    const MAX_BITS: usize = 1 << 30; // 1,073,741,824 bits ~ 128 MiB

    /// Create a new bloom filter using random per-instance hash keys.
    ///
    /// # Arguments
    /// * `expected_items` - Expected number of items to insert (must be > 0)
    /// * `false_positive_rate` - Desired false positive rate (0.0 - 1.0,
    ///   exclusive)
    ///
    /// Returns an error on invalid parameters or if the computed allocation is
    /// too large.
    pub fn new(expected_items: usize, false_positive_rate: f64) -> Result<Self, BloomError> {
        Self::with_keys(expected_items, false_positive_rate, HashKeys::random())
    }

    /// Create a new bloom filter with caller-provided seed material.
    ///
    /// Useful for deterministic testing or reproducible deployments.
    pub fn with_seed(
        expected_items: usize,
        false_positive_rate: f64,
        seed: [u8; 32],
    ) -> Result<Self, BloomError> {
        Self::with_keys(expected_items, false_positive_rate, HashKeys::from_seed(seed))
    }

    /// Try to create a bloom filter. Identical to [`BloomFilter::new`].
    pub fn try_new(expected_items: usize, false_positive_rate: f64) -> Result<Self, BloomError> {
        Self::new(expected_items, false_positive_rate)
    }

    fn with_keys(
        expected_items: usize,
        false_positive_rate: f64,
        keys: HashKeys,
    ) -> Result<Self, BloomError> {
        if expected_items == 0 || !(0.0..1.0).contains(&false_positive_rate) {
            return Err(BloomError::InvalidParameters { expected_items, false_positive_rate });
        }

        let bit_len = Self::optimal_size(expected_items, false_positive_rate);
        let num_hashes = Self::optimal_hashes(bit_len, expected_items);

        if bit_len == 0 || num_hashes == 0 {
            return Err(BloomError::InvalidParameters { expected_items, false_positive_rate });
        }
        if bit_len > Self::MAX_BITS {
            return Err(BloomError::AllocationTooLarge { bits: bit_len });
        }

        let word_len = bit_len.div_ceil(64);

        Ok(Self { bits: vec![0; word_len], bit_len, num_hashes, inserted: 0, keys })
    }

    /// Insert an item into the filter.
    pub fn insert<T: ?Sized + Hash>(&mut self, item: &T) {
        let (h1, mut h2) = self.base_hashes(item);
        let m = self.bit_len as u64;
        if m == 0 {
            return;
        }
        h2 |= 1; // keep the second hash odd to avoid poor dispersion

        for i in 0..self.num_hashes as u64 {
            let idx = h1.wrapping_add(i.wrapping_mul(h2)) % m;
            self.set_bit(idx as usize);
        }
        self.inserted = self.inserted.saturating_add(1);
    }

    /// Check if an item might be in the filter.
    ///
    /// Returns `true` if the item might be in the set (with possible false
    /// positives). Returns `false` if the item is definitely not in the
    /// set.
    pub fn contains<T: ?Sized + Hash>(&self, item: &T) -> bool {
        let (h1, mut h2) = self.base_hashes(item);
        let m = self.bit_len as u64;
        if m == 0 {
            return false;
        }
        h2 |= 1;

        for i in 0..self.num_hashes as u64 {
            let idx = h1.wrapping_add(i.wrapping_mul(h2)) % m;
            if !self.test_bit(idx as usize) {
                return false;
            }
        }
        true
    }

    /// Clear all items from the filter (resets all bits and the insert
    /// counter).
    pub fn clear(&mut self) {
        self.bits.fill(0);
        self.inserted = 0;
    }

    /// Get the size of the filter in bits.
    pub fn size(&self) -> usize {
        self.bit_len
    }

    /// Get the number of hash functions used.
    pub fn num_hashes(&self) -> usize {
        self.num_hashes
    }

    /// Estimate the current false positive rate using the standard formula:
    /// `fpr ≈ (1 - e^{-k n / m})^k`
    pub fn estimated_false_positive_rate(&self) -> f64 {
        if self.inserted == 0 || self.bit_len == 0 {
            return 0.0;
        }
        let m = self.bit_len as f64;
        let k = self.num_hashes as f64;
        let n = self.inserted as f64;

        let inner = (-k * n / m).exp();
        let fpr = (1.0 - inner).powf(k);
        fpr.clamp(0.0, 1.0)
    }

    fn set_bit(&mut self, idx: usize) {
        let (word, mask) = Self::bit_position(idx);
        if let Some(entry) = self.bits.get_mut(word) {
            *entry |= mask;
        }
    }

    fn test_bit(&self, idx: usize) -> bool {
        let (word, mask) = Self::bit_position(idx);
        self.bits.get(word).is_some_and(|entry| (*entry & mask) != 0)
    }

    fn bit_position(idx: usize) -> (usize, u64) {
        let word = idx / 64;
        let bit = (idx % 64) as u32;
        (word, 1u64 << bit)
    }

    fn base_hashes<T: ?Sized + Hash>(&self, item: &T) -> (u64, u64) {
        let mut collector = BytesCollector::default();
        item.hash(&mut collector);
        let data = collector.as_slice();

        let h1 = self.keys.keyed_hash(0u8, data);
        let h2 = self.keys.keyed_hash(1u8, data);
        (h1, h2)
    }

    // Calculate optimal filter size.
    fn optimal_size(n: usize, p: f64) -> usize {
        let ln2_sq = (2f64.ln()).powi(2);
        let m = -(n as f64 * p.ln()) / ln2_sq;
        if !m.is_finite() || m <= 0.0 {
            0
        } else {
            m.ceil() as usize
        }
    }

    // Calculate optimal number of hash functions.
    fn optimal_hashes(m: usize, n: usize) -> usize {
        if n == 0 || m == 0 {
            return 0;
        }
        let k = (m as f64 / n as f64) * 2f64.ln();
        if !k.is_finite() || k <= 0.0 {
            0
        } else {
            k.ceil() as usize
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
struct HashKeys {
    key: [u8; 32],
}

impl HashKeys {
    fn random() -> Self {
        let mut seed = [0u8; 32];
        OsRng.fill_bytes(&mut seed);
        Self::from_seed(seed)
    }

    fn from_seed(seed: [u8; 32]) -> Self {
        let mut key = seed;
        if key.iter().all(|&b| b == 0) {
            key = [0xA5; 32];
        }
        Self { key }
    }

    fn keyed_hash(&self, domain: u8, data: &[u8]) -> u64 {
        let mut hasher = blake3::Hasher::new_keyed(&self.key);
        hasher.update(&[domain]);
        hasher.update(data);
        let out = hasher.finalize();
        let bytes: [u8; 8] = out.as_bytes()[..8].try_into().expect("slice has 8 bytes");
        u64::from_le_bytes(bytes)
    }
}

#[derive(Default)]
struct BytesCollector {
    bytes: Vec<u8>,
}

impl BytesCollector {
    fn as_slice(&self) -> &[u8] {
        &self.bytes
    }
}

impl Hasher for BytesCollector {
    fn write(&mut self, bytes: &[u8]) {
        self.bytes.extend_from_slice(bytes);
    }

    fn finish(&self) -> u64 {
        // Not used in this implementation; return a stable placeholder.
        0
    }
}

#[cfg(test)]
mod tests {
    //! Unit tests for collections::bloom_filter.
    use super::*;

    const TEST_SEED: [u8; 32] = [42; 32];

    /// Validates `BloomFilter::with_seed` behavior for the insert and contains
    /// scenario.
    ///
    /// Assertions:
    /// - Ensures `filter.contains(&"hello")` evaluates to true.
    /// - Ensures `filter.contains(&"world")` evaluates to true.
    #[test]
    fn insert_and_contains() {
        let mut filter = BloomFilter::with_seed(100, 0.01, TEST_SEED).unwrap();

        filter.insert(&"hello");
        filter.insert(&"world");

        assert!(filter.contains(&"hello"));
        assert!(filter.contains(&"world"));
    }

    /// Validates `BloomFilter::with_seed` behavior for the not contains when
    /// empty scenario.
    ///
    /// Assertions:
    /// - Ensures `!filter.contains(&"hello")` evaluates to true.
    #[test]
    fn not_contains_when_empty() {
        let filter = BloomFilter::with_seed(100, 0.01, TEST_SEED).unwrap();
        assert!(!filter.contains(&"hello"));
    }

    /// Validates `BloomFilter::with_seed` behavior for the clear resets state
    /// scenario.
    ///
    /// Assertions:
    /// - Ensures `filter.contains(&"hello")` evaluates to true.
    /// - Ensures `!filter.contains(&"hello")` evaluates to true.
    /// - Confirms `filter.estimated_false_positive_rate()` equals `0.0`.
    #[test]
    fn clear_resets_state() {
        let mut filter = BloomFilter::with_seed(100, 0.01, TEST_SEED).unwrap();
        filter.insert(&"hello");
        assert!(filter.contains(&"hello"));
        filter.clear();
        assert!(!filter.contains(&"hello"));
        assert_eq!(filter.estimated_false_positive_rate(), 0.0);
    }

    /// Validates `BloomFilter::with_seed` behavior for the size and hashes
    /// positive scenario.
    ///
    /// Assertions:
    /// - Ensures `filter.size() > 0` evaluates to true.
    /// - Ensures `filter.num_hashes() > 0` evaluates to true.
    #[test]
    fn size_and_hashes_positive() {
        let filter = BloomFilter::with_seed(1000, 0.01, TEST_SEED).unwrap();
        assert!(filter.size() > 0);
        assert!(filter.num_hashes() > 0);
    }

    /// Validates `BloomFilter::with_seed` behavior for the estimated false
    /// positive rate within bounds scenario.
    ///
    /// Assertions:
    /// - Ensures `(0.0..=1.0).contains(&fpr)` evaluates to true.
    #[test]
    fn estimated_false_positive_rate_within_bounds() {
        let mut filter = BloomFilter::with_seed(100, 0.01, TEST_SEED).unwrap();
        for i in 0..50 {
            filter.insert(&i);
        }
        let fpr = filter.estimated_false_positive_rate();
        assert!((0.0..=1.0).contains(&fpr));
    }

    /// Validates `BloomFilter::with_seed` behavior for the deterministic with
    /// shared seed scenario.
    ///
    /// Assertions:
    /// - Confirms `a.bits` equals `b.bits`.
    #[test]
    fn deterministic_with_shared_seed() {
        let mut a = BloomFilter::with_seed(100, 0.01, TEST_SEED).unwrap();
        let mut b = BloomFilter::with_seed(100, 0.01, TEST_SEED).unwrap();

        a.insert(&"consistent");
        b.insert(&"consistent");

        assert_eq!(a.bits, b.bits);
    }

    /// Validates `BloomFilter::new` behavior for the invalid parameters
    /// rejected scenario.
    ///
    /// Assertions:
    /// - Ensures `matches!(err, BloomError::InvalidParameters { .. })`
    ///   evaluates to true.
    /// - Ensures `matches!(err, BloomError::InvalidParameters { .. })`
    ///   evaluates to true.
    #[test]
    fn invalid_parameters_rejected() {
        let err = BloomFilter::new(0, 0.01).unwrap_err();
        assert!(matches!(err, BloomError::InvalidParameters { .. }));

        let err = BloomFilter::new(10, 0.0).unwrap_err();
        assert!(matches!(err, BloomError::InvalidParameters { .. }));
    }

    /// Validates `BloomFilter::new` behavior for the allocation too large
    /// rejected scenario.
    ///
    /// Assertions:
    /// - Ensures `matches!(err, BloomError::AllocationTooLarge { .. })`
    ///   evaluates to true.
    #[test]
    fn allocation_too_large_rejected() {
        let err = BloomFilter::new(60_000_000, 0.0001).unwrap_err();
        assert!(matches!(err, BloomError::AllocationTooLarge { .. }));
    }

    /// Validates `HashKeys::from_seed` behavior for the zero seed is hardened
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `keys.key` differs from `[0u8; 32]`.
    #[test]
    fn zero_seed_is_hardened() {
        let keys = HashKeys::from_seed([0u8; 32]);
        assert_ne!(keys.key, [0u8; 32]);
    }
}
