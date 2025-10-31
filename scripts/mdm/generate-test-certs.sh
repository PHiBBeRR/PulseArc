#!/bin/bash
# Generate self-signed certificates for MDM testing
# Safe for CI/CD and local development

set -euo pipefail

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CERTS_DIR="${CERTS_DIR:-${SCRIPT_DIR}/../../.mdm-certs}"
DAYS_VALID="${DAYS_VALID:-365}"
KEY_SIZE=4096

# Certificate details
COUNTRY="${CERT_COUNTRY:-US}"
STATE="${CERT_STATE:-California}"
CITY="${CERT_CITY:-San Francisco}"
ORG="${CERT_ORG:-PulseArc}"
ORG_UNIT="${CERT_ORG_UNIT:-MDM Testing}"
COMMON_NAME="${CERT_CN:-localhost}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check if openssl is available
if ! command -v openssl &> /dev/null; then
    log_error "openssl is not installed. Please install it first:"
    echo "  macOS: brew install openssl"
    echo "  Ubuntu: sudo apt-get install openssl"
    exit 1
fi

# Create certificates directory
mkdir -p "$CERTS_DIR"
cd "$CERTS_DIR"

log_info "Generating MDM test certificates in: $CERTS_DIR"
log_info "Validity: $DAYS_VALID days"
log_info "Common Name: $COMMON_NAME"

# Generate CA private key
log_info "Generating CA private key..."
openssl genrsa -out ca-key.pem $KEY_SIZE 2>/dev/null

# Generate CA certificate
log_info "Generating CA certificate..."
openssl req -new -x509 -days $DAYS_VALID -key ca-key.pem -out ca-cert.pem \
    -subj "/C=$COUNTRY/ST=$STATE/L=$CITY/O=$ORG/OU=$ORG_UNIT CA/CN=$ORG Root CA" \
    2>/dev/null

# Generate server private key
log_info "Generating server private key..."
openssl genrsa -out server-key.pem $KEY_SIZE 2>/dev/null

# Create server certificate signing request
log_info "Creating server certificate signing request..."
openssl req -new -key server-key.pem -out server.csr \
    -subj "/C=$COUNTRY/ST=$STATE/L=$CITY/O=$ORG/OU=$ORG_UNIT/CN=$COMMON_NAME" \
    2>/dev/null

# Create OpenSSL extension config for SAN (Subject Alternative Names)
cat > server-extensions.cnf <<EOF
[v3_req]
basicConstraints = CA:FALSE
keyUsage = nonRepudiation, digitalSignature, keyEncipherment
extendedKeyUsage = serverAuth
subjectAltName = @alt_names

[alt_names]
DNS.1 = localhost
DNS.2 = *.localhost
DNS.3 = 127.0.0.1
IP.1 = 127.0.0.1
IP.2 = ::1
EOF

# Sign server certificate with CA
log_info "Signing server certificate with CA..."
openssl x509 -req -in server.csr -CA ca-cert.pem -CAkey ca-key.pem \
    -CAcreateserial -out server-cert.pem -days $DAYS_VALID \
    -extensions v3_req -extfile server-extensions.cnf \
    2>/dev/null

# Generate client private key (for mutual TLS if needed)
log_info "Generating client private key..."
openssl genrsa -out client-key.pem $KEY_SIZE 2>/dev/null

# Create client certificate signing request
log_info "Creating client certificate signing request..."
openssl req -new -key client-key.pem -out client.csr \
    -subj "/C=$COUNTRY/ST=$STATE/L=$CITY/O=$ORG/OU=$ORG_UNIT/CN=MDM Test Client" \
    2>/dev/null

# Sign client certificate with CA
log_info "Signing client certificate with CA..."
openssl x509 -req -in client.csr -CA ca-cert.pem -CAkey ca-key.pem \
    -CAcreateserial -out client-cert.pem -days $DAYS_VALID \
    2>/dev/null

# Create combined PEM files (for easier usage)
log_info "Creating combined PEM files..."
cat server-cert.pem ca-cert.pem > server-fullchain.pem
cat client-cert.pem ca-cert.pem > client-fullchain.pem

# Set restrictive permissions on private keys
chmod 600 *-key.pem

# Create certificate bundle in various formats
log_info "Creating certificate bundles..."

# PKCS#12 format (for importing into keychain/browsers)
openssl pkcs12 -export -out server.p12 \
    -inkey server-key.pem -in server-cert.pem -certfile ca-cert.pem \
    -passout pass:pulsearc-mdm-test 2>/dev/null

openssl pkcs12 -export -out client.p12 \
    -inkey client-key.pem -in client-cert.pem -certfile ca-cert.pem \
    -passout pass:pulsearc-mdm-test 2>/dev/null

# Create .gitignore to prevent committing private keys
cat > .gitignore <<EOF
# Private keys - DO NOT COMMIT
*-key.pem
*.p12

# Certificate signing requests
*.csr
*.srl

# Extension configs
*-extensions.cnf
EOF

# Cleanup temporary files
rm -f server.csr client.csr server-extensions.cnf ca-cert.srl

# Display certificate information
log_info "Certificate generation complete!"
echo ""
echo "Generated files:"
echo "  📁 Directory: $CERTS_DIR"
echo ""
echo "  🔐 CA Certificates:"
echo "     - ca-cert.pem (Root CA certificate - trust this in your system)"
echo "     - ca-key.pem  (CA private key - keep secret)"
echo ""
echo "  🖥️  Server Certificates:"
echo "     - server-cert.pem (Server certificate)"
echo "     - server-key.pem  (Server private key)"
echo "     - server-fullchain.pem (Server cert + CA chain)"
echo "     - server.p12 (PKCS#12 bundle, password: pulsearc-mdm-test)"
echo ""
echo "  👤 Client Certificates (for mutual TLS):"
echo "     - client-cert.pem (Client certificate)"
echo "     - client-key.pem  (Client private key)"
echo "     - client-fullchain.pem (Client cert + CA chain)"
echo "     - client.p12 (PKCS#12 bundle, password: pulsearc-mdm-test)"
echo ""

# Verify certificates
log_info "Verifying certificates..."
if openssl verify -CAfile ca-cert.pem server-cert.pem &>/dev/null; then
    echo -e "  ${GREEN}✓${NC} Server certificate: Valid"
else
    echo -e "  ${RED}✗${NC} Server certificate: Invalid"
fi

if openssl verify -CAfile ca-cert.pem client-cert.pem &>/dev/null; then
    echo -e "  ${GREEN}✓${NC} Client certificate: Valid"
else
    echo -e "  ${RED}✗${NC} Client certificate: Invalid"
fi

echo ""
log_info "Next steps:"
echo ""
echo "  1. Trust the CA certificate (macOS):"
echo "     sudo security add-trusted-cert -d -r trustRoot -k /Library/Keychains/System.keychain $CERTS_DIR/ca-cert.pem"
echo ""
echo "  2. Use in Rust/reqwest (disable verification for testing):"
echo "     reqwest::Client::builder()"
echo "         .danger_accept_invalid_certs(true)  // For testing only!"
echo "         .build()?;"
echo ""
echo "  3. Use with curl:"
echo "     curl --cacert $CERTS_DIR/ca-cert.pem https://localhost:8080"
echo ""
echo "  4. Environment variables for MDM testing:"
echo "     export MDM_CA_CERT=$CERTS_DIR/ca-cert.pem"
echo "     export MDM_SERVER_CERT=$CERTS_DIR/server-cert.pem"
echo "     export MDM_SERVER_KEY=$CERTS_DIR/server-key.pem"
echo ""

log_warn "These are TEST certificates - DO NOT use in production!"
log_warn "Private keys are stored in: $CERTS_DIR"
log_warn "Keep these files secure and never commit them to git!"
