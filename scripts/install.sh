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
