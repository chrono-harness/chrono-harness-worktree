#!/bin/sh
set -eu

if command -v mise >/dev/null 2>&1; then
    manager=mise
elif [ "${GITHUB_ACTIONS:-}" = true ]; then
    case "$(uname -s)/$(uname -m)" in
        Darwin/arm64) asset=macos-arm64; digest=87b2bd5a1b88a612ee8856df23499404b115870b5a8a8e008763c7cd51c8d412 ;;
        Linux/x86_64) asset=linux-x64; digest=9c12c24f5eac6b6357e5265e2629166cc3005459037f5d747680a99e375ff2ab ;;
        *) echo 'Unsupported runner platform' >&2; exit 1 ;;
    esac
    stage=$(mktemp -d "$RUNNER_TEMP/worktree-mise.XXXXXXXX")
    trap 'rm -rf -- "$stage"' EXIT
    manager=$stage/mise
    curl --fail --silent --show-error --location "https://github.com/jdx/mise/releases/download/v2026.11.12/mise-v2026.11.12-$asset" --output "$manager"
    (cd "$stage" && echo "$digest  mise" | shasum -a 256 -c -)
    chmod 700 "$manager"
else
    echo 'Install mise and run mise trust in the checkout' >&2
    exit 1
fi

if [ "${GITHUB_ACTIONS:-}" = true ]; then
    export MISE_TRUSTED_CONFIG_PATHS="$PWD"
fi
set -- chrono-harness chrono-worktree chrono-diagnostics chrono-ci chrono-judge-ci chrono-judge-routes chrono-judge-registration chrono-judge-projects chrono-judge-filemap chrono-host-bootstrap verus
mkdir -p .chrono-harness/bin
for tool do
    if ! executable=$("$manager" which "$tool" 2>/dev/null); then
        "$manager" install "http:$tool"
        executable=$("$manager" which "$tool")
    fi
    ln -sfn "$executable" ".chrono-harness/bin/$tool"
    if [ -n "${GITHUB_PATH:-}" ]; then
        dirname "$executable" >> "$GITHUB_PATH"
    fi
done
