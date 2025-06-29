#!/bin/bash

# Script to update all rendering test expected outputs
# This script regenerates the expected screenshot files for all rendering tests

set -e  # Exit on any error

echo "🔄 Updating rendering test expected outputs..."

# Directory containing test configurations
TEST_DIR="tests/rendering_tests"

# Check if the test directory exists
if [ ! -d "$TEST_DIR" ]; then
    echo "❌ Test directory $TEST_DIR not found!"
    exit 1
fi

# Counter for updated files
updated_count=0

# Find all .json test configuration files
for config_file in "$TEST_DIR"/*.json; do
    # Skip if no .json files found
    [ -e "$config_file" ] || continue
    
    # Extract test name (filename without extension)
    test_name=$(basename "$config_file" .json)
    expected_file="$TEST_DIR/${test_name}.expected.txt"
    
    echo "📸 Generating screenshot for: $test_name"
    
    # Generate the screenshot and save to expected file (using 80x25 to match test dimensions)
    if cargo run --bin git-lineage screenshot --config "$config_file" --width 80 --height 25 2>/dev/null > "$expected_file"; then
        echo "✅ Updated: $expected_file"
        ((updated_count++))
    else
        echo "❌ Failed to generate screenshot for: $test_name"
        exit 1
    fi
done

echo ""
echo "🎉 Successfully updated $updated_count rendering test expected outputs!"
echo ""
echo "🧪 Running tests to verify..."

# Run the rendering tests to verify they pass
if cargo test test_all_rendering --quiet; then
    echo "✅ All rendering tests pass!"
else
    echo "❌ Some rendering tests still fail. Please check the output above."
    exit 1
fi

echo ""
echo "✨ All done! Rendering test expected outputs have been updated and verified."