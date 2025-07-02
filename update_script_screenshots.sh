#!/bin/bash

# Script to update all script test screenshots
# This script runs from the project root and updates all screenshots for script tests

set -e  # Exit on error

# Check if we're in the right directory
if [ ! -f "Cargo.toml" ]; then
    echo "❌ Error: This script must be run from the project root (where Cargo.toml is located)"
    exit 1
fi

# Check if test-repo exists
if [ ! -d "tests/test-repo" ]; then
    echo "❌ Error: tests/test-repo directory not found"
    exit 1
fi

echo "🔄 Updating all script test screenshots..."

# Find all script test directories
script_dirs=$(find tests/scripts -name "script" -type f | xargs dirname | sort)

if [ -z "$script_dirs" ]; then
    echo "❌ No script tests found in tests/scripts/"
    exit 1
fi

echo "📁 Found script test directories:"
echo "$script_dirs" | sed 's/^/  - /'

# Change to test-repo directory
cd tests/test-repo

echo ""
echo "🏗️  Building project..."
cargo build --bin git-lineage --quiet

total_tests=0
successful_tests=0
failed_tests=()

# Process each script test
for script_dir in $script_dirs; do
    test_name=$(basename "$script_dir")
    script_file="../../$script_dir/script"
    
    echo ""
    echo "📸 Updating screenshots for: $test_name"
    
    total_tests=$((total_tests + 1))
    
    # Run the script test in overwrite mode
    if cargo run --bin git-lineage --quiet -- test --script "$script_file" --overwrite; then
        # Copy generated screenshots to the test directory
        if ls *.txt >/dev/null 2>&1; then
            cp *.txt "../../$script_dir/"
            rm *.txt
            echo "✅ Successfully updated screenshots for $test_name"
            successful_tests=$((successful_tests + 1))
        else
            echo "⚠️  No screenshots generated for $test_name"
        fi
    else
        echo "❌ Failed to update screenshots for $test_name"
        failed_tests+=("$test_name")
    fi
done

# Return to project root
cd ../..

echo ""
echo "📊 Summary:"
echo "  Total tests: $total_tests"
echo "  Successful: $successful_tests"
echo "  Failed: ${#failed_tests[@]}"

if [ ${#failed_tests[@]} -eq 0 ]; then
    echo ""
    echo "🎉 All script test screenshots updated successfully!"
    echo ""
    echo "💡 Next steps:"
    echo "  1. Review the changes: git diff tests/scripts/"
    echo "  2. Stage the changes: git add tests/scripts/"
    echo "  3. Commit the changes: git commit -m '📸 Update script test screenshots'"
else
    echo ""
    echo "❌ Some tests failed:"
    printf '  - %s\n' "${failed_tests[@]}"
    exit 1
fi