# MDM Test Certificate Scripts

This directory contains scripts for generating self-signed certificates for MDM testing.

## Quick Start

```bash
# Generate certificates
./generate-test-certs.sh

# Certificates will be created in ../../.mdm-certs/
```

## Files Generated

After running the script, you'll have:

```
.mdm-certs/
├── ca-cert.pem          # Root CA certificate (trust this)
├── ca-key.pem           # CA private key (keep secret)
├── server-cert.pem      # Server certificate
├── server-key.pem       # Server private key
├── server-fullchain.pem # Server cert + CA chain
├── server.p12           # PKCS#12 bundle (password: pulsearc-mdm-test)
├── client-cert.pem      # Client certificate (for mutual TLS)
├── client-key.pem       # Client private key
├── client-fullchain.pem # Client cert + CA chain
├── client.p12           # PKCS#12 bundle (password: pulsearc-mdm-test)
└── .gitignore           # Prevents committing private keys
```

## Usage

### 1. Generate Certificates

```bash
cd scripts/mdm
./generate-test-certs.sh
```

**Custom options:**
```bash
# Custom certificate directory
CERTS_DIR=/path/to/certs ./generate-test-certs.sh

# Custom validity period (default: 365 days)
DAYS_VALID=730 ./generate-test-certs.sh

# Custom common name (default: localhost)
CERT_CN=my-server.local ./generate-test-certs.sh

# Combine options
CERTS_DIR=./my-certs DAYS_VALID=90 CERT_CN=test.local ./generate-test-certs.sh
```

### 2. Trust the CA Certificate (macOS)

For system-wide trust:
```bash
sudo security add-trusted-cert -d -r trustRoot \
  -k /Library/Keychains/System.keychain \
  ../../.mdm-certs/ca-cert.pem
```

For user-only trust:
```bash
security add-trusted-cert -d -r trustRoot \
  -k ~/Library/Keychains/login.keychain-db \
  ../../.mdm-certs/ca-cert.pem
```

### 3. Use in Rust Code

#### Option A: Disable Certificate Validation (Testing Only)

```rust
use reqwest;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)  // ⚠️ TEST ONLY!
        .build()?;

    let response = client
        .get("https://localhost:8080/mdm/config")
        .send()
        .await?;

    println!("Response: {:?}", response);
    Ok(())
}
```

#### Option B: Use Custom CA Certificate (Production-Like)

```rust
use reqwest;
use std::fs;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load CA certificate
    let ca_cert = fs::read("../../.mdm-certs/ca-cert.pem")?;
    let ca_cert = reqwest::Certificate::from_pem(&ca_cert)?;

    let client = reqwest::Client::builder()
        .add_root_certificate(ca_cert)
        .build()?;

    let response = client
        .get("https://localhost:8080/mdm/config")
        .send()
        .await?;

    println!("Response: {:?}", response);
    Ok(())
}
```

#### Option C: Mutual TLS (Client Certificate)

```rust
use reqwest;
use std::fs;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load client certificate and key
    let client_cert = fs::read("../../.mdm-certs/client-fullchain.pem")?;
    let client_key = fs::read("../../.mdm-certs/client-key.pem")?;
    let identity = reqwest::Identity::from_pem(&[&client_cert[..], &client_key[..]].concat())?;

    // Load CA certificate
    let ca_cert = fs::read("../../.mdm-certs/ca-cert.pem")?;
    let ca_cert = reqwest::Certificate::from_pem(&ca_cert)?;

    let client = reqwest::Client::builder()
        .identity(identity)
        .add_root_certificate(ca_cert)
        .build()?;

    let response = client
        .get("https://localhost:8080/mdm/config")
        .send()
        .await?;

    println!("Response: {:?}", response);
    Ok(())
}
```

### 4. Use with curl

```bash
# Use CA certificate
curl --cacert ../../.mdm-certs/ca-cert.pem https://localhost:8080/mdm/config

# Mutual TLS
curl --cacert ../../.mdm-certs/ca-cert.pem \
     --cert ../../.mdm-certs/client-cert.pem \
     --key ../../.mdm-certs/client-key.pem \
     https://localhost:8080/mdm/config
```

### 5. CI/CD Integration

#### GitHub Actions

```yaml
- name: Generate MDM test certificates
  run: |
    cd scripts/mdm
    ./generate-test-certs.sh

- name: Run MDM tests
  run: cargo test --features mdm-integration
  env:
    MDM_CA_CERT: ${{ github.workspace }}/.mdm-certs/ca-cert.pem
    MDM_SERVER_CERT: ${{ github.workspace }}/.mdm-certs/server-cert.pem
    MDM_SERVER_KEY: ${{ github.workspace }}/.mdm-certs/server-key.pem
```

#### Manual CI Runner

```bash
# One-time setup on CI runner
cd /path/to/PulseArc/scripts/mdm
./generate-test-certs.sh

# Export environment variables for tests
export MDM_CA_CERT=$(pwd)/../../.mdm-certs/ca-cert.pem
export MDM_SERVER_CERT=$(pwd)/../../.mdm-certs/server-cert.pem
export MDM_SERVER_KEY=$(pwd)/../../.mdm-certs/server-key.pem

# Run tests
cargo test --features mdm-integration
```

## Environment Variables

The script respects these environment variables:

| Variable | Description | Default |
|----------|-------------|---------|
| `CERTS_DIR` | Directory to store certificates | `../../.mdm-certs` |
| `DAYS_VALID` | Certificate validity period | `365` |
| `CERT_COUNTRY` | Country code | `US` |
| `CERT_STATE` | State/Province | `California` |
| `CERT_CITY` | City | `San Francisco` |
| `CERT_ORG` | Organization | `PulseArc` |
| `CERT_ORG_UNIT` | Organizational Unit | `MDM Testing` |
| `CERT_CN` | Common Name | `localhost` |

## Security Notes

### ⚠️ FOR TESTING ONLY

These are **self-signed test certificates**. Do NOT use in production!

**What's Safe:**
- ✅ Local development
- ✅ CI/CD testing
- ✅ Internal testing environments
- ✅ Unit/integration tests

**What's NOT Safe:**
- ❌ Production deployments
- ❌ Public-facing services
- ❌ Services handling real user data
- ❌ Compliance-required environments

### Private Key Security

- Private keys (`.pem` files with `-key` in the name) are **sensitive**
- The script sets restrictive permissions (`chmod 600`)
- A `.gitignore` file is created to prevent accidental commits
- Never share private keys or commit them to version control

### When to Regenerate

Regenerate certificates when:
- They expire (check with `openssl x509 -in cert.pem -noout -dates`)
- Private keys are compromised
- Testing different SAN configurations
- Updating organizational details

## Troubleshooting

### "openssl: command not found"

**macOS:**
```bash
brew install openssl
```

**Ubuntu/Debian:**
```bash
sudo apt-get install openssl
```

### Certificate Not Trusted by Browser/curl

1. **Trust the CA certificate** (see "Trust the CA Certificate" section above)
2. **Or disable verification** in test code (not recommended for production-like testing)

### "Certificate verify failed"

This usually means:
1. The CA certificate isn't trusted by your system
2. The server certificate was signed by a different CA
3. The certificate doesn't match the hostname

**Solution:**
- Make sure you're using the CA certificate generated by this script
- Regenerate all certificates if you're not sure which CA was used
- Check that `CERT_CN` matches your server hostname

### Mutual TLS Not Working

Ensure:
1. Server is configured to require client certificates
2. Client certificate is signed by the same CA
3. Client is sending both certificate and key
4. Certificate formats are correct (PEM, not DER)

## Advanced Usage

### Generate Certificates for Multiple Domains

```bash
# Edit the script and add more DNS/IP entries to alt_names
cat > server-extensions.cnf <<EOF
[v3_req]
basicConstraints = CA:FALSE
keyUsage = nonRepudiation, digitalSignature, keyEncipherment
extendedKeyUsage = serverAuth
subjectAltName = @alt_names

[alt_names]
DNS.1 = localhost
DNS.2 = *.localhost
DNS.3 = my-server.local
DNS.4 = *.my-server.local
IP.1 = 127.0.0.1
IP.2 = ::1
IP.3 = 192.168.1.100
EOF
```

### Inspect Certificate Details

```bash
cd ../../.mdm-certs

# View certificate info
openssl x509 -in server-cert.pem -text -noout

# Check expiration date
openssl x509 -in server-cert.pem -noout -dates

# Verify certificate chain
openssl verify -CAfile ca-cert.pem server-cert.pem

# Check if certificate matches private key
openssl x509 -noout -modulus -in server-cert.pem | openssl md5
openssl rsa -noout -modulus -in server-key.pem | openssl md5
```

### Convert to Other Formats

```bash
cd ../../.mdm-certs

# PEM to DER
openssl x509 -in server-cert.pem -outform DER -out server-cert.der

# Extract from PKCS#12
openssl pkcs12 -in server.p12 -out server.pem -nodes -passin pass:pulsearc-mdm-test

# Create Java KeyStore
keytool -importkeystore -srckeystore server.p12 -srcstoretype PKCS12 \
  -destkeystore server.jks -deststoretype JKS \
  -srcstorepass pulsearc-mdm-test -deststorepass pulsearc-mdm-test
```

## Related Documentation

- [MDM Extraction Guide](../../docs/issues/MDM_EXTRACTION_GUIDE.md)
- [Reqwest TLS Configuration](https://docs.rs/reqwest/latest/reqwest/struct.ClientBuilder.html#tls)
- [OpenSSL Documentation](https://www.openssl.org/docs/)

## Questions?

- **"Do I need this for local development?"** - Only if testing MDM HTTPS communication
- **"Should I commit the certificates?"** - NO! They contain private keys
- **"Can I use these in production?"** - NO! Get proper certificates from a CA
- **"How do I rotate certificates?"** - Delete `.mdm-certs/` and run the script again
- **"What if my CI runner already has certificates?"** - The script is idempotent and won't overwrite without confirmation
