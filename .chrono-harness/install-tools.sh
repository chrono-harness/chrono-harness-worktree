#!/bin/sh
# Run from the checkout root. Publish a complete, verified mise selection by rename.
set -eu
fail() { echo "$*" >&2; exit 1; }
# Explicit user input applies to every selected organization tool.
if [ "${CHRONO_TOOL_CHANNEL+x}" = x ]; then
    case "$CHRONO_TOOL_CHANNEL" in
        stable|preview) ;;
        *[!A-Za-z0-9.-]*|'') fail 'Invalid CHRONO_TOOL_CHANNEL.' ;;
        *) printf '%s\n' "$CHRONO_TOOL_CHANNEL" | grep -Eq '^v[0-9]+[.][0-9]+[.][0-9]+(-[A-Za-z0-9-]+([.][A-Za-z0-9-]+)*)?$' ||
               fail 'CHRONO_TOOL_CHANNEL expects stable, preview or an exact v tag.' ;;
    esac
fi
select_tool() {
    selector=${CHRONO_TOOL_CHANNEL:-}
    [ -z "$selector" ] || return 0
    case "$1" in

        *) selector='pinned' ;;
    esac
}
case "$(uname -s)-$(uname -m)" in
    Linux-x86_64) platform=linux-x64; asset=linux-x86_64; seed_sha=271e79ab48a9c7442c8edc1d404e44430711ac486893240246f754255839b785 ;;
    Darwin-arm64) platform=macos-arm64; asset=macos-aarch64; seed_sha=7a7ac849578b7d53b98baaa7396b4273a97c9a4a9eb4998607681fb769f1a692 ;;
    *) fail 'Unsupported installation platform.' ;;
esac
verify() {
    if command -v sha256sum >/dev/null 2>&1; then digest=$(sha256sum < "$1")
    else digest=$(shasum -a 256 < "$1"); fi
    test "${digest%% *}" = "$2" || fail "SHA-256 mismatch for $1"
}
config_dir=$PWD/.config/mise/conf.d
work=$(mktemp -d "$config_dir/.chrono-install.XXXXXXXX")
downloads=
cleanup() {
    rm -rf -- "$work"
    if [ -n "$downloads" ]; then rm -rf -- "$downloads"; fi
}
trap cleanup EXIT
trap 'exit 1' HUP INT TERM
manifest=$work/mise.toml
: > "$manifest"
disabled=$(mise settings get disable_tools)
enabled() {
    case "$disabled" in *\"http:$1\"*) return 1 ;; *) return 0 ;; esac
}
resolved=$config_dir/zz-chrono-harness-resolved.toml
export MISE_IGNORED_CONFIG_PATHS="${MISE_IGNORED_CONFIG_PATHS:+$MISE_IGNORED_CONFIG_PATHS:}$resolved"
export MISE_TRUSTED_CONFIG_PATHS="$work${MISE_TRUSTED_CONFIG_PATHS:+:$MISE_TRUSTED_CONFIG_PATHS}"
while read -r command repository; do
    enabled "$command" || continue
    select_tool "$command"
    [ "$selector" != pinned ] || continue
    if [ -z "$downloads" ]; then
        temp_parent=$(cd "${TMPDIR:-/tmp}" && pwd -P)
        case "$temp_parent/" in "$(pwd -P)/"*) temp_parent=/tmp ;; esac
        downloads=$(mktemp -d "$temp_parent/chrono-install.XXXXXXXX")
        seed=$downloads/chrono-host-release
        curl --disable --proto '=https' --proto-redir '=https' --fail --silent --show-error --location \
            "https://github.com/chrono-harness/chrono-harness-host-release/releases/download/v0.1.0-alpha.29/chrono-host-release-$asset" -o "$seed"
        verify "$seed" "$seed_sha"
        chmod 700 "$seed"
    fi
    pages=$downloads/${repository#*/}
    if [ ! -d "$pages" ]; then
        mkdir "$pages"
        page=1
        while :; do
            set -- --disable --proto '=https' --fail --silent --show-error
            auth=
            if [ -n "${GITHUB_TOKEN:-}" ]; then
                auth="Authorization: Bearer $GITHUB_TOKEN"
                set -- "$@" -H @-
            fi
            if curl "$@" -D "$pages/headers" -o "$pages/$page.json" \
                "https://api.github.com/repos/$repository/releases?per_page=100&page=$page" <<AUTH
$auth
AUTH
            then :
            else
                fetch_status=$?
                echo "Failed to read releases for $repository (page $page)." >&2
                exit "$fetch_status"
            fi
            # Fetch the next numbered page only when GitHub advertises it.
            if grep -i '^link:' "$pages/headers" | grep -Fq 'rel="next"'; then page=$((page + 1))
            else break; fi
        done
    fi
    "$seed" resolve-install-v1 --mise-tool "$command" "$repository" "$selector" \
        "$command-linux-x86_64" "$command-macos-aarch64" "$pages"/*.json >> "$manifest"
done <<'TOOLS'
chrono-cache chrono-harness/chrono-harness-cache
chrono-ci chrono-harness/chrono-harness-ci
chrono-diagnostics chrono-harness/chrono-harness-diagnostics
chrono-distribution chrono-harness/chrono-harness-distribution
chrono-harness chrono-harness/chrono-harness-runner
chrono-host-bootstrap chrono-harness/chrono-harness-host-bootstrap
chrono-host-release chrono-harness/chrono-harness-host-release
chrono-inputs chrono-harness/chrono-harness-inputs
chrono-instructions chrono-harness/chrono-harness-instructions
chrono-judge-ci chrono-harness/chrono-harness-judge-ci
chrono-judge-cost chrono-harness/chrono-harness-judge-cost
chrono-judge-filemap chrono-harness/chrono-harness-judge-filemap
chrono-judge-mixed chrono-harness/chrono-harness-judge-mixed
chrono-judge-projects chrono-harness/chrono-harness-judge-projects
chrono-judge-registration chrono-harness/chrono-harness-judge-registration
chrono-judge-routes chrono-harness/chrono-harness-judge-routes
chrono-judge-workflow chrono-harness/chrono-harness-judge-workflow
chrono-pr chrono-harness/chrono-harness-pr
chrono-test-git chrono-harness/chrono-harness-test-git
chrono-test-git-profile chrono-harness/chrono-harness-test-git
chrono-worktree chrono-harness/chrono-harness-worktree
TOOLS
# The nested config takes precedence over any previous checkout receipt.
# mise installs explicit versions and verifies fresh downloads. Also hash cache hits.
mise -C "$work" install
for command in chrono-cache chrono-ci chrono-diagnostics chrono-distribution chrono-harness chrono-host-bootstrap chrono-host-release chrono-inputs chrono-instructions chrono-judge-ci chrono-judge-cost chrono-judge-filemap chrono-judge-mixed chrono-judge-projects chrono-judge-registration chrono-judge-routes chrono-judge-workflow chrono-pr chrono-test-git chrono-test-git-profile chrono-worktree; do
    enabled "$command" || continue
    select_tool "$command"
    source=$manifest
    if [ "$selector" = pinned ]; then source=$config_dir/chrono-harness.toml; fi
    expected=$(mise config get --file "$source" "tools.\"http:$command\".platforms.$platform.checksum")
    executable=$(mise -C "$work" which "$command")
    verify "$executable" "${expected#sha256:}"
done
if [ -s "$manifest" ]; then
    mv -f "$manifest" "$resolved"
else
    rm -f "$resolved"
fi
