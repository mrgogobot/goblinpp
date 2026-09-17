#!/bin/sh
set -eu

prefix=""
build_from_source=0
while [ "$#" -gt 0 ]; do
    case "$1" in
        --prefix)
            [ "$#" -ge 2 ] || { echo "--prefix needs a path" >&2; exit 2; }
            prefix=$2
            shift 2
            ;;
        --build-from-source)
            build_from_source=1
            shift
            ;;
        *)
            echo "unknown option: $1" >&2
            exit 2
            ;;
    esac
done
[ -n "$prefix" ] || {
    echo "usage: ./install.sh --prefix /path/to/install [--build-from-source]" >&2
    echo "example: ./install.sh --prefix \"\$HOME/.local\"" >&2
    exit 2
}

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
cd "$script_dir"
mkdir -p "$prefix/bin"
prebuilt="$script_dir/dist/macos-arm64/goblin++"
if [ "$build_from_source" -eq 0 ] && [ "$(uname -s)" = "Darwin" ] && [ "$(uname -m)" = "arm64" ] && [ -f "$prebuilt" ]; then
    install -m 755 "$prebuilt" "$prefix/bin/goblin++"
    echo "Installed bundled macOS arm64 build."
else
    cargo build --release --locked
    install -m 755 target/release/goblinpp "$prefix/bin/goblin++"
    echo "Built and installed from locked Rust source."
fi

echo "Installed $prefix/bin/goblin++"
echo "Make sure $prefix/bin is on PATH."
