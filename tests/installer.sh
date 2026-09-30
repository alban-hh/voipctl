#!/bin/sh
set -eu

test_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
installer="$test_dir/../scripts/install.sh"
workspace=$(mktemp -d)
trap 'rm -rf -- "$workspace"' EXIT
trap 'exit 1' HUP INT TERM

fail() {
    printf 'FAIL: %s\n' "$*" >&2
    exit 1
}

expect_failure() {
    if sh "$installer" "$@" >"$workspace/output" 2>&1; then
        fail "unexpected success: $*"
    fi
}

binary="$workspace/input binary"
printf '#!/bin/sh\nexit 0\n' >"$binary"
chmod 0755 "$binary"

sh "$installer" --binary "$binary" --destdir "$workspace/stage" --prefix '/opt/voip ctl' >"$workspace/output"
installed="$workspace/stage/opt/voip ctl/sbin/voipctl"
cmp "$binary" "$installed" || fail 'staged binary differs'
[ -x "$installed" ] || fail 'staged binary is not executable'

