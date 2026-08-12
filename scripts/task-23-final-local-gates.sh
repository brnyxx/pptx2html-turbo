#!/usr/bin/env bash
set -euo pipefail

root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
evidence="$root/.omo/evidence/task-23-exactness/task-23-final-local-gates.txt"
mkdir -p "$(dirname "$evidence")"

run_gates() {
  cd "$root"
  echo '$ cargo fmt --all -- --check'
  cargo fmt --all -- --check
  echo '$ cargo clippy --workspace --all-targets -- -D warnings'
  cargo clippy --workspace --all-targets -- -D warnings
  echo '$ cargo test --workspace'
  cargo test --workspace
  echo '$ python3 -m unittest discover -s evaluate/tests -p '\''test_*.py'\'' -v'
  python3 -m unittest discover -s evaluate/tests -p 'test_*.py' -v
  echo '$ python3 evaluate/check_completeness_manifest.py --manifest evaluate/completeness_manifest.json --repo-root .'
  python3 evaluate/check_completeness_manifest.py --manifest evaluate/completeness_manifest.json --repo-root .
  echo '$ python3 evaluate/check_preset_adjustments.py --repo-root .'
  python3 evaluate/check_preset_adjustments.py --repo-root .
  echo '$ python3 evaluate/check_exactness_contract.py --repo-root .'
  python3 evaluate/check_exactness_contract.py --repo-root .
  echo '$ python3 -m py_compile evaluate/*.py evaluate/tests/test_*.py'
  python3 -m py_compile evaluate/*.py evaluate/tests/test_*.py
  echo '$ git diff --check && git diff --cached --check'
  git diff --check
  git diff --cached --check
}

run_gates 2>&1 | tee "$evidence"
