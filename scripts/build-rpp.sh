#!/usr/bin/env bash
#
# Builds rpp to WebAssembly for the browser, as a trunk pre_build hook.
# Rebuilds only when the pinned submodule or the link flags change, so the
# `trunk serve` watch loop does not wait on C++ every reload.

set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source_dir="$root/vendor/rpp"
build_dir="$root/target/rpp-build"
out_dir="$root/target/rpp"

# base.r is embedded so an #include resolves without staging, and main waits
# for an explicit callMain instead of running on load.
link_flags="-O3 \
-sMODULARIZE=1 \
-sEXPORT_NAME=createRpp \
-sINVOKE_RUN=0 \
-sEXPORTED_RUNTIME_METHODS=callMain,FS \
-sEXIT_RUNTIME=0 \
-sALLOW_MEMORY_GROWTH=1 \
--embed-file $source_dir/rules++/base.r@/rpp/base.r"

if [[ ! -f "$source_dir/CMakeLists.txt" ]]; then
    echo "vendor/rpp is empty, run: git submodule update --init" >&2
    exit 1
fi

revision="$(git -C "$source_dir" rev-parse HEAD)"
flag_hash="$(printf '%s' "$link_flags" | sha256sum | cut -d' ' -f1)"
stamp="$out_dir/.stamp"
expected="$revision $flag_hash"

if [[ -f "$stamp" && "$(cat "$stamp")" == "$expected" ]]; then
    echo "rpp wasm is up to date (${revision:0:7})"
    exit 0
fi

if ! command -v emcmake > /dev/null; then
    echo "emcmake not found, install emscripten (Arch: pacman -S emscripten)" >&2
    exit 1
fi

emcmake cmake -S "$source_dir" -B "$build_dir" \
    -DCMAKE_BUILD_TYPE=Release \
    -DCMAKE_EXE_LINKER_FLAGS="$link_flags"
cmake --build "$build_dir" -j"$(getconf _NPROCESSORS_ONLN 2> /dev/null || echo 4)"

mkdir -p "$out_dir"
cp "$build_dir/rpp.js" "$build_dir/rpp.wasm" "$out_dir/"
cp "$source_dir/LICENSE" "$out_dir/LICENSE-rpp"

printf '%s' "$expected" > "$stamp"
echo "built rpp wasm (${revision:0:7})"
