//! Encryption key management using system keyring
use keyring::Entry;
use pulsearc_domain::{PulseArcError, Result};
use rand::Rng;

const SERVICE_NAME: &str = "com.pulsearc.app";
const KEY_NAME: &str = "database_encryption_key";

/// Manages encryption keys using the system keyring
pub struct KeyManager;

impl KeyManager {
    /// Get or create an encryption key
    pub fn get_or_create_key() -> Result<String> {
        let entry = Entry::new(SERVICE_NAME, KEY_NAME)
            .map_err(|e| PulseArcError::Security(format!("Failed to access keyring: {}", e)))?;

        // Try to get existing key
        match entry.get_password() {
            Ok(key) => Ok(key),
            Err(_) => {
                // Generate a new key
                let key = Self::generate_key();

                // Store it in the keyring
                entry
                    .set_password(&key)
                    .map_err(|e| PulseArcError::Security(format!("Failed to store key: {}", e)))?;

                Ok(key)
            }
        }
    }

    /// Generate a new random encryption key
    fn generate_key() -> String {
        let mut rng = rand::thread_rng();
        let key: Vec<u8> = (0..32).map(|_| rng.gen()).collect();
        hex::encode(key)
    }

    /// Delete the stored encryption key (use with caution!)
    pub fn delete_key() -> Result<()> {
        let entry = Entry::new(SERVICE_NAME, KEY_NAME)
            .map_err(|e| PulseArcError::Security(format!("Failed to access keyring: {}", e)))?;

        entry
            .delete_password()
            .map_err(|e| PulseArcError::Security(format!("Failed to delete key: {}", e)))?;

        Ok(())
    }
}
