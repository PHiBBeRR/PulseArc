use rand::{thread_rng, Rng};
use serde::{Deserialize, Serialize};

use super::error::{HashError, HashResult};

const DEFAULT_SALT_LENGTH: usize = 32;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HashConfig {
    pub org_salt: String,
    pub salt_length: usize,
    pub algorithm: HashAlgorithm,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum HashAlgorithm {
    Sha256,
    Sha384,
    Sha512,
}

impl Default for HashConfig {
    fn default() -> Self {
        Self::new().expect("HashConfig::default should always succeed with secure salt generation")
    }
}

impl HashConfig {
    pub fn new() -> HashResult<Self> {
        Self::with_algorithm(HashAlgorithm::Sha256)
    }

    pub fn with_algorithm(algorithm: HashAlgorithm) -> HashResult<Self> {
        Self::with_algorithm_and_salt_length(algorithm, DEFAULT_SALT_LENGTH)
    }

    pub fn generate_org_salt(&mut self) -> HashResult<()> {
        let salt = generate_secure_salt(self.salt_length)?;
        self.org_salt = salt;
        Ok(())
    }

    pub fn set_org_salt(&mut self, salt: String) -> HashResult<()> {
        if salt.is_empty() {
            return Err(HashError::InvalidInput("Salt cannot be empty".to_string()));
        }
        self.org_salt = salt;
        Ok(())
    }

    pub fn with_algorithm_and_salt_length(
        algorithm: HashAlgorithm,
        salt_length: usize,
    ) -> HashResult<Self> {
        if salt_length == 0 {
            return Err(HashError::InvalidInput("Salt length cannot be zero".to_string()));
        }

        Ok(Self { org_salt: generate_secure_salt(salt_length)?, salt_length, algorithm })
    }
}

fn generate_secure_salt(length: usize) -> HashResult<String> {
    if length == 0 {
        return Err(HashError::InvalidInput("Salt length cannot be zero".to_string()));
    }

    let mut rng = thread_rng();
    let salt: Vec<u8> = (0..length).map(|_| rng.gen()).collect();

    Ok(hex::encode(salt))
}
