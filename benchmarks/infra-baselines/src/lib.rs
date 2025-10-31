//! Benchmark harness crate for measuring legacy infrastructure performance.

pub fn init_test_encryption_key() {
    // Ensure SQLCipher-based components use a deterministic test key instead of
    // hitting the keychain.
    std::env::set_var(
        "PULSARC_TEST_DB_KEY",
        "test_key_64_chars_long_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    );
}
