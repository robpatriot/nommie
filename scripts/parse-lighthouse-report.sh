#!/usr/bin/env bash
# Parse and display Lighthouse performance report summaries
#
# Usage:
#   ./parse-lighthouse-report.sh [REPORT_FILE]
#   ./parse-lighthouse-report.sh ./performance-reports/lighthouse-desktop-*.json
#   ./parse-lighthouse-report.sh --compare report1.json report2.json

set -euo pipefail

# Colors for output
readonly RED='\033[0;31m'
readonly GREEN='\033[0;32m'
readonly YELLOW='\033[1;33m'
readonly BLUE='\033[0;34m'
readonly CYAN='\033[0;36m'
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

# Check if jq is available, fall back to Python
if command -v jq >/dev/null 2>&1; then
    USE_JQ=true
else
    USE_JQ=false
    if ! command -v python3 >/dev/null 2>&1; then
        log_error "Neither jq nor python3 is available. Please install one of them."
        exit 1
    fi
fi

# Format score with color
format_score() {
    local score=$1
    if (( $(echo "$score >= 90" | bc -l 2>/dev/null || echo 0) )); then
        echo -e "${GREEN}${score}${NC}"
    elif (( $(echo "$score >= 50" | bc -l 2>/dev/null || echo 0) )); then
        echo -e "${YELLOW}${score}${NC}"
    else
        echo -e "${RED}${score}${NC}"
    fi
}

# Format metric with color based on thresholds
format_metric() {
    local value=$1
    local format=$2
    local threshold_good=$3
    local threshold_warn=$4
    
    if [[ "$value" == "N/A" ]] || [[ "$value" == "null" ]]; then
        echo -e "${YELLOW}N/A${NC}"
    else
        local formatted=$(printf "$format" "$value")
        if (( $(echo "$value <= $threshold_good" | bc -l 2>/dev/null || echo 0) )); then
            echo -e "${GREEN}${formatted}${NC}"
        elif (( $(echo "$value <= $threshold_warn" | bc -l 2>/dev/null || echo 0) )); then
            echo -e "${YELLOW}${formatted}${NC}"
        else
            echo -e "${RED}${formatted}${NC}"
        fi
    fi
}

# Parse single report using Python
parse_report_python() {
    local file="$1"
    
    python3 << EOF
import json
import sys

try:
    with open('$file', 'r') as f:
        data = json.load(f)
    
    # Overall score
    score = data.get('categories', {}).get('performance', {}).get('score', 0) * 100
    
    # Core Web Vitals
    audits = data.get('audits', {})
    fcp = audits.get('first-contentful-paint', {}).get('numericValue', 0)
    lcp = audits.get('largest-contentful-paint', {}).get('numericValue', 0)
    tbt = audits.get('total-blocking-time', {}).get('numericValue', 0)
    cls = audits.get('cumulative-layout-shift', {}).get('numericValue', 0)
    ttfb = audits.get('server-response-time', {}).get('numericValue', 0)
    
    # Device type from URL or filename
    url = data.get('finalUrl', '')
    device_type = 'desktop' if 'desktop' in '$file' else 'mobile'
    
    print(f"SCORE:{score:.0f}")
    print(f"DEVICE:{device_type}")
    print(f"FCP:{fcp:.0f}")
    print(f"LCP:{lcp:.0f}")
    print(f"TBT:{tbt:.0f}")
    print(f"CLS:{cls:.3f}")
    print(f"TTFB:{ttfb:.0f}")
    
    # Opportunities
    opps = []
    for key, audit in audits.items():
        if audit.get('score') is not None and audit.get('score') < 1:
            wasted = audit.get('details', {}).get('overallSavingsMs', 0)
            if wasted > 0 or audit.get('score', 1) < 0.9:
                opps.append({
                    'title': audit.get('title', key),
                    'score': audit.get('score', 1) * 100,
                    'wasted': wasted,
                })
    
    # Sort by wasted time
    opps.sort(key=lambda x: x['wasted'], reverse=True)
    
    for opp in opps[:10]:
        print(f"OPP:{opp['title']}|{opp['score']:.0f}|{opp['wasted']:.0f}")
    
    # Diagnostics
    diags = []
    for key, audit in audits.items():
        if audit.get('scoreDisplayMode') == 'informative' and audit.get('numericValue'):
            diags.append({
                'title': audit.get('title', key),
                'value': audit.get('numericValue', 0),
                'unit': audit.get('numericUnit', ''),
            })
    
    diags.sort(key=lambda x: x['value'], reverse=True)
    for diag in diags[:5]:
        print(f"DIAG:{diag['title']}|{diag['value']:.0f}|{diag['unit']}")
        
except Exception as e:
    print(f"ERROR:{e}", file=sys.stderr)
    sys.exit(1)
EOF
}

# Display single report
display_report() {
    local file="$1"
    local label="${2:-$(basename "$file")}"
    
    if [ ! -f "$file" ]; then
        log_error "Report file not found: $file"
        return 1
    fi
    
    log_info "Performance Report: $label"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    
    # Parse report
    local parsed_output
    if [ "$USE_JQ" = true ]; then
        # Using jq (faster but more complex parsing)
        local score=$(jq -r '.categories.performance.score * 100' "$file" 2>/dev/null || echo "0")
        local device=$(echo "$file" | grep -q "mobile" && echo "mobile" || echo "desktop")
        local fcp=$(jq -r '.audits["first-contentful-paint"].numericValue' "$file" 2>/dev/null || echo "0")
        local lcp=$(jq -r '.audits["largest-contentful-paint"].numericValue' "$file" 2>/dev/null || echo "0")
        local tbt=$(jq -r '.audits["total-blocking-time"].numericValue' "$file" 2>/dev/null || echo "0")
        local cls=$(jq -r '.audits["cumulative-layout-shift"].numericValue' "$file" 2>/dev/null || echo "0")
        local ttfb=$(jq -r '.audits["server-response-time"].numericValue' "$file" 2>/dev/null || echo "0")
        
        parsed_output="SCORE:${score}
DEVICE:${device}
FCP:${fcp}
LCP:${lcp}
TBT:${tbt}
CLS:${cls}
TTFB:${ttfb}"
    else
        parsed_output=$(parse_report_python "$file")
    fi
    
    # Extract values
    local score=$(echo "$parsed_output" | grep "^SCORE:" | cut -d: -f2)
    local device=$(echo "$parsed_output" | grep "^DEVICE:" | cut -d: -f2)
    local fcp=$(echo "$parsed_output" | grep "^FCP:" | cut -d: -f2)
    local lcp=$(echo "$parsed_output" | grep "^LCP:" | cut -d: -f2)
    local tbt=$(echo "$parsed_output" | grep "^TBT:" | cut -d: -f2)
    local cls=$(echo "$parsed_output" | grep "^CLS:" | cut -d: -f2)
    local ttfb=$(echo "$parsed_output" | grep "^TTFB:" | cut -d: -f2)
    
    # Display score
    echo ""
    echo -e "Performance Score: $(format_score "$score")/100"
    echo -e "Device Type: ${CYAN}${device}${NC}"
    echo ""
    
    # Display Core Web Vitals
    echo "Core Web Vitals:"
    echo "────────────────────────────────────────────────────────────────────────────────"
    
    # Thresholds (mobile vs desktop)
    local fcp_good fcp_warn lcp_good lcp_warn tbt_good tbt_warn
    if [ "$device" = "mobile" ]; then
        fcp_good=1800
        fcp_warn=3000
        lcp_good=2500
        lcp_warn=4000
        tbt_good=200
        tbt_warn=600
    else
        fcp_good=1000
        fcp_warn=2500
        lcp_good=2500
        lcp_warn=4000
        tbt_good=200
        tbt_warn=600
    fi
    
    printf "  First Contentful Paint (FCP):  %s\n" "$(format_metric "$fcp" "%.0f ms" "$fcp_good" "$fcp_warn")"
    printf "  Largest Contentful Paint (LCP): %s\n" "$(format_metric "$lcp" "%.0f ms" "$lcp_good" "$lcp_warn")"
    printf "  Total Blocking Time (TBT):      %s\n" "$(format_metric "$tbt" "%.0f ms" "$tbt_good" "$tbt_warn")"
    printf "  Cumulative Layout Shift (CLS):  %s\n" "$(format_metric "$cls" "%.3f" "0.1" "0.25")"
    printf "  Time to First Byte (TTFB):      %s\n" "$(format_metric "$ttfb" "%.0f ms" "800" "1800")"
    echo ""
    
    # Display opportunities
    local opps=$(echo "$parsed_output" | grep "^OPP:")
    if [ -n "$opps" ]; then
        echo "Top Optimization Opportunities:"
        echo "────────────────────────────────────────────────────────────────────────────────"
        echo "$opps" | head -5 | while IFS='|' read -r title score wasted; do
            title=$(echo "$title" | sed 's/^OPP://')
            if [ -n "$wasted" ] && [ "$wasted" != "0" ]; then
                printf "  • %s (Score: %s%%, Savings: %sms)\n" "$title" "$(format_score "$score")" "$wasted"
            else
                printf "  • %s (Score: %s%%)\n" "$title" "$(format_score "$score")"
            fi
        done
        echo ""
    fi
    
    # Display diagnostics
    local diags=$(echo "$parsed_output" | grep "^DIAG:")
    if [ -n "$diags" ]; then
        echo "Diagnostics:"
        echo "────────────────────────────────────────────────────────────────────────────────"
        echo "$diags" | head -3 | while IFS='|' read -r title value unit; do
            title=$(echo "$title" | sed 's/^DIAG://')
            printf "  %s: %s%s\n" "$title" "$value" "$unit"
        done
        echo ""
    fi
}

# Compare two reports
compare_reports() {
    local file1="$1"
    local file2="$2"
    
    log_info "Comparing Reports"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo ""
    
    # Parse both reports
    local parsed1 parsed2
    if [ "$USE_JQ" = true ]; then
        parsed1=$(parse_report_python "$file1")
        parsed2=$(parse_report_python "$file2")
    else
        parsed1=$(parse_report_python "$file1")
        parsed2=$(parse_report_python "$file2")
    fi
    
    # Extract values
    local score1=$(echo "$parsed1" | grep "^SCORE:" | cut -d: -f2)
    local score2=$(echo "$parsed2" | grep "^SCORE:" | cut -d: -f2)
    local fcp1=$(echo "$parsed1" | grep "^FCP:" | cut -d: -f2)
    local fcp2=$(echo "$parsed2" | grep "^FCP:" | cut -d: -f2)
    local lcp1=$(echo "$parsed1" | grep "^LCP:" | cut -d: -f2)
    local lcp2=$(echo "$parsed2" | grep "^LCP:" | cut -d: -f2)
    local tbt1=$(echo "$parsed1" | grep "^TBT:" | cut -d: -f2)
    local tbt2=$(echo "$parsed2" | grep "^TBT:" | cut -d: -f2)
    local cls1=$(echo "$parsed1" | grep "^CLS:" | cut -d: -f2)
    local cls2=$(echo "$parsed2" | grep "^CLS:" | cut -d: -f2)
    local ttfb1=$(echo "$parsed1" | grep "^TTFB:" | cut -d: -f2)
    local ttfb2=$(echo "$parsed2" | grep "^TTFB:" | cut -d: -f2)
    
    local label1=$(basename "$file1" | sed 's/lighthouse-//; s/-.*//')
    local label2=$(basename "$file2" | sed 's/lighthouse-//; s/-.*//')
    
    echo "Metric Comparison:"
    echo "────────────────────────────────────────────────────────────────────────────────"
    printf "%-30s %15s %15s %15s\n" "Metric" "$label1" "$label2" "Difference"
    echo "────────────────────────────────────────────────────────────────────────────────"
    
    # Score
    local diff_score=$(echo "$score2 - $score1" | bc -l 2>/dev/null || echo "0")
    printf "%-30s %15s %15s %15s\n" "Performance Score" "$(format_score "$score1")/100" "$(format_score "$score2")/100" "$(printf "%+.0f" "$diff_score")"
    
    # FCP
    local diff_fcp=$(echo "$fcp2 - $fcp1" | bc -l 2>/dev/null || echo "0")
    local pct_fcp=$(echo "scale=1; ($diff_fcp / $fcp1) * 100" | bc -l 2>/dev/null || echo "0")
    printf "%-30s %15s %15s %15s\n" "FCP" "${fcp1}ms" "${fcp2}ms" "$(printf "%+.0fms (%+.1f%%)" "$diff_fcp" "$pct_fcp")"
    
    # LCP
    local diff_lcp=$(echo "$lcp2 - $lcp1" | bc -l 2>/dev/null || echo "0")
    local pct_lcp=$(echo "scale=1; ($diff_lcp / $lcp1) * 100" | bc -l 2>/dev/null || echo "0")
    printf "%-30s %15s %15s %15s\n" "LCP" "${lcp1}ms" "${lcp2}ms" "$(printf "%+.0fms (%+.1f%%)" "$diff_lcp" "$pct_lcp")"
    
    # TBT
    local diff_tbt=$(echo "$tbt2 - $tbt1" | bc -l 2>/dev/null || echo "0")
    local pct_tbt=$(echo "scale=1; ($diff_tbt / $tbt1) * 100" | bc -l 2>/dev/null || echo "0")
    printf "%-30s %15s %15s %15s\n" "TBT" "${tbt1}ms" "${tbt2}ms" "$(printf "%+.0fms (%+.1f%%)" "$diff_tbt" "$pct_tbt")"
    
    # CLS
    local diff_cls=$(echo "$cls2 - $cls1" | bc -l 2>/dev/null || echo "0")
    printf "%-30s %15s %15s %15s\n" "CLS" "$cls1" "$cls2" "$(printf "%+.3f" "$diff_cls")"
    
    # TTFB
    local diff_ttfb=$(echo "$ttfb2 - $ttfb1" | bc -l 2>/dev/null || echo "0")
    local pct_ttfb=$(echo "scale=1; ($diff_ttfb / $ttfb1) * 100" | bc -l 2>/dev/null || echo "0")
    printf "%-30s %15s %15s %15s\n" "TTFB" "${ttfb1}ms" "${ttfb2}ms" "$(printf "%+.0fms (%+.1f%%)" "$diff_ttfb" "$pct_ttfb")"
    
    echo ""
}

# Main script logic
main() {
    if [ $# -eq 0 ]; then
        # No arguments - find most recent report
        local latest=$(ls -t ./performance-reports/*.json 2>/dev/null | head -1)
        if [ -z "$latest" ]; then
            log_error "No report files found in ./performance-reports/"
            echo ""
            echo "Usage:"
            echo "  $0 [REPORT_FILE]"
            echo "  $0 --compare report1.json report2.json"
            exit 1
        fi
        display_report "$latest" "Latest Report"
    elif [ "$1" = "--compare" ] || [ "$1" = "-c" ]; then
        # Compare mode
        if [ $# -lt 3 ]; then
            log_error "Compare mode requires two report files"
            echo "Usage: $0 --compare report1.json report2.json"
            exit 1
        fi
        display_report "$2" "Report 1"
        echo ""
        display_report "$3" "Report 2"
        echo ""
        compare_reports "$2" "$3"
    else
        # Single report
        display_report "$1"
    fi
}

main "$@"

