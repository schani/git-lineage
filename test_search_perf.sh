#!/bin/bash
# Test search performance with detailed profiling

echo "Testing search performance with query 's'"
echo "Run with RUST_LOG=debug to see detailed profiling"
echo ""

# Run git-lineage with search query 's' and capture logs
echo "Running optimized version..."
RUST_LOG=debug timeout 10s cargo run --release -- --test-search-query s 2>&1 | grep -E "(Search:|Phase|Directory|Avg|⚠️|✨|📋)" || true

echo ""
echo "To run interactively and see the performance:"
echo "RUST_LOG=debug cargo run --release"
echo "Then press '/' to search and type 's'"
echo ""
echo "The optimization changes O(n×m) directory checks to O(1) lookups!"