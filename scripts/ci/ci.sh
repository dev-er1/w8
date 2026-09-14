#!/bin/sh

set -e

root=$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)

workspaces="
$root/w8
$root/wdt
"

steps="
Check|cargo check --manifest-path {WS}/Cargo.toml
Format|cargo fmt --all --check --manifest-path {WS}/Cargo.toml
Clippy|cargo clippy --all-targets --all-features --manifest-path {WS}/Cargo.toml -- -D warnings
Build|cargo build --release --manifest-path {WS}/Cargo.toml
Test|cargo test --workspace --release --manifest-path {WS}/Cargo.toml
"

for ws in $workspaces; do
    [ -z "$ws" ] && continue

    while IFS='|' read -r name command; do
        [ -z "$name" ] && continue

        cmd=$(printf '%s' "$command" | sed "s|{WS}|$ws|g")

        printf '======> [%s] %s\n' "$(basename "$ws")" "$name"

        sh -c "$cmd" || {
            status=$?
            printf '[%s] %s failed with exit code %s\n' "$(basename "$ws")" "$name" "$status" >&2
            exit "$status"
        }
    done <<EOF
$steps
EOF
done

printf '\n'
printf '        CI passed!\n'