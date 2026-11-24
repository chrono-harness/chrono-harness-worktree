#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"
mkdir -p target
scratch=$(mktemp -d "$(pwd -P)/target/check.XXXXXXXX")
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

prove() {
  chrono-host-bootstrap run-compiler-proof "$scratch" "$1" "${VERUS:-verus}" "$2" ||
    coverage_failure "$1: verifier execution failed"
}

admit_proof() {
  local operation=$1
  shift
  chrono-host-bootstrap collect-compiler-proof "$scratch" "$operation" "${VERUS:-verus}" \
    0.2026.04.24.f8e1704 f8e17045c4ac17a02e566b8997070f2eecc255be "$@" ||
    coverage_failure "$operation: required proof execution was not admitted"
}

# Read the exact library selected by Cargo in the build of the tested executable.
metadata=$(jq -ers --arg manifest "$(pwd -P)/$product" '
  [.[] | select(.reason == "compiler-artifact" and .manifest_path == $manifest
                 and .target.kind == ["lib"]) | .filenames[] | select(endswith(".rmeta"))]
  | if length == 1 then .[0] else error("expected the product library") end
' "$scratch/build.jsonl") || coverage_failure "native library was not identified"
filename=${metadata##*/}
stem=${filename#lib}
stem=${metadata%/*}/${stem%.rmeta}
chrono-host-bootstrap check-compiler-sources native "$stem" src/verified_owner_publication.rs src/verified_lifecycle.rs src/maintenance.rs \
  src/check_inputs.rs src/start.rs src/automatic.rs \
  src/automatic/owner_deployment.rs src/automatic/age.rs src/automatic/inventory.rs \
  src/automatic/migration.rs src/automatic/service.rs src/automatic/producer.rs \
  src/automatic/producer/rolling.rs src/automatic/retirement.rs src/recovery.rs \
  src/lib.rs src/log_retention.rs src/verified_attention.rs src/automatic/standalone_expiry.rs ||
  coverage_failure "native library differs from required sources"

src=crates/worktree/src
tests=crates/worktree-tests/tests
included="$tests/../../worktree/src"
support="$tests/proof_support/worktree"
prove verify.owner-publication "$src/verified_owner_publication.rs"
prove verify.worktree-lifecycle "$src/verified_lifecycle.rs"
prove verify.worktree-attention "$src/verified_attention.rs"
prove verify.owner-publication-tests "$tests/verified_owner_publication.rs"
prove verify.worktree-attention-tests "$tests/verified_attention.rs"
prove verify.worktree-lifecycle-tests "$tests/verified_lifecycle.rs"

admit_proof verify.owner-publication "$src/verified_owner_publication.rs"
admit_proof verify.worktree-lifecycle "$src/verified_lifecycle.rs" "$src/maintenance.rs" \
  "$src/check_inputs.rs" "$src/start.rs" \
  "$src/automatic.rs" "$src/automatic/owner_deployment.rs" \
  "$src/automatic/age.rs" "$src/automatic/inventory.rs" \
  "$src/automatic/migration.rs" "$src/automatic/service.rs" \
  "$src/automatic/producer.rs" "$src/automatic/producer/rolling.rs" \
  "$src/automatic/retirement.rs" "$src/recovery.rs" \
  "$src/lib.rs"
admit_proof verify.worktree-attention "$src/verified_attention.rs"
admit_proof verify.owner-publication-tests "$tests/verified_owner_publication.rs" "$included/verified_owner_publication.rs"
admit_proof verify.worktree-attention-tests "$tests/verified_attention.rs" "$included/verified_attention.rs"
admit_proof verify.worktree-lifecycle-tests "$tests/verified_lifecycle.rs" "$included/verified_lifecycle.rs" \
  "$included/maintenance.rs" "$included/check_inputs.rs" \
  "$included/start.rs" "$support/check_inputs.rs" \
  "$support/automatic.rs" "$support/complete_inventory.rs" \
  "$support/unified_maintenance.rs" "$included/automatic.rs" \
  "$included/automatic/owner_deployment.rs" "$included/automatic/age.rs" \
  "$included/automatic/inventory.rs" "$included/automatic/migration.rs" \
  "$included/automatic/service.rs" "$included/automatic/producer.rs" \
  "$included/automatic/producer/rolling.rs" "$included/automatic/retirement.rs" \
  "$included/recovery.rs" "$included/lib.rs" \
  "$support/temporary.rs" "$support/age.rs" \
  "$support/bounded_cleanup.rs" "$support/creation.rs" \
  "$support/interrupted_cleanup.rs" "$support/policy_migration.rs" \
  "$tests/proof_support/ci/behavior.rs" "$support/fixture_body.rs" \
  "$support/support/git_fault.rs"
