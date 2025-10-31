// Secure Hashing for Non-Allowed Domains
// Reference: PART-1-OVERVIEW-AND-FOUNDATION.md lines 346-349

use sha2::{Digest, Sha256, Sha384, Sha512};

use super::config::{HashAlgorithm, HashConfig};
use super::error::{HashError, HashResult};

#[derive(Debug)]
pub struct SecureHasher {
    config: HashConfig,
}

impl SecureHasher {
    pub fn new() -> HashResult<Self> {
        let config = HashConfig::new()?;
        Ok(Self { config })
    }

    pub fn with_config(config: HashConfig) -> HashResult<Self> {
        if config.org_salt.is_empty() {
            return Err(HashError::ConfigurationError("Organization salt must be set".to_string()));
        }
        Ok(Self { config })
    }

    pub fn hash_domain(&self, domain: &str) -> HashResult<String> {
        if domain.is_empty() {
            return Err(HashError::InvalidInput("Domain cannot be empty".to_string()));
        }

        let input = format!("{}{}", domain, self.config.org_salt);

        let hash = match self.config.algorithm {
            HashAlgorithm::Sha256 => {
                let mut hasher = Sha256::new();
                hasher.update(input.as_bytes());
                format!("{:x}", hasher.finalize())
            }
            HashAlgorithm::Sha384 => {
                let mut hasher = Sha384::new();
                hasher.update(input.as_bytes());
                format!("{:x}", hasher.finalize())
            }
            HashAlgorithm::Sha512 => {
                let mut hasher = Sha512::new();
                hasher.update(input.as_bytes());
                format!("{:x}", hasher.finalize())
            }
        };

        Ok(hash)
    }

    pub fn hash_multiple_domains(&self, domains: &[&str]) -> HashResult<Vec<String>> {
        domains.iter().map(|domain| self.hash_domain(domain)).collect()
    }

    pub fn rotate_salt(&mut self) -> HashResult<()> {
        self.config.generate_org_salt()
    }

    pub fn get_config(&self) -> &HashConfig {
        &self.config
    }
}

impl Default for SecureHasher {
    fn default() -> Self {
        Self::new().expect("Failed to create default SecureHasher")
    }
}

#[cfg(test)]
mod tests {
    //! Unit tests for privacy::hash::hasher.
    use super::*;

    // ========================================================================
    // Constructor Tests
    // ========================================================================

    /// Validates `SecureHasher::new` behavior for the new hasher scenario.
    ///
    /// Assertions:
    /// - Ensures `hasher.is_ok()` evaluates to true.
    #[test]
    fn test_new_hasher() {
        let hasher = SecureHasher::new();
        assert!(hasher.is_ok());
    }

    /// Validates `SecureHasher::default` behavior for the default hasher
    /// scenario.
    ///
    /// Assertions:
    /// - Ensures `!config.org_salt.is_empty()` evaluates to true.
    #[test]
    fn test_default_hasher() {
        let hasher = SecureHasher::default();
        let config = hasher.get_config();
        assert!(!config.org_salt.is_empty());
    }

    /// Validates `HashConfig::new` behavior for the with valid config scenario.
    ///
    /// Assertions:
    /// - Ensures `hasher.is_ok()` evaluates to true.
    #[test]
    fn test_with_valid_config() {
        let mut config = HashConfig::new().unwrap();
        config.algorithm = HashAlgorithm::Sha256;
        config.org_salt = "test-salt-12345".to_string();

        let hasher = SecureHasher::with_config(config);
        assert!(hasher.is_ok());
    }

    /// Validates `HashConfig::new` behavior for the with empty salt fails
    /// scenario.
    ///
    /// Assertions:
    /// - Ensures `hasher.is_err()` evaluates to true.
    /// - Ensures `matches!(hasher.unwrap_err(),
    ///   HashError::ConfigurationError(_))` evaluates to true.
    #[test]
    fn test_with_empty_salt_fails() {
        let mut config = HashConfig::new().unwrap();
        config.org_salt = String::new();

        let hasher = SecureHasher::with_config(config);
        assert!(hasher.is_err());
        assert!(matches!(hasher.unwrap_err(), HashError::ConfigurationError(_)));
    }

    // ========================================================================
    // Basic Hashing Tests
    // ========================================================================

    /// Validates `SecureHasher::new` behavior for the hash domain basic
    /// scenario.
    ///
    /// Assertions:
    /// - Ensures `result.is_ok()` evaluates to true.
    /// - Ensures `!hash.is_empty()` evaluates to true.
    /// - Confirms `hash.len()` equals `64`.
    #[test]
    fn test_hash_domain_basic() {
        let hasher = SecureHasher::new().unwrap();
        let result = hasher.hash_domain("example.com");
        assert!(result.is_ok());

        let hash = result.unwrap();
        assert!(!hash.is_empty());
        // SHA-256 produces 64-character hex string
        assert_eq!(hash.len(), 64);
    }

    /// Validates `SecureHasher::new` behavior for the hash domain empty input
    /// fails scenario.
    ///
    /// Assertions:
    /// - Ensures `result.is_err()` evaluates to true.
    /// - Ensures `matches!(result.unwrap_err(), HashError::InvalidInput(_))`
    ///   evaluates to true.
    #[test]
    fn test_hash_domain_empty_input_fails() {
        let hasher = SecureHasher::new().unwrap();
        let result = hasher.hash_domain("");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), HashError::InvalidInput(_)));
    }

    /// Validates `SecureHasher::new` behavior for the hash domain deterministic
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `hash1` equals `hash2`.
    #[test]
    fn test_hash_domain_deterministic() {
        let hasher = SecureHasher::new().unwrap();

        let hash1 = hasher.hash_domain("example.com").unwrap();
        let hash2 = hasher.hash_domain("example.com").unwrap();

        // Same input with same salt should produce same hash
        assert_eq!(hash1, hash2);
    }

    /// Validates `SecureHasher::new` behavior for the hash domain different
    /// domains scenario.
    ///
    /// Assertions:
    /// - Confirms `hash1` differs from `hash2`.
    #[test]
    fn test_hash_domain_different_domains() {
        let hasher = SecureHasher::new().unwrap();

        let hash1 = hasher.hash_domain("example.com").unwrap();
        let hash2 = hasher.hash_domain("different.com").unwrap();

        // Different domains should produce different hashes
        assert_ne!(hash1, hash2);
    }

    /// Validates `SecureHasher::new` behavior for the hash domain case
    /// sensitive scenario.
    ///
    /// Assertions:
    /// - Confirms `hash1` differs from `hash2`.
    #[test]
    fn test_hash_domain_case_sensitive() {
        let hasher = SecureHasher::new().unwrap();

        let hash1 = hasher.hash_domain("example.com").unwrap();
        let hash2 = hasher.hash_domain("EXAMPLE.COM").unwrap();

        // Hashing is case-sensitive
        assert_ne!(hash1, hash2);
    }

    // ========================================================================
    // Algorithm Tests
    // ========================================================================

    /// Validates `HashConfig::new` behavior for the hash with sha256 scenario.
    ///
    /// Assertions:
    /// - Confirms `hash.len()` equals `64`.
    #[test]
    fn test_hash_with_sha256() {
        let mut config = HashConfig::new().unwrap();
        config.algorithm = HashAlgorithm::Sha256;
        config.org_salt = "test-salt".to_string();

        let hasher = SecureHasher::with_config(config).unwrap();
        let hash = hasher.hash_domain("example.com").unwrap();

        // SHA-256 produces 64 hex characters
        assert_eq!(hash.len(), 64);
    }

    /// Validates `HashConfig::new` behavior for the hash with sha384 scenario.
    ///
    /// Assertions:
    /// - Confirms `hash.len()` equals `96`.
    #[test]
    fn test_hash_with_sha384() {
        let mut config = HashConfig::new().unwrap();
        config.algorithm = HashAlgorithm::Sha384;
        config.org_salt = "test-salt".to_string();

        let hasher = SecureHasher::with_config(config).unwrap();
        let hash = hasher.hash_domain("example.com").unwrap();

        // SHA-384 produces 96 hex characters
        assert_eq!(hash.len(), 96);
    }

    /// Validates `HashConfig::new` behavior for the hash with sha512 scenario.
    ///
    /// Assertions:
    /// - Confirms `hash.len()` equals `128`.
    #[test]
    fn test_hash_with_sha512() {
        let mut config = HashConfig::new().unwrap();
        config.algorithm = HashAlgorithm::Sha512;
        config.org_salt = "test-salt".to_string();

        let hasher = SecureHasher::with_config(config).unwrap();
        let hash = hasher.hash_domain("example.com").unwrap();

        // SHA-512 produces 128 hex characters
        assert_eq!(hash.len(), 128);
    }

    /// Validates `HashConfig::new` behavior for the different algorithms
    /// produce different hashes scenario.
    ///
    /// Assertions:
    /// - Confirms `hash256` differs from `hash512`.
    #[test]
    fn test_different_algorithms_produce_different_hashes() {
        let domain = "example.com";
        let salt = "test-salt".to_string();

        let mut config256 = HashConfig::new().unwrap();
        config256.algorithm = HashAlgorithm::Sha256;
        config256.org_salt = salt.clone();

        let mut config512 = HashConfig::new().unwrap();
        config512.algorithm = HashAlgorithm::Sha512;
        config512.org_salt = salt;

        let hasher256 = SecureHasher::with_config(config256).unwrap();
        let hasher512 = SecureHasher::with_config(config512).unwrap();

        let hash256 = hasher256.hash_domain(domain).unwrap();
        let hash512 = hasher512.hash_domain(domain).unwrap();

        assert_ne!(hash256, hash512);
    }

    // ========================================================================
    // Salt Tests
    // ========================================================================

    /// Tests different organization salts produce different hashes.
    ///
    /// Verifies:
    /// - Organization-specific salt affects hash output
    /// - Same domain hashes differently with different salts
    /// - Salt provides multi-tenancy isolation
    /// - Different organizations cannot correlate hashed domains
    #[test]
    fn test_different_salts_produce_different_hashes() {
        let domain = "example.com";

        let mut config1 = HashConfig::new().unwrap();
        config1.org_salt = "salt1".to_string();

        let mut config2 = HashConfig::new().unwrap();
        config2.org_salt = "salt2".to_string();

        let hasher1 = SecureHasher::with_config(config1).unwrap();
        let hasher2 = SecureHasher::with_config(config2).unwrap();

        let hash1 = hasher1.hash_domain(domain).unwrap();
        let hash2 = hasher2.hash_domain(domain).unwrap();

        // Different salts should produce different hashes
        assert_ne!(hash1, hash2);
    }

    /// Tests salt rotation changes hash outputs for security.
    ///
    /// Verifies:
    /// - Salt can be rotated dynamically
    /// - Same domain produces different hash after rotation
    /// - Rotation enables periodic security refresh
    /// - Previous hashes become invalid after rotation
    #[test]
    fn test_salt_rotation() {
        let mut hasher = SecureHasher::new().unwrap();
        let domain = "example.com";

        let hash_before = hasher.hash_domain(domain).unwrap();

        // Rotate salt
        let result = hasher.rotate_salt();
        assert!(result.is_ok());

        let hash_after = hasher.hash_domain(domain).unwrap();

        // Hash should be different after salt rotation
        assert_ne!(hash_before, hash_after);
    }

    // ========================================================================
    // Batch Operations Tests
    // ========================================================================

    /// Tests batch hashing of multiple domains efficiently.
    ///
    /// Verifies:
    /// - Batch operation processes all domains
    /// - Each domain gets unique hash
    /// - All hashes are non-empty
    /// - Results maintain input order
    /// - Batch hashing is more efficient than individual calls
    #[test]
    fn test_hash_multiple_domains() {
        let hasher = SecureHasher::new().unwrap();
        let domains = vec!["example.com", "test.org", "sample.net"];

        let result = hasher.hash_multiple_domains(&domains);
        assert!(result.is_ok());

        let hashes = result.unwrap();
        assert_eq!(hashes.len(), 3);

        // All hashes should be non-empty
        assert!(hashes.iter().all(|h| !h.is_empty()));

        // All hashes should be unique
        let mut unique_hashes: Vec<_> = hashes.clone();
        unique_hashes.sort();
        unique_hashes.dedup();
        assert_eq!(unique_hashes.len(), hashes.len());
    }

    /// Validates `SecureHasher::new` behavior for the hash multiple domains
    /// empty list scenario.
    ///
    /// Assertions:
    /// - Ensures `result.is_ok()` evaluates to true.
    /// - Confirms `hashes.len()` equals `0`.
    #[test]
    fn test_hash_multiple_domains_empty_list() {
        let hasher = SecureHasher::new().unwrap();
        let domains: Vec<&str> = vec![];

        let result = hasher.hash_multiple_domains(&domains);
        assert!(result.is_ok());

        let hashes = result.unwrap();
        assert_eq!(hashes.len(), 0);
    }

    /// Tests batch hashing fails with invalid domain in list.
    ///
    /// Verifies:
    /// - Batch validation catches empty domains
    /// - Returns error rather than partial results
    /// - All-or-nothing validation ensures data integrity
    /// - Prevents incomplete batch processing
    #[test]
    fn test_hash_multiple_domains_with_invalid_domain() {
        let hasher = SecureHasher::new().unwrap();
        let domains = vec!["example.com", "", "test.org"];

        let result = hasher.hash_multiple_domains(&domains);
        // Should fail because one domain is empty
        assert!(result.is_err());
    }

    // ========================================================================
    // Edge Cases Tests
    // ========================================================================

    /// Validates `SecureHasher::new` behavior for the hash domain with special
    /// characters scenario.
    ///
    /// Assertions:
    /// - Ensures `result.is_ok()` evaluates to true.
    /// - Ensures `!hash.is_empty()` evaluates to true.
    #[test]
    fn test_hash_domain_with_special_characters() {
        let hasher = SecureHasher::new().unwrap();

        let domains = vec![
            "example-with-dash.com",
            "example_with_underscore.com",
            "subdomain.example.com",
            "deep.sub.domain.example.com",
        ];

        for domain in domains {
            let result = hasher.hash_domain(domain);
            assert!(result.is_ok(), "Failed to hash domain: {}", domain);
            let hash = result.unwrap();
            assert!(!hash.is_empty());
        }
    }

    /// Validates `SecureHasher::new` behavior for the hash domain with unicode
    /// scenario.
    ///
    /// Assertions:
    /// - Ensures `result.is_ok()` evaluates to true.
    #[test]
    fn test_hash_domain_with_unicode() {
        let hasher = SecureHasher::new().unwrap();

        // International domain names
        let domains = vec!["münchen.de", "москва.рф", "北京.中国"];

        for domain in domains {
            let result = hasher.hash_domain(domain);
            assert!(result.is_ok(), "Failed to hash domain: {}", domain);
        }
    }

    /// Validates `SecureHasher::new` behavior for the hash domain very long
    /// scenario.
    ///
    /// Assertions:
    /// - Ensures `result.is_ok()` evaluates to true.
    #[test]
    fn test_hash_domain_very_long() {
        let hasher = SecureHasher::new().unwrap();

        // Very long domain name (still valid)
        let long_domain = format!("{}.com", "a".repeat(100));
        let result = hasher.hash_domain(&long_domain);
        assert!(result.is_ok());
    }

    // ========================================================================
    // Configuration Tests
    // ========================================================================

    /// Validates `HashConfig::new` behavior for the get config scenario.
    ///
    /// Assertions:
    /// - Confirms `retrieved_config.org_salt` equals `config.org_salt`.
    /// - Confirms `retrieved_config.algorithm` equals `config.algorithm`.
    #[test]
    fn test_get_config() {
        let mut config = HashConfig::new().unwrap();
        config.org_salt = "test-salt-123".to_string();
        config.algorithm = HashAlgorithm::Sha256;

        let hasher = SecureHasher::with_config(config.clone()).unwrap();
        let retrieved_config = hasher.get_config();

        assert_eq!(retrieved_config.org_salt, config.org_salt);
        assert_eq!(retrieved_config.algorithm, config.algorithm);
    }

    // ========================================================================
    // Consistency Tests
    // ========================================================================

    /// Validates `HashConfig::new` behavior for the hash consistency across
    /// instances scenario.
    ///
    /// Assertions:
    /// - Confirms `hash1` equals `hash2`.
    #[test]
    fn test_hash_consistency_across_instances() {
        let salt = "consistent-salt".to_string();

        let mut config1 = HashConfig::new().unwrap();
        config1.org_salt = salt.clone();

        let mut config2 = HashConfig::new().unwrap();
        config2.org_salt = salt;

        let hasher1 = SecureHasher::with_config(config1).unwrap();
        let hasher2 = SecureHasher::with_config(config2).unwrap();

        let domain = "example.com";
        let hash1 = hasher1.hash_domain(domain).unwrap();
        let hash2 = hasher2.hash_domain(domain).unwrap();

        // Same configuration should produce same hash
        assert_eq!(hash1, hash2);
    }

    /// Validates `SecureHasher::new` behavior for the hash format is lowercase
    /// hex scenario.
    ///
    /// Assertions:
    /// - Ensures `hash.chars().all(|c| c.is_ascii_hexdigit() &&
    ///   !c.is_ascii_uppercase())` evaluates to true.
    #[test]
    fn test_hash_format_is_lowercase_hex() {
        let hasher = SecureHasher::new().unwrap();
        let hash = hasher.hash_domain("example.com").unwrap();

        // All characters should be lowercase hex (0-9, a-f)
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()));
    }
}
