#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"
mkdir -p target
scratch=$(mktemp -d "$(pwd -P)/target/check.XXXXXXXX")
trap 'rm -rf -- "$scratch"' EXIT
export TMPDIR="$scratch"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$scratch/cargo}"
cargo fmt --check --manifest-path crates/worktree/Cargo.toml
cargo build --locked --manifest-path crates/worktree/Cargo.toml
cargo fmt --check --manifest-path crates/worktree-tests/Cargo.toml
cargo test --locked --manifest-path crates/worktree-tests/Cargo.toml
prove() {
  local label=$1 root=$2
  local output="$scratch/$label.rlib"
  "${VERUS:-verus}" --crate-type=lib --no-cheating --no-external-by-default --num-threads 2 \
    --output-json --compile "--emit=link=$output,dep-info=$output.d" "$root" -o "$output" \
    -Zchecksum-hash-algorithm=sha256 > "$scratch/proof.json" || { cat "$scratch/proof.json" >&2; return 1; }
  cat "$scratch/proof.json"
  jq -e '.verus.version == "0.2026.04.24.f8e1704" and
    .verus.commit == "f8e17045c4ac17a02e566b8997070f2eecc255be" and
    (."verification-results" | .success == true and ."is-verifying-entire-crate" == true and
    ."encountered-error" == false and ."encountered-vir-error" == false and
    .errors == 0 and .verified > 0)' "$scratch/proof.json" >/dev/null
  test -s "$output" && test -s "$output.d"
}

prove verify.owner-publication crates/worktree/src/verified_owner_publication.rs
prove verify.worktree-lifecycle crates/worktree/src/verified_lifecycle.rs
prove verify.worktree-attention crates/worktree/src/verified_attention.rs
prove verify.owner-publication-tests crates/worktree-tests/tests/verified_owner_publication.rs
prove verify.worktree-attention-tests crates/worktree-tests/tests/verified_attention.rs
prove verify.worktree-lifecycle-tests crates/worktree-tests/tests/verified_lifecycle.rs
