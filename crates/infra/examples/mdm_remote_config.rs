//! Example: Fetching MDM configuration from a remote server
//!
//! This example demonstrates how to use the MDM client to fetch
//! configuration from a remote server using the test certificates.
//!
//! # Setup
//!
//! 1. Generate test certificates: ```bash cd scripts/mdm
//!    ./generate-test-certs.sh ```
//!
//! 2. Set up environment variables: ```bash export
//!    MDM_CA_CERT=/path/to/.mdm-certs/ca-cert.pem ```
//!
//! 3. Run a test MDM server (or use the mock below)
//!
//! 4. Run this example: ```bash cargo run --example mdm_remote_config
//!    --features audit-compliance ```

use pulsearc_infra::mdm::{MdmClient, MdmConfig};

#[tokio::main]
#[allow(clippy::type_complexity)]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    println!("MDM Remote Configuration Example");
    println!("=================================\n");

    // Example 1: Using CA certificate (production-like)
    if let Ok(ca_cert_path) = std::env::var("MDM_CA_CERT") {
        println!("🔐 Using CA certificate from: {}", ca_cert_path);

        let _client = MdmClient::with_ca_cert("https://localhost:8080/mdm/config", &ca_cert_path)?;

        println!("✓ MDM client created with custom CA");
        println!("  URL: https://localhost:8080/mdm/config");
        println!("  CA:  {}\n", ca_cert_path);

        // Uncomment to actually fetch (requires running server):
        // match client.fetch_config().await {
        //     Ok(config) => {
        //         println!("✓ Configuration fetched successfully!");
        //         println!("  Policy enforcement: {}",
        // config.policy_enforcement);         println!("  Update
        // interval: {}s", config.update_interval_secs);     }
        //     Err(e) => {
        //         println!("✗ Failed to fetch configuration: {}", e);
        //     }
        // }
    } else {
        println!("ℹ️  MDM_CA_CERT not set, skipping CA certificate example");
        println!("   To use: export MDM_CA_CERT=/path/to/.mdm-certs/ca-cert.pem\n");
    }

    // Example 2: Testing mode (insecure - for development only)
    #[cfg(feature = "test-utils")]
    {
        println!("⚠️  Testing mode: Disabling certificate validation");
        println!("   (DO NOT use in production!)\n");

        let _client = MdmClient::with_insecure_tls("https://localhost:8080/mdm/config")?;

        println!("✓ MDM client created in testing mode");
        println!("  URL: https://localhost:8080/mdm/config");
        println!("  Cert validation: DISABLED (testing only)\n");
    }

    // Example 3: Working with local configuration
    println!("📝 Creating local MDM configuration");

    let local_config = MdmConfig::builder()
        .policy_enforcement(true)
        .remote_config_url("https://mdm.example.com/config")
        .update_interval_secs(3600)
        .allow_local_override(true)
        .build()?;

    println!("✓ Local configuration created");
    println!("  Policy enforcement: {}", local_config.policy_enforcement);
    println!("  Remote URL: {:?}", local_config.remote_config_url);
    println!("  Update interval: {}s", local_config.update_interval_secs);
    println!("  Allow override: {}\n", local_config.allow_local_override);

    // Example 4: Configuration validation
    println!("✅ Validating configuration");
    match local_config.validate() {
        Ok(()) => println!("✓ Configuration is valid\n"),
        Err(e) => println!("✗ Configuration validation failed: {}\n", e),
    }

    // Example 5: Certificate paths from environment
    println!("🔧 Certificate paths (if set):");
    if let Ok(ca_cert) = std::env::var("MDM_CA_CERT") {
        println!("  CA Certificate:     {}", ca_cert);
    }
    if let Ok(server_cert) = std::env::var("MDM_SERVER_CERT") {
        println!("  Server Certificate: {}", server_cert);
    }
    if let Ok(server_key) = std::env::var("MDM_SERVER_KEY") {
        println!("  Server Key:         {}", server_key);
    }

    println!("\n📚 Next steps:");
    println!("  1. Generate certificates: cd scripts/mdm && ./generate-test-certs.sh");
    println!("  2. Set environment variables (see example above)");
    println!("  3. Run a test MDM server on https://localhost:8080");
    println!("  4. Uncomment the fetch_config() call above to test");

    Ok(())
}
