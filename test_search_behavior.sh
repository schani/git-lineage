#!/bin/bash

# Test script to verify search behavior
# This will create a simple test scenario and check the behavior

echo "Testing search exit behavior fix..."

# Set up environment variable to enable new navigator
export GIT_LINEAGE_LOG=1

# Create a test directory structure (if we're in a git repo)
if [ -d ".git" ]; then
    echo "✓ In git repository"
    
    # Run a simple test with the new binary
    echo "Starting git-lineage to test search behavior..."
    echo "Test scenario:"
    echo "1. Press '/' to start search"
    echo "2. Type 'foo' and press Enter - should show 'Search: foo' with no cursor"
    echo "3. Press '/' again, clear query, press Enter - should show just 'File Navigator'"
    echo "4. Press 'q' to quit"
    echo ""
    echo "Press any key to start the test application..."
    read -n 1 -s
    
    # Start the application
    ./target/debug/git-lineage
    
    echo "Test completed!"
else
    echo "❌ Not in a git repository. Please run this from a git repository root."
    exit 1
fi