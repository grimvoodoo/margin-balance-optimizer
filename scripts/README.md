# Development Scripts

This directory contains helpful scripts for development and maintenance.

## install-hooks.sh

Installs git pre-commit hooks that run the same checks as the GitHub Actions CI pipeline.

### Usage

```bash
./scripts/install-hooks.sh
```

### What it does

The pre-commit hook will automatically run before each commit and check:

- **Code formatting** (`cargo fmt --check`) - Ensures code follows Rust formatting standards
- **Linting** (`cargo clippy`) - Checks for common mistakes and code quality issues
- **Build** (`cargo build`) - Verifies the project compiles successfully
- **Tests** (`cargo test`) - Runs all unit and integration tests

If any check fails, the commit will be blocked and you'll need to fix the issues first.

### Why use this?

Running these checks locally before committing helps:

- Catch issues early in your development workflow
- Avoid wasting GitHub Actions minutes on preventable failures
- Keep the main branch stable
- Get faster feedback (local checks are faster than waiting for CI)

### Bypassing the hook

If you absolutely need to commit without running checks (not recommended):

```bash
git commit --no-verify
```

## Setting up a new clone

After cloning this repository, run:

```bash
./scripts/install-hooks.sh
```

This ensures your local environment is set up to match the team's workflow.
