#!/usr/bin/env bash
# Performance profiling script for Nommie
# This script runs Lighthouse audits and saves reports
#
# Usage:
#   ./profile-performance.sh [--url URL] [--output-dir DIR] [--desktop|--mobile] [--cookie-file FILE] [-- ...lighthouse-args]
#   ./profile-performance.sh --url http://localhost:3000 --desktop
#   ./profile-performance.sh --mobile --output-dir ./reports
#   ./profile-performance.sh --cookie-file cookies.txt --url http://localhost:3000/game/123
#   ./profile-performance.sh -- --throttling-method=simulate
#
# Cookie file format (for authenticated pages):
#   Simple format: One cookie per line as name=value
#     Example:
#       next-auth.session-token=abc123...
#       backend-jwt=xyz789...
#   
#   Or Netscape format (exported from browser extensions like "Cookie-Editor")
#
#   To extract cookies from browser:
#   1. Open DevTools (F12) → Application → Cookies
#   2. Copy cookie name=value pairs, one per line
#   3. Save to a file (e.g., cookies.txt)
#   4. Use: --cookie-file cookies.txt

set -euo pipefail

# Colors for output
readonly RED='\033[0;31m'
readonly GREEN='\033[0;32m'
readonly YELLOW='\033[1;33m'
readonly BLUE='\033[0;34m'
readonly NC='\033[0m' # No Color

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

# Default values
URL="http://localhost:3000"
OUTPUT_DIR="./performance-reports"
DESKTOP="true"
COOKIE_FILE=""
LIGHTHOUSE_ARGS=()

# Parse named arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --url)
            URL="$2"
            shift 2
            ;;
        --output-dir|--output)
            OUTPUT_DIR="$2"
            shift 2
            ;;
        --desktop)
            DESKTOP="true"
            shift
            ;;
        --mobile)
            DESKTOP="false"
            shift
            ;;
        --cookie-file)
            COOKIE_FILE="$2"
            shift 2
            ;;
        --)
            # Everything after -- goes to lighthouse
            shift
            LIGHTHOUSE_ARGS=("$@")
            break
            ;;
        *)
            # Unknown argument, assume it's for lighthouse
            LIGHTHOUSE_ARGS+=("$1")
            shift
            ;;
    esac
done

readonly URL
readonly OUTPUT_DIR
readonly DESKTOP

# Function to parse cookie file and format as Cookie header
parse_cookie_file() {
    local file="$1"
    local cookie_string=""
    
    if [ ! -f "$file" ]; then
        log_error "Cookie file not found: $file"
        exit 1
    fi
    
    # Check if it's Netscape format (starts with # Netscape)
    if head -1 "$file" | grep -q "^# Netscape"; then
        # Parse Netscape cookie format
        # Format: domain, flag, path, secure, expiration, name, value
        while IFS=$'\t' read -r domain flag path secure expiration name value; do
            # Skip comments and empty lines
            [[ "$domain" =~ ^# ]] && continue
            [[ -z "$domain" ]] && continue
            
            # Only include cookies for the current domain
            # Extract domain from URL
            url_domain=$(echo "$URL" | sed -E 's|https?://([^/]+).*|\1|')
            
            # Match domain (handle subdomains)
            if [[ "$domain" == "$url_domain" ]] || [[ "$domain" == ".$url_domain" ]] || [[ "$url_domain" == *".$domain" ]]; then
                if [ -n "$cookie_string" ]; then
                    cookie_string="$cookie_string; $name=$value"
                else
                    cookie_string="$name=$value"
                fi
            fi
        done < "$file"
    else
        # Simple format: one cookie per line as name=value
        while IFS= read -r line; do
            # Skip comments and empty lines
            [[ "$line" =~ ^# ]] && continue
            [[ -z "$line" ]] && continue
            
            # Remove leading/trailing whitespace
            line=$(echo "$line" | xargs)
            
            # Check if line contains = (cookie format)
            if [[ "$line" == *"="* ]]; then
                if [ -n "$cookie_string" ]; then
                    cookie_string="$cookie_string; $line"
                else
                    cookie_string="$line"
                fi
            fi
        done < "$file"
    fi
    
    echo "$cookie_string"
}

log_info "Nommie Performance Profiling"
echo "URL: $URL"
echo "Output directory: $OUTPUT_DIR"
echo "Device: $([ "$DESKTOP" = "true" ] && echo "Desktop" || echo "Mobile")"
if [ -n "$COOKIE_FILE" ]; then
    echo "Cookie file: $COOKIE_FILE"
fi
echo ""

# Check required tools
if ! command -v curl >/dev/null 2>&1; then
    log_error "curl is required but not found. Please install curl."
    exit 1
fi

# Check if URL is reachable
if ! curl -s --head --fail "$URL" > /dev/null 2>&1; then
    log_error "Cannot reach $URL"
    echo ""
    echo "Make sure your services are running:"
    echo "  • Frontend: pnpm fe:dev (dev) or pnpm start (prod)"
    echo "  • Backend:  pnpm be:dev (dev) or pnpm be:start (prod)"
    echo ""
    echo "Default frontend URL: http://localhost:3000"
    exit 1
fi

# Check if lighthouse is installed
if ! command -v lighthouse >/dev/null 2>&1; then
    log_warning "Lighthouse not found. Installing..."
    if ! npm install -g lighthouse; then
        log_error "Failed to install Lighthouse. Please install manually: npm install -g lighthouse"
        exit 1
    fi
fi

# Check if Chrome/Chromium is installed and get its path
CHROME_PATH=""
for chrome_cmd in google-chrome chromium-browser chromium chrome; do
    if command -v "$chrome_cmd" >/dev/null 2>&1; then
        CHROME_PATH=$(command -v "$chrome_cmd")
        break
    fi
done

if [ -z "$CHROME_PATH" ]; then
    log_error "Chrome or Chromium is required but not found."
    echo ""
    echo "Lighthouse needs Chrome or Chromium to run audits."
    echo ""
    echo "Installation options:"
    echo ""
    
    # Detect OS and provide appropriate instructions
    if [ -f /etc/os-release ]; then
        . /etc/os-release
        case "$ID" in
            ubuntu|debian)
                echo "  Ubuntu/Debian:"
                echo "    sudo apt-get update"
                echo "    sudo apt-get install -y chromium-browser"
                ;;
            fedora|rhel|centos)
                echo "  Fedora/RHEL/CentOS:"
                echo "    sudo dnf install -y chromium"
                ;;
            arch|manjaro)
                echo "  Arch/Manjaro:"
                echo "    sudo pacman -S chromium"
                ;;
            *)
                echo "  Install Chromium or Google Chrome for your distribution"
                ;;
        esac
    else
        echo "  Install Chromium or Google Chrome for your system"
    fi
    
    echo ""
    echo "Alternatively, install Google Chrome from: https://www.google.com/chrome/"
    exit 1
fi

# Create output directory
mkdir -p "$OUTPUT_DIR"

# Run Lighthouse audit
log_info "Running Lighthouse audit..."

# Export CHROME_PATH so Lighthouse can find Chrome/Chromium
export CHROME_PATH

# Parse cookies if cookie file is provided
COOKIE_HEADER_JSON=""
if [ -n "$COOKIE_FILE" ]; then
    log_info "Loading cookies from: $COOKIE_FILE"
    COOKIE_STRING=$(parse_cookie_file "$COOKIE_FILE")
    if [ -n "$COOKIE_STRING" ]; then
        # Lighthouse expects --extra-headers as JSON object
        # Escape quotes in cookie string for JSON
        ESCAPED_COOKIES=$(echo "$COOKIE_STRING" | sed 's/"/\\"/g')
        COOKIE_HEADER_JSON="{\"Cookie\":\"$ESCAPED_COOKIES\"}"
        log_info "Using cookies for authentication"
    else
        log_warning "No valid cookies found in cookie file"
    fi
fi

readonly DEVICE_TYPE=$([ "$DESKTOP" = "true" ] && echo "desktop" || echo "mobile")
readonly TIMESTAMP=$(date +"%Y%m%d-%H%M%S")
readonly OUTPUT_PREFIX="$OUTPUT_DIR/lighthouse-$DEVICE_TYPE-$TIMESTAMP"

# Create a controlled temp directory for Chrome user data (to avoid cluttering project root)
readonly TEMP_USER_DATA_DIR=$(mktemp -d -t lighthouse-user-data-XXXXXX 2>/dev/null || echo "/tmp/lighthouse-user-data-$$")

# Build Lighthouse command
# For desktop, use --preset=desktop; for mobile, use --form-factor=mobile
# Use --user-data-dir to control where Chrome stores temp data
LIGHTHOUSE_CMD=(
    lighthouse "$URL"
    --only-categories=performance
    --output=html
    --output=json
    --output-path="$OUTPUT_PREFIX"
    --chrome-flags="--headless --no-sandbox --user-data-dir=$TEMP_USER_DATA_DIR"
    --quiet
)

# Add device-specific flags
if [ "$DESKTOP" = "true" ]; then
    LIGHTHOUSE_CMD+=(--preset=desktop)
else
    LIGHTHOUSE_CMD+=(--form-factor=mobile)
fi

# Add cookie header if provided (Lighthouse expects JSON format)
if [ -n "$COOKIE_HEADER_JSON" ]; then
    LIGHTHOUSE_CMD+=(--extra-headers="$COOKIE_HEADER_JSON")
fi

# Add any additional Lighthouse arguments
LIGHTHOUSE_CMD+=("${LIGHTHOUSE_ARGS[@]}")

if ! "${LIGHTHOUSE_CMD[@]}"; then
    log_error "Lighthouse audit failed"
    exit 1
fi

readonly HTML_FILE="$OUTPUT_PREFIX.report.html"
readonly JSON_FILE="$OUTPUT_PREFIX.report.json"

if [ -f "$HTML_FILE" ]; then
    log_success "Lighthouse report saved:"
    echo "  HTML: $HTML_FILE"
    echo "  JSON: $JSON_FILE"
    echo ""
    log_info "Opening report in browser..."
    
    # Try to open in default browser (platform-specific)
    if command -v xdg-open >/dev/null 2>&1; then
        xdg-open "$HTML_FILE" 2>/dev/null &
    elif command -v open >/dev/null 2>&1; then
        open "$HTML_FILE" 2>/dev/null &
    elif command -v start >/dev/null 2>&1; then
        start "$HTML_FILE" 2>/dev/null &
    fi
else
    log_error "Failed to generate Lighthouse report (HTML file not found)"
    exit 1
fi

# Extract key metrics from JSON (if jq is available)
if command -v jq >/dev/null 2>&1 && [ -f "$JSON_FILE" ]; then
    echo ""
    log_info "Key Performance Metrics:"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    
    SCORE=$(jq -r '.categories.performance.score * 100' "$JSON_FILE" 2>/dev/null || echo "N/A")
    FCP=$(jq -r '.audits["first-contentful-paint"].numericValue' "$JSON_FILE" 2>/dev/null || echo "N/A")
    LCP=$(jq -r '.audits["largest-contentful-paint"].numericValue' "$JSON_FILE" 2>/dev/null || echo "N/A")
    TBT=$(jq -r '.audits["total-blocking-time"].numericValue' "$JSON_FILE" 2>/dev/null || echo "N/A")
    CLS=$(jq -r '.audits["cumulative-layout-shift"].numericValue' "$JSON_FILE" 2>/dev/null || echo "N/A")
    TTFB=$(jq -r '.audits["server-response-time"].numericValue' "$JSON_FILE" 2>/dev/null || echo "N/A")
    
    format_metric() {
        local value=$1
        local format=$2
        if [[ "$value" == "N/A" ]] || [[ "$value" == "null" ]]; then
            echo "N/A"
        else
            printf "$format" "$value"
        fi
    }
    
    printf "Performance Score:  %s\n" "$(format_metric "$SCORE" "%.0f/100")"
    printf "FCP:               %s\n" "$(format_metric "$FCP" "%.0f ms")"
    printf "LCP:               %s\n" "$(format_metric "$LCP" "%.0f ms")"
    printf "TBT:               %s\n" "$(format_metric "$TBT" "%.0f ms")"
    printf "CLS:               %s\n" "$(format_metric "$CLS" "%.3f")"
    printf "TTFB:              %s\n" "$(format_metric "$TTFB" "%.0f ms")"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
fi

echo ""
log_success "Profiling complete!"

# Clean up Lighthouse temporary Chrome user data directory
log_info "Cleaning up temporary Lighthouse directories..."
if [ -d "$TEMP_USER_DATA_DIR" ]; then
    if rm -rf "$TEMP_USER_DATA_DIR" 2>/dev/null; then
        log_success "Cleaned up temporary directory"
    else
        log_warning "Could not remove temporary directory: $TEMP_USER_DATA_DIR"
    fi
fi

# Also clean up any old directories with Windows-style paths in project root (legacy cleanup)
cleaned=0
while IFS= read -r dir; do
    if [ -n "$dir" ] && [ -d "$dir" ] && [[ "$dir" == *"lighthouse."* ]]; then
        if rm -rf "$dir" 2>/dev/null; then
            ((cleaned++))
        fi
    fi
done < <(find . -maxdepth 1 -type d 2>/dev/null | grep -i "^\./C:" || true)
if [ $cleaned -gt 0 ]; then
    log_success "Cleaned up $cleaned legacy temporary directory(ies)"
fi

echo ""
echo "Tips:"
echo "  • Compare reports over time to track improvements"
echo "  • Check the 'Opportunities' section for optimization suggestions"
echo "  • Use Chrome DevTools Performance tab for deeper analysis"

