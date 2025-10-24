#!/bin/bash

# Script to install git hooks for this repository
# Run this after cloning the repository to set up pre-commit checks

set -euo pipefail

# Preflight checks
command -v cargo >/dev/null 2>&1 || { echo "❌ cargo not found in PATH"; exit 1; }
rustup component list 2>/dev/null | grep -q 'rustfmt.*installed' || { echo "❌ Install rustfmt: rustup component add rustfmt"; exit 1; }
rustup component list 2>/dev/null | grep -q 'clippy.*installed' || { echo "❌ Install clippy: rustup component add clippy"; exit 1; }

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Use configured hooks path if present; else default to .git/hooks
CONFIGURED_HOOKS_DIR="$(git config --get core.hooksPath || true)"
if [ -n "$CONFIGURED_HOOKS_DIR" ]; then
  # If relative, resolve from repo root
  if [[ "$CONFIGURED_HOOKS_DIR" = /* ]]; then
    HOOKS_DIR="$CONFIGURED_HOOKS_DIR"
  else
    HOOKS_DIR="$REPO_ROOT/$CONFIGURED_HOOKS_DIR"
  fi
else
  HOOKS_DIR="$REPO_ROOT/.git/hooks"
fi

mkdir -p "$HOOKS_DIR"

echo "Installing git hooks for margin-balance-optimizer..."
echo "Hooks directory: $HOOKS_DIR"
echo ""

# Create the pre-commit hook
cat > "$HOOKS_DIR/pre-commit" << 'EOF'
#!/bin/bash

# Pre-commit hook to run the same checks as GitHub Actions
# This helps avoid wasting GitHub Actions minutes on preventable failures

set -euo pipefail

echo "🔍 Running pre-commit checks..."
echo ""

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Track if any checks fail
FAILED=0

# 1. Check code formatting
echo "📝 Checking code formatting..."
if cargo fmt --all -- --check > /dev/null 2>&1; then
    echo -e "${GREEN}✓${NC} Code formatting check passed"
else
    echo -e "${RED}✗${NC} Code formatting check failed"
    echo -e "${YELLOW}Run 'cargo fmt' to fix formatting issues${NC}"
    FAILED=1
fi
echo ""

# 2. Run clippy
echo "🔎 Running clippy lints..."
if cargo clippy --workspace --all-targets -- -D warnings > /dev/null 2>&1; then
    echo -e "${GREEN}✓${NC} Clippy check passed"
else
    echo -e "${RED}✗${NC} Clippy found issues"
    echo -e "${YELLOW}Fix clippy warnings before committing${NC}"
    cargo clippy --workspace --all-targets -- -D warnings
    FAILED=1
fi
echo ""

# 3. Build the project
echo "🔨 Building project..."
if cargo build --verbose > /dev/null 2>&1; then
    echo -e "${GREEN}✓${NC} Build successful"
else
    echo -e "${RED}✗${NC} Build failed"
    echo -e "${YELLOW}Fix build errors before committing${NC}"
    cargo build --verbose
    FAILED=1
fi
echo ""

# 4. Run tests
echo "🧪 Running tests..."
if [ "${SKIP_TESTS:-0}" = "1" ]; then
    echo -e "${YELLOW}↷${NC} Skipping tests (SKIP_TESTS=1)"
else
    if cargo test --workspace --verbose > /dev/null 2>&1; then
        echo -e "${GREEN}✓${NC} All tests passed"
    else
        echo -e "${RED}✗${NC} Tests failed"
        echo -e "${YELLOW}Fix failing tests before committing${NC}"
        cargo test --workspace --verbose
        FAILED=1
    fi
fi
echo ""

# Exit with error if any check failed
if [ $FAILED -eq 1 ]; then
    echo -e "${RED}❌ Pre-commit checks failed!${NC}"
    echo -e "${YELLOW}Fix the issues above before committing.${NC}"
    echo ""
    echo "To bypass this hook (not recommended), use: git commit --no-verify"
    exit 1
fi

echo -e "${GREEN}✅ All pre-commit checks passed!${NC}"
echo ""
EOF

# Make the hook executable
chmod +x "$HOOKS_DIR/pre-commit"

echo "✅ Pre-commit hook installed successfully!"
echo ""
echo "The hook will run automatically before each commit and check:"
echo "  • Code formatting (cargo fmt)"
echo "  • Linting (cargo clippy)"
echo "  • Build success (cargo build)"
echo "  • Tests (cargo test)"
echo ""
echo "To bypass the hook (not recommended), use: git commit --no-verify"
