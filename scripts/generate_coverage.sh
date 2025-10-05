#!/bin/bash

# Generate Test Coverage Report for Vision Analysis System
# Uses cargo-tarpaulin to generate comprehensive coverage metrics

set -e

echo "📊 Generating test coverage report..."

# Check if tarpaulin is installed
if ! command -v cargo-tarpaulin &> /dev/null; then
    echo "📦 Installing cargo-tarpaulin..."
    cargo install cargo-tarpaulin
fi

# Set database URL for integration tests
export DATABASE_URL="${DATABASE_URL:-postgresql://postgres:password@localhost:5432/ai_coach}"

# Clean previous coverage data
rm -rf target/coverage
mkdir -p target/coverage

echo ""
echo "🧪 Running tests with coverage..."

# Generate coverage report
cargo tarpaulin \
    --workspace \
    --out Html \
    --out Lcov \
    --output-dir target/coverage \
    --exclude-files "tests/*" \
    --exclude-files "*/mod.rs" \
    --timeout 300 \
    --verbose

# Calculate overall coverage percentage
COVERAGE=$(grep -oP 'line-rate="\K[0-9.]+' target/coverage/cobertura.xml 2>/dev/null | head -1)
COVERAGE_PERCENT=$(echo "$COVERAGE * 100" | bc 2>/dev/null || echo "0")

echo ""
echo "=============================="
echo "📈 Coverage Report Summary"
echo "=============================="
echo ""
echo "Overall Coverage: ${COVERAGE_PERCENT}%"
echo ""

# Check if we meet the 80% target
if (( $(echo "$COVERAGE_PERCENT >= 80" | bc -l) )); then
    echo "✅ Coverage target met (≥80%)"
else
    echo "❌ Coverage target NOT met (<80%)"
    echo "   Current: ${COVERAGE_PERCENT}%"
    echo "   Target:  80%"
    echo "   Gap:     $(echo "80 - $COVERAGE_PERCENT" | bc)%"
fi

echo ""
echo "📂 Coverage reports generated:"
echo "  HTML:  target/coverage/index.html"
echo "  LCOV:  target/coverage/lcov.info"
echo ""

# Generate module-specific coverage breakdown
echo "📊 Module Coverage Breakdown:"
echo ""

# Extract module coverage from lcov file if available
if [ -f target/coverage/lcov.info ]; then
    echo "Analyzing coverage by module..."

    # Group by service modules
    grep -E "SF:.*services/(.*\.rs)" target/coverage/lcov.info | \
        sed 's/SF:.*services\///' | \
        sed 's/\.rs//' | \
        sort -u | \
        while read module; do
            # Count covered and total lines for this module
            LINES=$(grep -A 100 "SF:.*services/${module}.rs" target/coverage/lcov.info | grep -E "^(LH|LF):" || echo "")
            if [ -n "$LINES" ]; then
                COVERED=$(echo "$LINES" | grep "^LH:" | cut -d: -f2)
                TOTAL=$(echo "$LINES" | grep "^LF:" | cut -d: -f2)

                if [ -n "$COVERED" ] && [ -n "$TOTAL" ] && [ "$TOTAL" -gt 0 ]; then
                    PERCENT=$(echo "scale=1; $COVERED * 100 / $TOTAL" | bc)
                    printf "  %-30s %5.1f%% (%d/%d lines)\n" "$module" "$PERCENT" "$COVERED" "$TOTAL"
                fi
            fi
        done
fi

echo ""
echo "💡 Tips for improving coverage:"
echo "  1. Add unit tests for uncovered edge cases"
echo "  2. Test error handling paths"
echo "  3. Add integration tests for critical workflows"
echo "  4. Test boundary conditions and invalid inputs"
echo ""

# Open HTML report in browser (macOS/Linux)
if command -v open &> /dev/null; then
    echo "🌐 Opening coverage report in browser..."
    open target/coverage/index.html
elif command -v xdg-open &> /dev/null; then
    echo "🌐 Opening coverage report in browser..."
    xdg-open target/coverage/index.html
else
    echo "📖 View report: open target/coverage/index.html"
fi

echo ""
echo "✅ Coverage generation complete!"
