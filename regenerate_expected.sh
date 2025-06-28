#!/bin/bash

echo "🔄 Regenerating expected rendering test screenshots"
echo "================================================="

# Directory containing test files
TEST_DIR="tests/rendering_tests"

# Find all JSON test configurations (excluding .expected files)
for config_file in "$TEST_DIR"/*.json; do
    if [[ ! "$config_file" =~ \.expected\. ]]; then
        # Extract test name (remove path and .json extension)
        test_name=$(basename "$config_file" .json)
        expected_file="$TEST_DIR/${test_name}.expected.txt"
        
        echo "📸 Regenerating: $test_name"
        
        # Generate the screenshot with fixed dimensions
        cargo run -- screenshot --config "$config_file" --output "$expected_file" --width 80 --height 25
        
        if [ $? -eq 0 ]; then
            echo "✅ Generated: $expected_file"
        else
            echo "❌ Failed to generate: $expected_file"
        fi
    fi
done

echo ""
echo "🎉 Screenshot regeneration complete!"
echo "Run 'cargo test test_all_rendering' to verify all tests pass"