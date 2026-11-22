#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"
scratch=$(mktemp -d)
trap 'rm -rf -- "$scratch"' EXIT
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$scratch/cargo}"
cargo fmt --check --manifest-path crates/worktree/Cargo.toml
cargo build --locked --manifest-path crates/worktree/Cargo.toml
cargo fmt --check --manifest-path crates/worktree-tests/Cargo.toml
cargo test --locked --manifest-path crates/worktree-tests/Cargo.toml
proof_bindings=()
prove() {
  local label=$1 root=$2
  local output="$scratch/$label.rlib"
  "${VERUS:-verus}" --crate-type=lib --no-cheating --no-external-by-default --num-threads 2 \
    "${proof_bindings[@]}" --output-json --compile "--emit=link=$output,dep-info=$output.d" "$root" -o "$output" \
    -Zchecksum-hash-algorithm=sha256 > "$scratch/proof.json" || { cat "$scratch/proof.json" >&2; return 1; }
  cat "$scratch/proof.json"
  jq -e '.verus.version == "0.2026.04.24.f8e1704" and
    .verus.commit == "f8e17045c4ac17a02e566b8997070f2eecc255be" and
    (."verification-results" | .success == true and ."is-verifying-entire-crate" == true and
    ."encountered-error" == false and ."encountered-vir-error" == false and
    .errors == 0 and .verified > 0)' "$scratch/proof.json" >/dev/null
  test -s "$output" && test -s "$output.d"
}

# Resolve the exact source that Cargo fetched for the runtime dependency.
identifiers_manifest=$(cargo metadata --locked --manifest-path crates/worktree/Cargo.toml --format-version 1 |
  jq -er '[.packages[] | select(.name == "chrono-identifiers") | .manifest_path] | select(length == 1) | first')
proof_bindings=(--crate-name chrono_identifiers --export "$scratch/chrono_identifiers.vir")
prove libchrono_identifiers "$(dirname "$identifiers_manifest")/src/lib.rs"
proof_bindings=(--extern "chrono_identifiers=$scratch/libchrono_identifiers.rlib"
                --import "chrono_identifiers=$scratch/chrono_identifiers.vir")
prove verify.owner-publication crates/worktree/src/verified_owner_publication.rs
prove verify.worktree-lifecycle crates/worktree/src/verified_lifecycle.rs
prove verify.worktree-attention crates/worktree/src/verified_attention.rs
prove verify.owner-publication-tests crates/worktree-tests/tests/verified_owner_publication.rs
prove verify.worktree-attention-tests crates/worktree-tests/tests/verified_attention.rs
prove verify.worktree-lifecycle-tests crates/worktree-tests/tests/verified_lifecycle.rs
