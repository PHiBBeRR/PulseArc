//! Integration tests for testing utilities
//!
//! Verifies that testing utilities work correctly together

use std::time::Duration;

use pulsearc_common::testing::fixtures::{
    random_bool_seeded, random_email_seeded, random_string_seeded, random_u64_seeded,
};
use pulsearc_common::testing::temp::{TempDir, TempFile};
use tokio::time::sleep;

/// Test that fixtures produce deterministic output when seeded
#[test]
fn test_fixtures_deterministic() {
    let seed1 = 12345u64;
    let seed2 = 12345u64;

    // Note: random_uuid_seeded doesn't exist, removed from test
    assert_eq!(random_email_seeded(seed1), random_email_seeded(seed2));
    assert_eq!(random_string_seeded(20, seed1), random_string_seeded(20, seed2));
    assert_eq!(random_bool_seeded(seed1), random_bool_seeded(seed2));
    assert_eq!(random_u64_seeded(seed1), random_u64_seeded(seed2));
}

/// Test that fixtures produce different output with different seeds
#[test]
fn test_fixtures_random() {
    let seed1 = 11111u64;
    let seed2 = 22222u64;

    // Note: random_uuid_seeded doesn't exist, removed from test
    assert_ne!(random_email_seeded(seed1), random_email_seeded(seed2));
}

/// Test temp files cleanup
#[tokio::test(flavor = "multi_thread")]
async fn test_temp_cleanup() {
    let paths;
    {
        let dir = TempDir::new("test").expect("create dir");
        let file = TempFile::new("test", "tmp").expect("create file");
        paths = (dir.path().to_path_buf(), file.path().to_path_buf());
        assert!(paths.0.exists());
        assert!(paths.1.exists());
    }
    sleep(Duration::from_millis(10)).await;
    assert!(!paths.0.exists());
    assert!(!paths.1.exists());
}
