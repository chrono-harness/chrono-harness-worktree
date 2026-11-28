#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"
if [[ $# -eq 0 ]]; then
  /bin/sh .chrono-harness/ci/install.sh
  export PATH="$PWD/.chrono-harness/bin:$PATH"
  export CHRONO_CHECK_SOURCE=${CHRONO_CHECK_SOURCE:-local}
  exec .chrono-harness/bin/chrono-harness check
fi
if [[ $# -ne 1 || $1 != --native ]]; then
  echo 'usage: ./check.sh [--native]' >&2
  exit 2
fi
scratch_root=$(pwd -P)/.chrono-harness/state/native-check
mkdir -p "$scratch_root"
scratch=$(mktemp -d "$scratch_root/check.XXXXXXXX")
trap 'rm -rf -- "$scratch"' EXIT
export TMPDIR="$scratch"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$scratch/cargo}"
cargo fmt --check --manifest-path crates/worktree/Cargo.toml
product=crates/worktree/Cargo.toml
cargo build --locked --message-format=json-render-diagnostics --manifest-path "$product" > "$scratch/build.jsonl"
cargo fmt --check --manifest-path crates/worktree-tests/Cargo.toml
cargo test --locked --manifest-path crates/worktree-tests/Cargo.toml

coverage_failure() {
  echo "E_VERUS_COVERAGE: $*" >&2
  exit 1
}

# Read the exact library selected by Cargo in the build of the tested executable.
stem=$(chrono-host-bootstrap select-native-cargo-library "$(pwd -P)/$product" "$scratch/build.jsonl") ||
  coverage_failure "native library was not identified"
chrono-host-bootstrap check-compiler-sources native "$stem" src/facts.rs src/verified_owner_publication.rs src/verified_lifecycle.rs src/maintenance.rs src/remote.rs \
  src/check_inputs.rs src/check_inputs/observation_age.rs src/start.rs src/automatic.rs \
  src/automatic/owner_deployment.rs src/automatic/age.rs src/automatic/cache_adoption.rs src/automatic/inventory.rs \
  src/automatic/migration.rs src/automatic/service.rs src/automatic/producer.rs \
  src/automatic/producer/rolling.rs src/automatic/retirement.rs src/recovery.rs \
  src/lib.rs src/log_retention.rs src/verified_attention.rs src/automatic/standalone_expiry.rs ||
  coverage_failure "native library differs from required sources"
