#!/bin/bash

echo "🧪 Testing Git Lineage Executor System"
echo "======================================="

# Test 1: Panel navigation
echo "📋 Test 1: Panel Navigation"
cargo run -- execute --config test_configs/default.json --command "next_panel" --output test1_result.json
echo "✅ Next panel executed"

# Test 2: History navigation
echo "📋 Test 2: History Navigation"
cargo run -- execute --config test_configs/history_panel.json --command "history_down" --output test2_result.json
echo "✅ History down executed"

# Test 3: Search mode
echo "📋 Test 3: Search Mode"
cargo run -- execute --config test_configs/default.json --command "start_search" --output test3_result.json
echo "✅ Search mode activated"

# Test 4: Inspector toggle diff (switch to inspector first)
echo "📋 Test 4: Inspector Toggle Diff"
# First switch to inspector panel, then toggle diff
cargo run -- execute --config test_configs/default.json --command "next_panel" --output temp.json 2>/dev/null
cargo run -- execute temp.json --command "next_panel" --output temp2.json 2>/dev/null
cargo run -- execute temp2.json --command "toggle_diff" --output test4_result.json
echo "✅ Diff view toggled"

# Test 5: Search input sequence
echo "📋 Test 5: Search Input Sequence"
cargo run -- execute --config test_configs/default.json --command "start_search" --output temp_search.json 2>/dev/null
cargo run -- execute temp_search.json --command "search:c" --output temp_search2.json 2>/dev/null
cargo run -- execute temp_search2.json --command "search:o" --output temp_search3.json 2>/dev/null
cargo run -- execute temp_search3.json --command "search:n" --output test5_result.json
echo "✅ Search sequence executed"

# Test 6: Inspector navigation
echo "📋 Test 6: Inspector Navigation"
cargo run -- execute test4_result.json --command "inspector_down" --output test6_result.json
echo "✅ Inspector navigation executed"

# Verify results
echo ""
echo "🔍 Verification Results:"
echo "======================="

echo -n "Test 1 - Panel switch: "
if grep -q '"active_panel": "History"' test1_result.json; then
    echo "✅ PASSED"
else
    echo "❌ FAILED"
fi

echo -n "Test 2 - History navigation: "
if grep -q '"selected_commit_index": 2' test2_result.json; then
    echo "✅ PASSED"
else
    echo "❌ FAILED"
fi

echo -n "Test 3 - Search mode: "
if grep -q '"in_search_mode": true' test3_result.json; then
    echo "✅ PASSED"
else
    echo "❌ FAILED"
fi

echo -n "Test 4 - Diff toggle: "
if grep -q '"show_diff_view": true' test4_result.json; then
    echo "✅ PASSED"
else
    echo "❌ FAILED"
fi

echo -n "Test 5 - Search input: "
if grep -q '"search_query": "con"' test5_result.json; then
    echo "✅ PASSED"
else
    echo "❌ FAILED"
fi

echo -n "Test 6 - Inspector nav: "
if grep -q '"cursor_line": 4' test6_result.json; then
    echo "✅ PASSED"
else
    echo "❌ FAILED"
fi

# Cleanup
rm -f temp*.json

echo ""
echo "🎉 Executor system testing complete!"