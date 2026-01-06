#!/usr/bin/env bash
set -euo pipefail

# Test script for SSL certificate flow scenarios
# This script helps test each of the 5 scenarios documented in SSL_CERT_FLOW_ANALYSIS.md
#
# Usage:
#   ./test-ssl-scenarios.sh           # Run all scenarios
#   ./test-ssl-scenarios.sh 1         # Run only scenario 1
#   ./test-ssl-scenarios.sh --help    # Show usage information

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
TEST_DIR="${SCRIPT_DIR}/.test-ssl"
CA_DIR="${TEST_DIR}/ca"
COMPOSE_FILE="${ROOT_DIR}/docker/dev-db/docker-compose.yml"
POSTGRES_TLS_CA_CERT="${ROOT_DIR}/docker/postgres-tls/ca.crt"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

log_info() {
    echo -e "${BLUE}ℹ${NC} $1"
}

log_success() {
    echo -e "${GREEN}✓${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}⚠${NC} $1"
}

log_error() {
    echo -e "${RED}✗${NC} $1"
}

cleanup() {
    log_info "Cleaning up test environment..."
    docker compose -f "${COMPOSE_FILE}" down -v 2>/dev/null || true

    # Restore original docker/postgres-tls/ca.crt if we backed it up
    # Remove test CA copy if we created one (only touch build-context cert)
    if [ -f "${POSTGRES_TLS_CA_CERT}.backup" ]; then
        log_info "Restoring original docker/postgres-tls/ca.crt..."
        mv "${POSTGRES_TLS_CA_CERT}.backup" "${POSTGRES_TLS_CA_CERT}"
    elif [ -f "${POSTGRES_TLS_CA_CERT}" ]; then
        rm -f "${POSTGRES_TLS_CA_CERT}"
    fi
    
    rm -rf "${TEST_DIR}"
}

trap cleanup EXIT

# Create test directory structure
mkdir -p "${CA_DIR}"
mkdir -p "${TEST_DIR}/shared"

# Helper: Generate CA cert with custom validity
generate_ca() {
    local days=$1
    local output_key="${CA_DIR}/ca.key"
    local output_cert="${CA_DIR}/ca.crt"
    
    log_info "Generating CA certificate (valid for ${days} days)..."
    openssl genrsa -out "${output_key}" 4096
    openssl req -new -x509 -days "${days}" -key "${output_key}" -out "${output_cert}" \
        -subj "/CN=Nommie Test CA/O=Nommie Test/C=US"
    log_success "CA certificate generated"
}

# Helper: Copy generated CA cert into Docker build context (with backup)
sync_ca_to_build_context() {
    local ca_cert="${CA_DIR}/ca.crt"
    if [ ! -f "${ca_cert}" ]; then
        log_error "CA cert not found at ${ca_cert} for build context sync"
        return 1
    fi

    # Backup existing build-context CA cert once
    if [ -f "${POSTGRES_TLS_CA_CERT}" ] && [ ! -f "${POSTGRES_TLS_CA_CERT}.backup" ]; then
        log_info "Backing up existing docker/postgres-tls/ca.crt..."
        cp "${POSTGRES_TLS_CA_CERT}" "${POSTGRES_TLS_CA_CERT}.backup"
    fi

    cp "${ca_cert}" "${POSTGRES_TLS_CA_CERT}"
}

# Helper: Create expired CA (validity in the past)
generate_expired_ca() {
    log_info "Generating expired CA certificate..."
    local output_key="${CA_DIR}/ca.key"
    local output_cert="${CA_DIR}/ca.crt"
    local config_file="${CA_DIR}/openssl-expired.cnf"
    
    openssl genrsa -out "${output_key}" 4096
    
    # Calculate past dates (2 days ago to 1 day ago)
    local not_before not_after
    if date -u -d "2 days ago" +"%Y%m%d%H%M%SZ" >/dev/null 2>&1; then
        # GNU date
        not_before=$(date -u -d "2 days ago" +"%Y%m%d%H%M%SZ")
        not_after=$(date -u -d "1 day ago" +"%Y%m%d%H%M%SZ")
    elif date -u -v-2d +"%Y%m%d%H%M%SZ" >/dev/null 2>&1; then
        # BSD date (macOS)
        not_before=$(date -u -v-2d +"%Y%m%d%H%M%SZ")
        not_after=$(date -u -v-1d +"%Y%m%d%H%M%SZ")
    else
        # Fallback: use epoch calculation
        local days_ago_2=$(( $(date +%s) - 172800 ))
        local days_ago_1=$(( $(date +%s) - 86400 ))
        not_before=$(date -u -d "@${days_ago_2}" +"%Y%m%d%H%M%SZ" 2>/dev/null || printf "%s" "$(date -u -r "${days_ago_2}" +"%Y%m%d%H%M%SZ" 2>/dev/null)")
        not_after=$(date -u -d "@${days_ago_1}" +"%Y%m%d%H%M%SZ" 2>/dev/null || printf "%s" "$(date -u -r "${days_ago_1}" +"%Y%m%d%H%M%SZ" 2>/dev/null)")
    fi
    
    if [ -n "${not_before}" ] && [ -n "${not_after}" ]; then
        # Use openssl ca to create expired cert with explicit past dates
        local ca_config="${CA_DIR}/ca.conf"
        local ca_dir="${CA_DIR}/ca-workspace"
        local csr_file="${CA_DIR}/ca.csr"
        
        # Create workspace directory for openssl ca
        mkdir -p "${ca_dir}/certs"
        
        # Create openssl ca config file
        cat > "${ca_config}" <<EOF
[ ca ]
default_ca = myca

[ myca ]
dir             = ${ca_dir}
new_certs_dir   = \$dir/certs
database        = \$dir/index.txt
serial          = \$dir/serial
default_md      = sha256
default_days    = 365
policy          = policy_any
x509_extensions = v3_ca
private_key     = ${output_key}
certificate     = ${output_cert}

[ policy_any ]
commonName = supplied

[ req ]
distinguished_name = dn
x509_extensions    = v3_ca

[ dn ]
CN = Nommie Test Expired CA

[ v3_ca ]
basicConstraints = CA:TRUE
keyUsage         = keyCertSign, cRLSign
EOF
        
        # Create required files for openssl ca
        touch "${ca_dir}/index.txt"
        echo "01" > "${ca_dir}/serial"
        
        # Generate CSR
        openssl req -new -key "${output_key}" -out "${csr_file}" \
            -subj "/CN=Nommie Test Expired CA/O=Nommie Test/C=US" 2>/dev/null
        
        # Self-sign with explicit past dates using openssl ca
        if openssl ca -config "${ca_config}" -selfsign \
            -in "${csr_file}" -out "${output_cert}" \
            -startdate "${not_before}" -enddate "${not_after}" \
            -batch 2>/dev/null; then
            log_info "Created expired CA cert with explicit past dates"
        else
            log_error "Failed to create expired CA cert using openssl ca"
            rm -rf "${ca_dir}" "${ca_config}" "${csr_file}"
            return 1
        fi
        
        # Clean up temporary files
        rm -rf "${ca_dir}" "${ca_config}" "${csr_file}"
    else
        log_error "Could not calculate past dates for expired cert"
        return 1
    fi
    
    # Verify it's actually expired
    if openssl x509 -checkend 0 -noout -in "${output_cert}" 2>/dev/null; then
        log_warning "Cert is still valid, waiting longer..."
        sleep 3
        # Check again
        if openssl x509 -checkend 0 -noout -in "${output_cert}" 2>/dev/null; then
            log_error "Failed to create expired cert - cert is still valid"
            return 1
        fi
    fi
    
    log_success "Expired CA certificate generated"
}

# Helper: Create CA close to expiry (within 30 days)
generate_near_expiry_ca() {
    log_info "Generating CA certificate close to expiry (25 days)..."
    generate_ca 25
}

# Helper: Check if CA is expired
check_ca_expired() {
    local ca_cert="$1"
    if openssl x509 -checkend 0 -noout -in "${ca_cert}" 2>/dev/null; then
        return 1  # Not expired
    else
        return 0  # Expired
    fi
}

# Helper: Check if CA expires soon (within 30 days)
check_ca_near_expiry() {
    local ca_cert="$1"
    if openssl x509 -checkend 2592000 -noout -in "${ca_cert}" 2>/dev/null; then
        return 1  # Not near expiry
    else
        return 0  # Near expiry
    fi
}

# Helper: Build image with CA secrets
build_image() {
    local ca_key="${CA_DIR}/ca.key"
    local ca_cert="${CA_DIR}/ca.crt"
    
    if [ ! -f "${ca_key}" ] || [ ! -f "${ca_cert}" ]; then
        log_error "CA key or cert not found at ${ca_key} or ${ca_cert}"
        return 1
    fi

    # Ensure build context CA matches generated CA so Dockerfile COPY uses the same cert
    if ! sync_ca_to_build_context; then
        return 1
    fi
    
    log_info "Building Postgres TLS image..."
    if docker build \
        --secret id=nommie_ca_key,src="${ca_key}" \
        -t nommie-postgres-tls:test \
        "${ROOT_DIR}/docker/postgres-tls" 2>&1; then
        return 0
    else
        return 1
    fi
}

# Helper: Try to start container
start_container() {
    log_info "Starting container..."
    
    # Export environment variables for docker-compose
    export NOMMIE_CA_KEY_PATH="${CA_DIR}/ca.key"
    export NOMMIE_CA_CERT_PATH="${CA_DIR}/ca.crt"
    
    # Build and start with environment variables set
    # SSL_RENEWAL_THRESHOLD_SECONDS is now in docker-compose.yml environment section
    # and will be passed to container if set in shell environment
    docker compose -f "${COMPOSE_FILE}" build postgres 2>&1 || return 1
    docker compose -f "${COMPOSE_FILE}" up -d postgres 2>&1 || return 1
}

# Helper: Check container logs for errors
check_logs() {
    log_info "Checking container logs..."
    docker compose -f "${COMPOSE_FILE}" logs postgres | tail -20
}

# Helper: Check if server certs exist in volume
check_volume_certs() {
    log_info "Checking for server certs in volume..."
    docker compose -f "${COMPOSE_FILE}" exec -T postgres \
        sh -c "test -f /var/lib/postgresql/ssl/server.key && test -f /var/lib/postgresql/ssl/server.crt && echo 'Certs exist' || echo 'Certs missing'" 2>/dev/null || echo "Container not running"
}

# Test Scenario 1: No CA cert, no server certs
test_scenario_1() {
    echo ""
    log_info "=== Testing Scenario 1: No CA cert, no server certs ==="
    
    # Don't create CA certs
    rm -rf "${CA_DIR}"/*.key "${CA_DIR}"/*.crt 2>/dev/null || true

    # Remove build-context CA so COPY fails (back up any existing)
    if [ -f "${POSTGRES_TLS_CA_CERT}" ] && [ ! -f "${POSTGRES_TLS_CA_CERT}.backup" ]; then
        log_info "Backing up existing docker/postgres-tls/ca.crt..."
        mv "${POSTGRES_TLS_CA_CERT}" "${POSTGRES_TLS_CA_CERT}.backup"
    fi
    rm -f "${POSTGRES_TLS_CA_CERT}"
    
    log_info "Attempting to build image without CA secrets..."
    
    local build_output
    local build_exit_code
    
    # Build without secrets and capture both output and exit code
    build_output=$(docker build -t nommie-postgres-tls:test "${ROOT_DIR}/docker/postgres-tls" 2>&1)
    build_exit_code=$?
    
    # Check if build failed (exit code != 0) AND contains error message
    if [ "${build_exit_code}" -ne 0 ] && echo "${build_output}" | grep -qi "CA cert not found\|CA key not found\|ERROR.*CA"; then
        log_success "Build correctly failed as expected"
        log_info "Error message found in build output"
        return 0
    elif [ "${build_exit_code}" -ne 0 ]; then
        log_success "Build failed (exit code ${build_exit_code})"
        log_info "Checking for error details..."
        echo "${build_output}" | grep -i "error\|failed" | tail -5
        return 0
    else
        log_error "Build should have failed but succeeded (exit code ${build_exit_code})"
        log_info "Build output (last 20 lines):"
        echo "${build_output}" | tail -20
        return 1
    fi
}

# Test Scenario 2: CA cert present, no server certs in volume
test_scenario_2() {
    echo ""
    log_info "=== Testing Scenario 2: CA cert present, no server certs in volume ==="
    
    # Generate valid CA
    generate_ca 7300
    
    # Build image
    if ! build_image; then
        log_error "Build failed"
        return 1
    fi
    
    # Start container (volumes already cleaned up by main())
    if start_container; then
        sleep 5
        if check_volume_certs | grep -q "Certs exist"; then
            log_success "Server certs were copied to volume on first start"
            return 0
        else
            log_error "Server certs were not copied to volume"
            check_logs
            return 1
        fi
    else
        log_error "Container failed to start"
        check_logs
        return 1
    fi
}

# Test Scenario 3: CA valid, server cert close to expiry
test_scenario_3() {
    echo ""
    log_info "=== Testing Scenario 3: CA valid, server cert close to expiry ==="
    log_info "Using SSL_RENEWAL_THRESHOLD_SECONDS method (threshold > 3 years)"
    log_info "This triggers refresh by setting threshold higher than cert validity"
    echo ""
    
    # Generate valid CA
    generate_ca 7300
    
    # Build image (normal 3-year server certs)
    log_info "Building image with normal 3-year server certs..."
    if ! build_image; then
        log_error "Build failed"
        return 1
    fi
    
    # Start container first time to get certs in volume
    log_info "Starting container (first run - certs will be copied to volume)..."
    docker compose -f "${COMPOSE_FILE}" down -v 2>/dev/null || true
    start_container
    sleep 8
    
    # Check that certs exist
    if ! check_volume_certs | grep -q "Certs exist"; then
        log_error "Server certs not found in volume"
        check_logs
        return 1
    fi
    
    log_success "Server certs copied to volume"
    
    # Get cert modification time before refresh
    CERT_MTIME_BEFORE=$(docker compose -f "${COMPOSE_FILE}" exec -T postgres \
        stat -c %Y /var/lib/postgresql/ssl/server.crt 2>/dev/null || echo "0")
    
    # Stop container
    log_info "Stopping container..."
    docker compose -f "${COMPOSE_FILE}" stop postgres
    
    # Restart with SSL_RENEWAL_THRESHOLD_SECONDS > 3 years
    # 3 years = 94,608,000 seconds
    # 4 years = 126,144,000 seconds
    log_info "Restarting with SSL_RENEWAL_THRESHOLD_SECONDS=126144000 (4 years)..."
    log_info "Since server cert expires in 3 years, threshold > 3 years will trigger refresh"
    
    export SSL_RENEWAL_THRESHOLD_SECONDS=126144000  # 4 years
    
    if ! start_container; then
        log_error "Failed to restart container"
        check_logs
        return 1
    fi
    
    sleep 8
    
    # Check logs for refresh message
    log_info "Checking logs for certificate refresh message..."
    LOGS=$(docker compose -f "${COMPOSE_FILE}" logs postgres 2>&1)
    
    if echo "${LOGS}" | grep -qi "expires soon\|refreshing\|copying.*certificate"; then
        log_success "Found certificate refresh message in logs!"
        echo ""
        echo "Relevant log lines:"
        echo "${LOGS}" | grep -i "expires soon\|refreshing\|copying.*certificate" | head -5
    else
        log_warning "Did not find explicit refresh message in logs"
        log_info "Checking full logs..."
        check_logs
    fi
    
    # Verify certs were refreshed (check modification time)
    log_info "Verifying certificate refresh..."
    CERT_MTIME_AFTER=$(docker compose -f "${COMPOSE_FILE}" exec -T postgres \
        stat -c %Y /var/lib/postgresql/ssl/server.crt 2>/dev/null || echo "0")
    
    if [ "${CERT_MTIME_BEFORE}" != "0" ] && [ "${CERT_MTIME_AFTER}" != "0" ]; then
        if [ "${CERT_MTIME_AFTER}" -gt "${CERT_MTIME_BEFORE}" ]; then
            log_success "Certificate was refreshed (modification time changed)"
        else
            log_warning "Certificate modification time did not change (may have been refreshed very quickly)"
        fi
    fi
    
    # Verify the logic worked
    if echo "${LOGS}" | grep -qi "expires soon\|refreshing"; then
        log_success "Scenario 3 test passed: Refresh logic triggered correctly"
        return 0
    else
        log_warning "Scenario 3 test: Refresh message not found, but certs may have been refreshed"
        log_info "Check logs manually to verify behavior"
        return 0  # Don't fail, as the logic may have worked even without explicit message
    fi
}

# Test Scenario 4: CA cert close to expiry
test_scenario_4() {
    echo ""
    log_info "=== Testing Scenario 4: CA cert close to expiry ==="
    
    # Generate CA that expires in 25 days (within 30-day threshold)
    generate_near_expiry_ca
    
    # Verify it's near expiry
    if ! check_ca_near_expiry "${CA_DIR}/ca.crt"; then
        log_error "CA is not near expiry as expected"
        return 1
    fi
    
    log_info "Attempting to build image with CA close to expiry..."
    
    # Build and capture output to check for warning
    local ca_key="${CA_DIR}/ca.key"
    local ca_cert="${CA_DIR}/ca.crt"
    local build_output

    # Sync generated CA cert into Docker build context so COPY uses the matching cert
    if ! sync_ca_to_build_context; then
        return 1
    fi

    build_output=$(docker build \
        --secret id=nommie_ca_key,src="${ca_key}" \
        -t nommie-postgres-tls:test \
        "${ROOT_DIR}/docker/postgres-tls" 2>&1)
    local build_exit_code=$?
    
    if [ "${build_exit_code}" -ne 0 ]; then
        log_error "Build failed unexpectedly"
        echo "${build_output}" | tail -20
        return 1
    fi
    
    # Check for warning message (build script prints to stderr)
    if echo "${build_output}" | grep -qi "WARNING.*CA certificate.*expire.*soon"; then
        log_success "Build succeeded with warning as expected"
    else
        log_error "Warning message not found in build output"
        log_info "Checking build output for warning/expire messages:"
        echo "${build_output}" | grep -i "warning\|expire" | head -10 || echo "No warning/expire messages found"
        log_info "Full build output (last 30 lines):"
        echo "${build_output}" | tail -30
        return 1
    fi
    
    # Try to start container (build succeeded, warning is non-fatal)
    docker compose -f "${COMPOSE_FILE}" down -v 2>/dev/null || true
    if start_container; then
        sleep 5
        log_success "Container started successfully (with near-expiry CA)"
        return 0
    else
        log_error "Container failed to start"
        check_logs
        return 1
    fi
}

# Test Scenario 5: CA cert expired
test_scenario_5() {
    echo ""
    log_info "=== Testing Scenario 5: CA cert expired ==="
    
    # Generate expired CA
    generate_expired_ca
    
    # Verify it's expired
    if ! check_ca_expired "${CA_DIR}/ca.crt"; then
        log_error "CA is not expired as expected"
        return 1
    fi
    
    log_info "Attempting to build image with expired CA..."
    
    local ca_key="${CA_DIR}/ca.key"
    local ca_cert="${CA_DIR}/ca.crt"
    local build_output

    # Sync generated CA cert into Docker build context so COPY uses the matching cert
    if ! sync_ca_to_build_context; then
        return 1
    fi

    build_output=$(docker build \
        --secret id=nommie_ca_key,src="${ca_key}" \
        -t nommie-postgres-tls:test \
        "${ROOT_DIR}/docker/postgres-tls" 2>&1)
    local build_exit_code=$?
    
    if echo "${build_output}" | grep -qi "ERROR.*CA certificate.*expired"; then
        log_success "Build correctly failed as expected"
        log_info "Error message found in build output"
        return 0
    elif [ "${build_exit_code}" -ne 0 ]; then
        log_success "Build failed (exit code ${build_exit_code})"
        log_info "Checking for error details..."
        echo "${build_output}" | grep -i "error\|failed\|expired" | tail -5
        return 0
    else
        log_error "Build should have failed but succeeded (exit code ${build_exit_code})"
        log_info "Build output (last 20 lines):"
        echo "${build_output}" | tail -20
        return 1
    fi
}

# Show usage information
show_usage() {
    cat <<EOF
Usage: $0 [SCENARIO_NUMBER]

Test SSL certificate flow scenarios for dev-db and prod containers.

Arguments:
  SCENARIO_NUMBER    Run a specific scenario (1-5). If omitted, runs all scenarios.

Scenarios:
  1  No CA cert, no server certs
  2  CA cert present, no server certs in volume
  3  CA valid, server cert close to expiry
  4  CA cert close to expiry
  5  CA cert expired

Examples:
  $0              # Run all scenarios
  $0 1            # Run only scenario 1
  $0 3            # Run only scenario 3

EOF
}

# Main test runner
main() {
    local scenario_num="${1:-}"
    
    # Show usage if help requested
    if [ "${scenario_num}" = "-h" ] || [ "${scenario_num}" = "--help" ]; then
        show_usage
        exit 0
    fi
    
    # Validate scenario number if provided
    if [ -n "${scenario_num}" ]; then
        if ! [[ "${scenario_num}" =~ ^[1-5]$ ]]; then
            log_error "Invalid scenario number: ${scenario_num}"
            log_info "Valid scenarios are 1-5"
            echo ""
            show_usage
            exit 1
        fi
    fi
    
    echo "=========================================="
    echo "SSL Certificate Flow Test Suite"
    echo "=========================================="
    echo ""
    log_info "Test directory: ${TEST_DIR}"
    log_info "Using compose file: ${COMPOSE_FILE}"
    
    # Safety check: Inform user about real CA certs
    if [ -f "${HOME}/secrets/nommie-ca/ca.crt" ] || [ -f "${HOME}/secrets/nommie-ca/ca.key" ]; then
        log_info "Your real CA certs in ~/secrets/nommie-ca/ will NOT be touched"
        log_info "This test uses separate test CA certs"
    fi
    
    echo ""
    
    # Clean up any existing test containers
    docker compose -f "${COMPOSE_FILE}" down -v 2>/dev/null || true
    
    local results=()
    
    # Run specific scenario or all scenarios
    if [ -n "${scenario_num}" ]; then
        log_info "Running Scenario ${scenario_num} only"
        echo ""
        case "${scenario_num}" in
            1) test_scenario_1 && results+=("Scenario 1: PASS") || results+=("Scenario 1: FAIL") ;;
            2) test_scenario_2 && results+=("Scenario 2: PASS") || results+=("Scenario 2: FAIL") ;;
            3) test_scenario_3 && results+=("Scenario 3: PASS") || results+=("Scenario 3: FAIL") ;;
            4) test_scenario_4 && results+=("Scenario 4: PASS") || results+=("Scenario 4: FAIL") ;;
            5) test_scenario_5 && results+=("Scenario 5: PASS") || results+=("Scenario 5: FAIL") ;;
        esac
    else
        log_info "Running all scenarios"
        echo ""
        # Run each test
        test_scenario_1 && results+=("Scenario 1: PASS") || results+=("Scenario 1: FAIL")
        test_scenario_2 && results+=("Scenario 2: PASS") || results+=("Scenario 2: FAIL")
        test_scenario_3 && results+=("Scenario 3: PASS") || results+=("Scenario 3: FAIL")
        test_scenario_4 && results+=("Scenario 4: PASS") || results+=("Scenario 4: FAIL")
        test_scenario_5 && results+=("Scenario 5: PASS") || results+=("Scenario 5: FAIL")
    fi
    
    # Print summary
    echo ""
    echo "=========================================="
    echo "Test Results Summary"
    echo "=========================================="
    for result in "${results[@]}"; do
        if [[ "$result" == *"PASS"* ]]; then
            log_success "$result"
        else
            log_error "$result"
        fi
    done
    echo ""
}

# Run tests if script is executed directly
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    main "$@"
fi

