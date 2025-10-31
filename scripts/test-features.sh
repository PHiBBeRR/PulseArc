#!/bin/bash
# Test all critical feature flag combinations for Phase 3 readiness
set -e
set -o pipefail

FEATURES=(
    ""                                    # default
    "calendar"
    "sap"
    "calendar,sap"
    "ml"
    "graphql"
    "tree-classifier"
    "sap,ml,graphql"
    "calendar,sap,ml"
    "calendar,sap,ml,graphql"            # all features
)

echo "🧪 Testing ${#FEATURES[@]} feature combinations for pulsearc-infra..."
echo ""

FAILED=0

for i in "${!FEATURES[@]}"; do
    FEATURE="${FEATURES[$i]}"
    if [ -z "$FEATURE" ]; then
        DISPLAY="default"
    else
        DISPLAY="$FEATURE"
    fi

    echo "[$((i+1))/${#FEATURES[@]}] Testing features: $DISPLAY"

    # Use exit code directly instead of grepping output
    if [ -z "$FEATURE" ]; then
        if cargo check -p pulsearc-infra --quiet 2>&1; then
            echo "✅ Features '$DISPLAY' compiled successfully"
        else
            echo "❌ Features '$DISPLAY' failed to compile"
            FAILED=$((FAILED + 1))
        fi
    else
        if cargo check -p pulsearc-infra --features "$FEATURE" --quiet 2>&1; then
            echo "✅ Features '$DISPLAY' compiled successfully"
        else
            echo "❌ Features '$DISPLAY' failed to compile"
            FAILED=$((FAILED + 1))
        fi
    fi
done

echo ""
if [ $FAILED -eq 0 ]; then
    echo "✅ All ${#FEATURES[@]} feature combinations compile successfully!"
    exit 0
else
    echo "❌ $FAILED feature combination(s) failed"
    exit 1
fi
