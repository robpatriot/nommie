#!/usr/bin/env bash
# Performance profiling script for Nommie
# This script runs Lighthouse audits and saves reports
#
# Usage:
#   ./profile-performance.sh [URL] [OUTPUT_DIR] [DESKTOP]
#   ./profile-performance.sh http://localhost:3000 ./reports true

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
readonly URL="${1:-http://localhost:3000}"
readonly OUTPUT_DIR="${2:-./performance-reports}"
readonly DESKTOP="${3:-false}"

log_info "Nommie Performance Profiling"
echo "URL: $URL"
echo "Output directory: $OUTPUT_DIR"
echo "Device: $([ "$DESKTOP" = "true" ] && echo "Desktop" || echo "Mobile")"
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

# Create output directory
mkdir -p "$OUTPUT_DIR"

# Run Lighthouse audit
log_info "Running Lighthouse audit..."

readonly DEVICE_TYPE=$([ "$DESKTOP" = "true" ] && echo "desktop" || echo "mobile")
readonly PRESET=$([ "$DESKTOP" = "true" ] && echo "desktop" || echo "mobile")
readonly TIMESTAMP=$(date +"%Y%m%d-%H%M%S")
readonly OUTPUT_PREFIX="$OUTPUT_DIR/lighthouse-$DEVICE_TYPE-$TIMESTAMP"

if ! lighthouse "$URL" \
    --only-categories=performance \
    --preset="$PRESET" \
    --output=html \
    --output=json \
    --output-path="$OUTPUT_PREFIX" \
    --chrome-flags="--headless --no-sandbox" \
    --quiet; then
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
echo ""
echo "Tips:"
echo "  • Compare reports over time to track improvements"
echo "  • Check the 'Opportunities' section for optimization suggestions"
echo "  • Use Chrome DevTools Performance tab for deeper analysis"

