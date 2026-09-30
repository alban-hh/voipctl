#!/bin/sh
set -eu

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
binary="$script_dir/../target/release/voipctl"
prefix=/usr/local
destdir=${DESTDIR:-}

fail() {
    printf 'voipctl: %s\n' "$*" >&2
    exit 1
}

usage() {
    printf '%s\n' 'Usage: install.sh [--binary FILE] [--prefix PATH] [--destdir PATH]'
}

while [ "$#" -gt 0 ]; do
    case "$1" in
        --binary|--prefix|--destdir)
            [ "$#" -ge 2 ] && [ -n "$2" ] || fail "$1 requires a value"
            case "$1" in
                --binary) binary=$2 ;;
                --prefix) prefix=$2 ;;
                --destdir) destdir=$2 ;;
            esac
            shift 2
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        *) fail "unknown argument: $1" ;;
    esac
done

case "$prefix" in
    /*) ;;
    *) fail 'prefix must be an absolute path' ;;
esac
case "$destdir" in
    ''|/*) ;;
    *) fail 'destination root must be an absolute path' ;;
esac
case "/$prefix/$destdir/" in
    */../*) fail 'installation paths cannot contain parent components' ;;
esac

[ -f "$binary" ] && [ -s "$binary" ] && [ -x "$binary" ] || fail "expected a nonempty executable: $binary"

install_dir="${destdir}${prefix%/}/sbin"
destination="$install_dir/voipctl"
[ ! -d "$destination" ] || fail "destination is a directory: $destination"
[ ! -L "$destination" ] || fail "destination is a symbolic link: $destination"
mkdir -p -- "$install_dir"
temporary=$(mktemp "$install_dir/.voipctl.XXXXXX")
trap 'rm -f -- "$temporary"' EXIT
trap 'exit 1' HUP INT TERM
install -m 0755 -- "$binary" "$temporary"
mv -f -- "$temporary" "$destination"
printf 'Installed %s\n' "$destination"
