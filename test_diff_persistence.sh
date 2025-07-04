#!/bin/bash
# Test script to verify diff view persistence

echo "Testing diff view persistence in git-lineage"
echo "============================================"
echo ""
echo "Instructions:"
echo "1. Press 'd' to enable diff view"
echo "2. Navigate to different files using Tab and arrow keys"
echo "3. Navigate to different commits using arrow keys in history panel"
echo "4. Verify that diff view remains active"
echo "5. Press 'd' again to disable diff view"
echo "6. Press 'q' to quit"
echo ""
echo "Starting git-lineage..."

cd tests/test-repo
cargo run --bin git-lineage