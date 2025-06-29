# TODO: Same-Line Tracking Edge Cases

This document tracks edge cases and improvements for the same-line tracking feature that are not yet implemented.

## 🚨 High Priority (User-facing crashes)

### 1. Binary Files
- **Problem**: Line mapping will fail/crash on binary files (images, executables, etc.)
- **Current behavior**: Likely error or nonsensical mapping
- **Solution needed**: Detect binary files and disable line mapping, fall back to basic positioning
- **Implementation**: Check file content for null bytes or use git's binary detection

### 2. Very Large Files (Performance)
- **Problem**: Diffing 10,000+ line files could cause UI freezing
- **Current behavior**: Might block the UI thread during expensive diff operations
- **Solution needed**: Size limits, async processing, or simplified fallback for huge files
- **Implementation**: Add file size check, move diffing to async worker for large files

### 3. Encoding Issues & Non-UTF8 Files
- **Problem**: Files with weird encodings (Latin-1, binary-ish text files)
- **Current behavior**: Likely crashes or produces garbage
- **Solution needed**: Encoding detection and graceful degradation
- **Implementation**: Use encoding detection crate, fallback to bytes if UTF-8 fails

## 🔧 Medium Priority (Broken but non-crashy)

### 4. File Renames/Moves
- **Problem**: When a file is renamed (`src/old.rs` → `src/new.rs`), we lose tracking completely
- **Current behavior**: Treats as completely different files, resets cursor to top
- **Solution needed**: Git rename detection to maintain line mapping across file moves
- **Implementation**: Use git's rename detection (`git diff --find-renames`) to track file moves

### 5. Empty Files & File Creation/Deletion
- **Problem**: Mapping from/to empty files, or files that didn't exist in one commit
- **Current behavior**: Probably works but untested edge cases
- **Solution needed**: Graceful handling when file doesn't exist in old/new commit
- **Implementation**: Add comprehensive tests and proper error handling for missing files

### 6. Massive Refactoring/Rewritten Files
- **Problem**: When >90% of a file changes, line mapping becomes meaningless
- **Current behavior**: Falls back to proportional mapping (probably OK)
- **Solution needed**: Detect "total rewrite" and use better fallback strategies
- **Implementation**: Calculate similarity ratio, use alternative strategies for low similarity

## 🔍 Low Priority (Edge cases)

### 7. Git Submodules & Symlinks
- **Problem**: Special Git objects that aren't regular files
- **Current behavior**: Unknown, probably errors
- **Solution needed**: Detect and skip line mapping for special file types
- **Implementation**: Check git object type before attempting line mapping

### 8. Line Ending Differences (CRLF vs LF)
- **Problem**: Windows vs Unix line endings could throw off line counts
- **Current behavior**: Might work by accident, but untested
- **Solution needed**: Normalize line endings before diffing
- **Implementation**: Convert all line endings to LF before processing

### 9. Merge Commits & Complex Git History
- **Problem**: Merge commits have multiple parents - which one to map from?
- **Current behavior**: Uses first parent (probably fine)
- **Solution needed**: Consider if we need smarter merge commit handling
- **Implementation**: Research git merge strategies, possibly offer user choice

### 10. Tab vs Spaces & Whitespace Changes
- **Problem**: Pure whitespace changes might break exact line matching
- **Current behavior**: Probably works (diff should handle it)
- **Solution needed**: Test and potentially improve whitespace handling
- **Implementation**: Add whitespace normalization options in diff algorithm

## 🧪 Testing & Validation Needed

### Test Cases to Add
- [ ] Binary file handling (images, executables, zip files)
- [ ] Large file performance (10K+ lines)
- [ ] Non-UTF8 encoded files
- [ ] File renames with `git mv`
- [ ] Empty file transitions (empty → content → empty)
- [ ] Total file rewrites (similarity < 10%)
- [ ] Submodule and symlink handling
- [ ] Mixed line endings (CRLF/LF in same repo)
- [ ] Merge commit navigation
- [ ] Whitespace-only changes

### Performance Benchmarks
- [ ] Diff performance on large files (1K, 5K, 10K, 50K lines)
- [ ] Memory usage during complex diffs
- [ ] UI responsiveness during heavy line mapping operations

## 🔮 Future Enhancements

### Advanced Features (Nice to have)
- **Semantic line mapping**: Use AST/syntax tree for better code tracking
- **Multi-file tracking**: Track cursor across file renames automatically
- **Intelligent rebase handling**: Smart positioning during interactive rebases
- **Configuration options**: User preferences for fallback strategies
- **Visual indicators**: Show when exact vs approximate mapping is used
- **Caching layer**: Cache line mappings for frequently accessed commit pairs

### User Experience Improvements
- **Progress indicators**: Show progress for expensive diff operations
- **Mapping confidence**: Visual indication of mapping reliability
- **Manual override**: Allow users to manually adjust cursor position
- **Mapping history**: Show how cursor moved through recent commits
- **Smart defaults**: Learn from user behavior to improve mapping

## 📝 Implementation Notes

### Code Organization
- Consider creating a `line_mapping/edge_cases.rs` module
- Add comprehensive error types for different failure modes
- Implement feature flags for experimental edge case handling

### Testing Strategy
- Create test repository with problematic files
- Add property-based testing for edge cases
- Performance regression tests for large files

### Documentation
- Update README with known limitations
- Add troubleshooting guide for edge cases
- Document performance characteristics

---

*This TODO list was generated based on analysis of the current same-line tracking implementation. Items should be prioritized based on user impact and frequency of occurrence.*