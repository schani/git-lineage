# 🚀 Git Lineage Development Commands
# This Makefile provides local development parity with CI

.PHONY: help check test coverage clean install-tools format clippy doc audit build release

# Default target
help: ## 📖 Show this help message
	@echo "🚀 Git Lineage Development Commands"
	@echo ""
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-20s\033[0m %s\n", $$1, $$2}'

# 🔧 Development workflow commands
check: format clippy test coverage ## 🔍 Run all quality checks (CI equivalent)

test: ## 🧪 Run all tests
	@echo "🧪 Running tests..."
	cargo test --verbose
	cargo test --doc

coverage: ## 📊 Generate coverage report and check 70% threshold
	@echo "📊 Generating coverage report..."
	@if command -v cargo-llvm-cov >/dev/null 2>&1; then \
		echo "Using cargo-llvm-cov..."; \
		cargo llvm-cov --all-features --workspace --html --output-dir coverage/; \
		COVERAGE=$$(cargo llvm-cov --all-features --workspace --summary-only | grep -oE '[0-9]+\.[0-9]+%' | head -1 | sed 's/%//'); \
	elif command -v cargo-tarpaulin >/dev/null 2>&1; then \
		echo "Using cargo-tarpaulin..."; \
		cargo tarpaulin --all-features --workspace --out Html --output-dir coverage/; \
		COVERAGE=$$(cargo tarpaulin --all-features --workspace | grep -oE '[0-9]+\.[0-9]+% coverage' | grep -oE '[0-9]+\.[0-9]+'); \
	else \
		echo "Installing cargo-tarpaulin..."; \
		cargo install cargo-tarpaulin; \
		cargo tarpaulin --all-features --workspace --out Html --output-dir coverage/; \
		COVERAGE=$$(cargo tarpaulin --all-features --workspace | grep -oE '[0-9]+\.[0-9]+% coverage' | grep -oE '[0-9]+\.[0-9]+'); \
	fi; \
	echo "Current coverage: $$COVERAGE%"; \
	if command -v bc >/dev/null 2>&1; then \
		if [ $$(echo "$$COVERAGE >= 70" | bc -l) -eq 1 ]; then \
			echo "✅ Coverage $$COVERAGE% meets the 70% threshold!"; \
		else \
			echo "❌ Coverage $$COVERAGE% is below the 70% threshold!"; \
			exit 1; \
		fi; \
	else \
		if awk "BEGIN {exit !($$COVERAGE >= 70)}"; then \
			echo "✅ Coverage $$COVERAGE% meets the 70% threshold!"; \
		else \
			echo "❌ Coverage $$COVERAGE% is below the 70% threshold!"; \
			exit 1; \
		fi; \
	fi
	@echo "📄 Coverage report available at: coverage/index.html"

format: ## 🎨 Format code
	@echo "🎨 Formatting code..."
	cargo fmt --all

clippy: ## 📎 Run Clippy linter
	@echo "📎 Running Clippy..."
	cargo clippy --all-targets --all-features -- -D warnings

doc: ## 📚 Generate documentation
	@echo "📚 Generating documentation..."
	cargo doc --no-deps --document-private-items --open

audit: ## 🛡️ Run security audit
	@echo "🛡️ Running security audit..."
	@if ! command -v cargo-audit >/dev/null 2>&1; then \
		echo "Installing cargo-audit..."; \
		cargo install cargo-audit; \
	fi
	cargo audit

# 🔧 Installation and setup
install-tools: ## 🔧 Install development tools
	@echo "🔧 Installing development tools..."
	rustup component add rustfmt clippy llvm-tools-preview
	cargo install cargo-llvm-cov cargo-audit

# 🚀 Build commands
build: ## 🏗️ Build project in debug mode
	@echo "🏗️ Building project..."
	cargo build --verbose

release: ## 🚀 Build project in release mode
	@echo "🚀 Building release..."
	cargo build --release --verbose

# 🧹 Cleanup
clean: ## 🧹 Clean build artifacts
	@echo "🧹 Cleaning build artifacts..."
	cargo clean
	rm -rf coverage/

# 🔍 Debugging helpers
check-fmt: ## 🔍 Check if code is formatted (CI equivalent)
	@echo "🔍 Checking code formatting..."
	cargo fmt --all -- --check

check-clippy: ## 🔍 Check Clippy warnings without fixing
	@echo "🔍 Checking Clippy warnings..."
	cargo clippy --all-targets --all-features -- -D warnings

quick-test: ## ⚡ Run tests quickly (unit tests only)
	@echo "⚡ Running quick tests..."
	cargo test --lib

# 📈 Performance testing
bench: ## 📈 Run benchmarks
	@echo "📈 Running benchmarks..."
	cargo test --release --test '*' -- --ignored

# 🎯 Specific test categories
test-unit: ## 🧪 Run unit tests only
	@echo "🧪 Running unit tests..."
	cargo test --lib

test-integration: ## 🧪 Run integration tests only
	@echo "🧪 Running integration tests..."
	cargo test --test '*'

test-doc: ## 📚 Run documentation tests only
	@echo "📚 Running documentation tests..."
	cargo test --doc

# 🔄 Pre-commit workflow
pre-commit: format check-fmt check-clippy quick-test ## 🔄 Quick pre-commit checks

# 🚀 CI simulation
ci-local: clean install-tools check ## 🚀 Simulate full CI pipeline locally