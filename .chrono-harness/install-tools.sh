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
case "$(uname -s)-$(uname -m)" in
    Linux-x86_64) platform=linux-x64; asset=linux-x86_64; seed_sha=8fd9f01086ad928167f2a6a0db1f6277ffc9c60cbfaf5b1994de946044040058 ;;
    Darwin-arm64) platform=macos-arm64; asset=macos-aarch64; seed_sha=ff5dd4a2943b672b7fd2f1450c4a1e27dd181cbefe12536c92c75a06dd84d63e ;;
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
legacy=$config_dir/zz-chrono-harness-resolved.toml
projection=$config_dir/selected-chrono-harness.toml
state=$PWD/.chrono-harness/state
selection=$state/tool-selection
resolved=$selection/versions.toml
data=$work/versions.toml
: > "$data"
validate_target() {
    for dir in "$PWD/.chrono-harness" "$state" "$selection"; do
        if [ -L "$dir" ] || { [ -e "$dir" ] && [ ! -d "$dir" ]; }; then
            fail "Expected an installation state directory: $dir"
        fi
    done
    for file in "$legacy" "$resolved"; do
        if [ -L "$file" ] || { [ -e "$file" ] && [ ! -f "$file" ]; }; then
            fail "Expected a regular installation selection: $file"
        fi
    done
    for entry in "$selection"/* "$selection"/.[!.]* "$selection"/..?*; do
        if [ "$entry" != "$resolved" ] && { [ -e "$entry" ] || [ -L "$entry" ]; }; then
            fail "Unexpected installation data entry: $entry"
        fi
    done
}
validate_target
export MISE_IGNORED_CONFIG_PATHS="${MISE_IGNORED_CONFIG_PATHS:+$MISE_IGNORED_CONFIG_PATHS:}$legacy:$projection"
export MISE_TRUSTED_CONFIG_PATHS="$work${MISE_TRUSTED_CONFIG_PATHS:+:$MISE_TRUSTED_CONFIG_PATHS}"
resolve_dynamic() {
    if [ -z "$downloads" ]; then
        temp_parent=$(cd "${TMPDIR:-/tmp}" && pwd -P)
        case "$temp_parent/" in "$(pwd -P)/"*) temp_parent=/tmp ;; esac
        downloads=$(mktemp -d "$temp_parent/chrono-install.XXXXXXXX")
        seed=$downloads/chrono-host-release
        curl --disable --proto '=https' --proto-redir '=https' --fail --silent --show-error --location \
            "https://github.com/chrono-harness/chrono-harness-host-release/releases/download/v0.1.1-alpha.2/chrono-host-release-$asset" -o "$seed"
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
            # Fixed-width names preserve GitHub's order when the resolver receives the glob.
            file=$(printf '%020d' "$page")
            if curl "$@" -D "$pages/$file.headers" -o "$pages/$file.json" \
                "https://api.github.com/repos/$repository/releases?per_page=100&page=$page" <<AUTH
$auth
AUTH
            then :
            else
                fetch_status=$?
                echo "Failed to read releases for $repository (page $page)." >&2
                exit "$fetch_status"
            fi
            state=$("$seed" resolve-install-v1 --page-state "$pages/$file.headers")
            case "$state" in
                next) page=$((page + 1)) ;;
                done) break ;;
                *) fail "Unexpected release page state: $state" ;;
            esac
        done
    fi
    "$seed" resolve-install-v1 --mise-tool "$command" "$repository" "$selector" \
        "$command-linux-x86_64" "$command-macos-aarch64" "$pages"/*.json >> "$manifest"
}
# Resolve each enabled command once, then install and verify that exact selection.
set --
: > "$work/checksums"
while read -r command repository default_selector; do
    enabled "$command" || continue
    selector=${CHRONO_TOOL_CHANNEL-$default_selector}
    if [ "$selector" = pinned ]; then
        source=$config_dir/chrono-harness.toml
    else
        resolve_dynamic
        source=$manifest
    fi
    version=$(mise config get --file "$source" "tools.\"http:$command\".version")
    expected=$(mise config get --file "$source" "tools.\"http:$command\".platforms.$platform.checksum")
    if [ "$selector" != pinned ]; then
        case $platform in
            linux-x64) linux=$expected; macos=$(mise config get --file "$source" "tools.\"http:$command\".platforms.macos-arm64.checksum") ;;
            macos-arm64) macos=$expected; linux=$(mise config get --file "$source" "tools.\"http:$command\".platforms.linux-x64.checksum") ;;
        esac
        printf '%s = ["%s", "%s", "%s"]\n' "$command" "$version" "$linux" "$macos" >> "$data"
    fi
    set -- "$@" "http:$command@$version"
    printf '%s %s\n' "$command" "$expected" >> "$work/checksums"
done <<'TOOLS'
chrono-cache chrono-harness/chrono-harness-cache pinned
chrono-ci chrono-harness/chrono-harness-ci pinned
chrono-diagnostics chrono-harness/chrono-harness-diagnostics pinned
chrono-distribution chrono-harness/chrono-harness-distribution pinned
chrono-harness chrono-harness/chrono-harness-runner pinned
chrono-host-bootstrap chrono-harness/chrono-harness-host-bootstrap pinned
chrono-host-release chrono-harness/chrono-harness-host-release pinned
chrono-inputs chrono-harness/chrono-harness-inputs pinned
chrono-instructions chrono-harness/chrono-harness-instructions pinned
chrono-judge-ci chrono-harness/chrono-harness-judge-ci pinned
chrono-judge-cost chrono-harness/chrono-harness-judge-cost pinned
chrono-judge-filemap chrono-harness/chrono-harness-judge-filemap pinned
chrono-judge-mixed chrono-harness/chrono-harness-judge-mixed pinned
chrono-judge-projects chrono-harness/chrono-harness-judge-projects pinned
chrono-judge-registration chrono-harness/chrono-harness-judge-registration pinned
chrono-judge-routes chrono-harness/chrono-harness-judge-routes pinned
chrono-judge-workflow chrono-harness/chrono-harness-judge-workflow pinned
chrono-pr chrono-harness/chrono-harness-pr pinned
chrono-test-git chrono-harness/chrono-harness-test-git pinned
chrono-test-git-profile chrono-harness/chrono-harness-test-git pinned
chrono-worktree chrono-harness/chrono-harness-worktree pinned
TOOLS
# An explicit mise argument installs the requested release even when a matching prerelease is cached.
if [ "$#" -gt 0 ]; then mise -C "$work" install "$@"; fi
# Install remaining host tools, including Rust and Verus where selected.
mise -C "$work" install
while read -r command expected; do
    executable=$(mise -C "$work" which "$command")
    verify "$executable" "${expected#sha256:}"
done < "$work/checksums"
validate_target
if [ -s "$data" ]; then
    mkdir -p "$selection"
    validate_target
    mv -f "$data" "$selection/"
else
    rm -f "$resolved"
fi
rm -f "$legacy"
