//! PKCE (Proof Key for Code Exchange) implementation for OAuth 2.0
//!
//! Implements RFC 7636 for secure OAuth authorization without client secrets.
//! Used for desktop applications where client secrets cannot be safely stored.

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use rand::Rng;
use sha2::{Digest, Sha256};

/// Generate a cryptographically secure code verifier
///
/// Returns a URL-safe base64-encoded random string of 32 bytes (43 characters).
/// Per RFC 7636, verifiers must be 43-128 characters long.
///
/// # Errors
/// Returns error if random number generation fails (extremely rare)
pub fn generate_code_verifier() -> Result<String, String> {
    let mut rng = rand::thread_rng();
    let random_bytes: Vec<u8> = (0..32).map(|_| rng.gen()).collect();
    Ok(URL_SAFE_NO_PAD.encode(random_bytes))
}

/// Generate code challenge from verifier using SHA256
///
/// Per RFC 7636, the challenge is BASE64URL(SHA256(ASCII(code_verifier)))
///
/// # Arguments
/// * `verifier` - The code verifier string
///
/// # Errors
/// Returns error if encoding fails
pub fn generate_code_challenge(verifier: &str) -> Result<String, String> {
    let mut hasher = Sha256::new();
    hasher.update(verifier.as_bytes());
    let hash = hasher.finalize();
    Ok(URL_SAFE_NO_PAD.encode(hash))
}

/// Generate a random state token for CSRF protection
///
/// Returns a URL-safe base64-encoded random string of 32 bytes (43 characters).
///
/// # Errors
/// Returns error if random number generation fails (extremely rare)
pub fn generate_state() -> Result<String, String> {
    let mut rng = rand::thread_rng();
    let random_bytes: Vec<u8> = (0..32).map(|_| rng.gen()).collect();
    Ok(URL_SAFE_NO_PAD.encode(random_bytes))
}

/// Validate that the state token matches
///
/// # Arguments
/// * `expected` - The state that was sent in the authorization request
/// * `actual` - The state received in the callback
///
/// # Returns
/// `true` if states match, `false` otherwise
pub fn validate_state(expected: &str, actual: &str) -> bool {
    expected == actual
}

/// PKCE challenge pair for OAuth 2.0 authorization
///
/// Contains the code verifier (sent during token exchange) and the
/// code challenge (sent during authorization request).
///
/// This is a convenience wrapper around the individual PKCE functions
/// to match the API style from the macOS implementation.
#[derive(Debug, Clone)]
pub struct PKCEChallenge {
    /// Random string (43-128 chars, base64url encoded)
    /// Kept secret until token exchange
    pub code_verifier: String,

    /// SHA256 hash of code_verifier (base64url encoded)
    /// Sent in authorization request for server validation
    pub code_challenge: String,

    /// Random CSRF protection token
    /// Must match between authorization request and callback
    pub state: String,
}

impl PKCEChallenge {
    /// Generate a new PKCE challenge with cryptographically secure random
    /// values
    ///
    /// # Returns
    /// A new `PKCEChallenge` with:
    /// - `code_verifier`: 32 random bytes → 43 chars base64url (within RFC 7636
    ///   43-128 limit)
    /// - `code_challenge`: SHA256(code_verifier) → base64url
    /// - `state`: 32 random bytes → 43 chars base64url for CSRF protection
    ///
    /// # Examples
    /// ```
    /// use pulsearc_common::auth::pkce::PKCEChallenge;
    ///
    /// let challenge = PKCEChallenge::generate().expect("Failed to generate PKCE challenge");
    /// assert!(challenge.code_verifier.len() >= 43);
    /// assert!(challenge.code_verifier.len() <= 128);
    /// ```
    ///
    /// # Errors
    /// Returns error if cryptographic random number generation fails (extremely
    /// rare)
    pub fn generate() -> Result<Self, String> {
        let code_verifier = generate_code_verifier()?;
        let code_challenge = generate_code_challenge(&code_verifier)?;
        let state = generate_state()?;

        Ok(Self { code_verifier, code_challenge, state })
    }

    /// Get the challenge method (always "S256" for SHA256)
    #[must_use]
    pub fn challenge_method(&self) -> &str {
        "S256"
    }
}

#[cfg(test)]
mod tests {
    //! Unit tests for auth::pkce.
    use super::*;

    /// Validates `PKCEChallenge::generate` behavior for the generate pkce
    /// challenge scenario.
    ///
    /// Assertions:
    /// - Ensures `challenge.code_verifier.len() >= 43` evaluates to true.
    /// - Ensures `challenge.code_verifier.len() <= 128` evaluates to true.
    /// - Ensures `!challenge.code_challenge.is_empty()` evaluates to true.
    /// - Ensures `!challenge.state.is_empty()` evaluates to true.
    #[test]
    fn test_generate_pkce_challenge() {
        let challenge = PKCEChallenge::generate().expect("Failed to generate challenge");

        // Verify code_verifier length (RFC 7636: 43-128 chars)
        assert!(
            challenge.code_verifier.len() >= 43,
            "code_verifier too short: {} chars",
            challenge.code_verifier.len()
        );
        assert!(
            challenge.code_verifier.len() <= 128,
            "code_verifier too long: {} chars",
            challenge.code_verifier.len()
        );

        // Verify code_challenge is not empty
        assert!(!challenge.code_challenge.is_empty());

        // Verify state is not empty
        assert!(!challenge.state.is_empty());
    }

    /// Validates `PKCEChallenge::generate` behavior for the unique challenges
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `challenge1.code_verifier` differs from
    ///   `challenge2.code_verifier`.
    /// - Confirms `challenge1.code_challenge` differs from
    ///   `challenge2.code_challenge`.
    /// - Confirms `challenge1.state` differs from `challenge2.state`.
    #[test]
    fn test_unique_challenges() {
        // Each generation should produce unique values
        let challenge1 = PKCEChallenge::generate().expect("Failed to generate challenge 1");
        let challenge2 = PKCEChallenge::generate().expect("Failed to generate challenge 2");

        assert_ne!(challenge1.code_verifier, challenge2.code_verifier);
        assert_ne!(challenge1.code_challenge, challenge2.code_challenge);
        assert_ne!(challenge1.state, challenge2.state);
    }

    /// Validates `PKCEChallenge::generate` behavior for the challenge method
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `challenge.challenge_method()` equals `"S256"`.
    #[test]
    fn test_challenge_method() {
        let challenge = PKCEChallenge::generate().expect("Failed to generate challenge");
        assert_eq!(challenge.challenge_method(), "S256");
    }

    /// Validates `PKCEChallenge::generate` behavior for the base64url encoding
    /// scenario.
    ///
    /// Assertions:
    /// - Ensures `!challenge.code_verifier.contains('=')` evaluates to true.
    /// - Ensures `!challenge.code_challenge.contains('=')` evaluates to true.
    /// - Ensures `!challenge.state.contains('=')` evaluates to true.
    /// - Ensures `!challenge.code_verifier.contains('+')` evaluates to true.
    /// - Ensures `!challenge.code_verifier.contains('/')` evaluates to true.
    /// - Ensures `!challenge.code_challenge.contains('+')` evaluates to true.
    /// - Ensures `!challenge.code_challenge.contains('/')` evaluates to true.
    /// - Ensures `!challenge.state.contains('+')` evaluates to true.
    /// - Ensures `!challenge.state.contains('/')` evaluates to true.
    #[test]
    fn test_base64url_encoding() {
        let challenge = PKCEChallenge::generate().expect("Failed to generate challenge");

        // Verify no padding characters (base64url should not have padding)
        assert!(!challenge.code_verifier.contains('='));
        assert!(!challenge.code_challenge.contains('='));
        assert!(!challenge.state.contains('='));

        // Verify URL-safe characters only (no + or /)
        assert!(!challenge.code_verifier.contains('+'));
        assert!(!challenge.code_verifier.contains('/'));
        assert!(!challenge.code_challenge.contains('+'));
        assert!(!challenge.code_challenge.contains('/'));
        assert!(!challenge.state.contains('+'));
        assert!(!challenge.state.contains('/'));
    }

    /// Validates `PKCEChallenge::generate` behavior for the code challenge
    /// deterministic scenario.
    ///
    /// Assertions:
    /// - Confirms `challenge1.code_challenge` equals `recomputed_challenge`.
    #[test]
    fn test_code_challenge_deterministic() {
        // Same verifier should produce same challenge
        let challenge1 = PKCEChallenge::generate().expect("Failed to generate challenge 1");

        // Manually compute challenge from verifier using the re-exported function
        let recomputed_challenge = generate_code_challenge(&challenge1.code_verifier)
            .expect("Failed to compute challenge");

        assert_eq!(challenge1.code_challenge, recomputed_challenge);
    }
}
