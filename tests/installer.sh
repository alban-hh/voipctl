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

printf '#!/bin/sh\nexit 42\n' >"$binary"
sh "$installer" --binary "$binary" --destdir "$workspace/stage" --prefix '/opt/voip ctl' >"$workspace/output"
cmp "$binary" "$installed" || fail 'existing binary was not replaced'

DESTDIR="$workspace/environment" sh "$installer" --binary "$binary" >"$workspace/output"
cmp "$binary" "$workspace/environment/usr/local/sbin/voipctl" || fail 'DESTDIR was ignored'

expect_failure --binary "$workspace/missing" --destdir "$workspace/stage"
expect_failure --binary "$binary" --prefix relative
expect_failure --binary "$binary" --destdir relative
expect_failure --binary "$binary" --destdir "$workspace/stage/../outside"
expect_failure --binary "$binary" --prefix /usr/../bin
expect_failure --binary
expect_failure --unknown

mkdir -p "$workspace/symlink/usr/local/sbin"
ln -s "$binary" "$workspace/symlink/usr/local/sbin/voipctl"
expect_failure --binary "$binary" --destdir "$workspace/symlink"

mkdir -p "$workspace/directory/usr/local/sbin/voipctl"
expect_failure --binary "$binary" --destdir "$workspace/directory"

printf '' >"$workspace/empty"
chmod 0755 "$workspace/empty"
expect_failure --binary "$workspace/empty" --destdir "$workspace/stage"

chmod 0644 "$binary"
expect_failure --binary "$binary" --destdir "$workspace/stage"

sh "$installer" --help >"$workspace/output"
printf '%s\n' 'Installer unit tests passed.'
