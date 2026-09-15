#!/usr/bin/env bash
# Verify, then commit the reference-expressions change.
# Usage: ./commit.sh   (do not commit if verification fails)

set -euo pipefail

echo "==> cargo fmt --check"
cargo fmt --check

echo "==> cargo clippy --all-targets"
cargo clippy --all-targets

echo "==> cargo test"
cargo test

echo "==> verification passed; committing"
git add -A
git commit -m "feat!: replace macro replacements with root-relative reference expressions" \
  -m "- m!path / m'...' removed; fail with migration errors
- Data::Reference(Vec<Segment>) with Key/Index selectors replaces Data::Macro
- root-relative resolution only; self/super are ordinary identifiers
- quoted single/multi-label selectors and zero-based array indices
- type-preserving resolution incl. composites; loop/ambiguity diagnostics
- README and module docs migrated; compatibility changes documented"
