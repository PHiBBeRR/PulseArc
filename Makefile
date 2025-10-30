# PulseArc Makefile
# Comprehensive build automation for Rust + TypeScript Tauri app

.PHONY: help install build test clean fmt lint ci dev run all

# Default target
.DEFAULT_GOAL := help

##@ General

help: ## Display this help message
	@awk 'BEGIN {FS = ":.*##"; printf "\nUsage:\n  make \033[36m<target>\033[0m\n"} /^[a-zA-Z_-]+:.*?##/ { printf "  \033[36m%-15s\033[0m %s\n", $$1, $$2 } /^##@/ { printf "\n\033[1m%s\033[0m\n", substr($$0, 5) } ' $(MAKEFILE_LIST)

##@ Setup & Installation

install: install-rust install-frontend ## Install all dependencies (Rust + pnpm)

install-rust: ## Install Rust dependencies
	@echo "Installing Rust dependencies..."
	cargo fetch

install-frontend: ## Install frontend dependencies
	@echo "Installing frontend dependencies..."
	pnpm install

##@ Building

build: build-frontend build-rust ## Build everything (frontend + backend)

build-frontend: ## Build frontend only
	@echo "Building frontend..."
	pnpm build

build-rust: ## Build Rust workspace
	@echo "Building Rust workspace..."
	cargo build --workspace

build-release: build-frontend ## Build optimized release binary
	@echo "Building release binary..."
	cargo build --workspace --release

build-tauri: build-frontend ## Build Tauri app bundle
	@echo "Building Tauri app..."
	cargo tauri build

##@ Development

dev: ## Run development server (Tauri dev mode)
	@echo "Starting Tauri dev server..."
	cargo tauri dev

dev-frontend: ## Run frontend dev server only
	@echo "Starting frontend dev server..."
	pnpm dev

watch: ## Watch and rebuild on changes
	@echo "Watching for changes..."
	cargo watch -x "build --workspace"

##@ Testing

test: test-rust ## Run all tests

test-rust: ## Run Rust tests
	@echo "Running Rust tests..."
	cargo test --workspace --all-features

test-frontend: ## Run frontend tests
	@echo "Running frontend tests..."
	pnpm test

##@ Code Quality

fmt: fmt-rust ## Format all code

fmt-rust: ## Format Rust code with nightly
	@echo "Formatting Rust code..."
	cargo +nightly fmt --all

fmt-check: ## Check Rust formatting
	@echo "Checking Rust formatting..."
	cargo +nightly fmt --all -- --check

fmt-frontend: ## Format frontend code
	@echo "Formatting frontend code..."
	pnpm format

lint: lint-rust ## Lint all code

lint-rust: ## Run clippy lints
	@echo "Running clippy..."
	cargo clippy --workspace --exclude xtask --all-targets --all-features -- -D warnings

lint-frontend: ## Lint frontend code
	@echo "Linting frontend..."
	pnpm lint

##@ Verification & CI

check: fmt-check lint test ## Run all checks (format, lint, test)

ci: ## Run full CI pipeline locally
	@echo "Running CI pipeline..."
	@$(MAKE) fmt-check
	@$(MAKE) lint-rust
	@$(MAKE) build-frontend
	@$(MAKE) test-rust
	@echo "✓ CI pipeline passed!"

audit: ## Run security audits
	@echo "Running cargo audit..."
	@cargo audit
	@echo "Running cargo deny..."
	@cargo deny check

##@ Cleaning

clean: clean-rust clean-frontend ## Clean all build artifacts

clean-rust: ## Clean Rust build artifacts
	@echo "Cleaning Rust artifacts..."
	cargo clean

clean-frontend: ## Clean frontend build artifacts
	@echo "Cleaning frontend artifacts..."
	rm -rf frontend/dist
	rm -rf node_modules

clean-all: clean ## Deep clean (includes dependencies)
	@echo "Deep cleaning..."
	rm -rf target
	rm -rf node_modules
	rm -rf frontend/dist
	rm -rf pnpm-lock.yaml

##@ Utilities

update: ## Update dependencies
	@echo "Updating Rust dependencies..."
	cargo update
	@echo "Updating frontend dependencies..."
	pnpm update

outdated: ## Check for outdated dependencies
	@echo "Checking outdated Rust crates..."
	cargo outdated
	@echo "Checking outdated npm packages..."
	pnpm outdated

tree: ## Show dependency tree
	@echo "Rust dependency tree:"
	cargo tree
	@echo "\nFrontend dependency tree:"
	pnpm list --depth=1

doctor: ## Check development environment
	@echo "Checking development environment..."
	@echo "\n=== Rust ==="
	@rustc --version || echo "❌ rustc not found"
	@cargo --version || echo "❌ cargo not found"
	@cargo +nightly --version 2>/dev/null || echo "⚠️  nightly toolchain not installed"
	@echo "\n=== Node.js ==="
	@node --version || echo "❌ node not found"
	@pnpm --version || echo "❌ pnpm not found"
	@echo "\n=== System ==="
	@echo "OS: $$(uname -s)"
	@echo "Arch: $$(uname -m)"

##@ Database

db-setup: ## Setup database
	@echo "Setting up database..."
	pnpm prisma generate
	pnpm prisma db push

db-migrate: ## Run database migrations
	@echo "Running migrations..."
	pnpm prisma migrate dev

db-reset: ## Reset database
	@echo "Resetting database..."
	pnpm prisma migrate reset

##@ Git

commit: fmt lint ## Format, lint, and prepare for commit
	@echo "Code is ready for commit!"

pre-push: ci ## Run full CI before pushing
	@echo "Ready to push!"
