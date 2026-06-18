#!/bin/bash
set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "$0")" && pwd)
STAGE_DIR="$SCRIPT_DIR/ananicy-cpp"
BUILD_DIR="$STAGE_DIR/.build"
VERSION="v1.1.1"
TARBALL="$BUILD_DIR/ananicy-cpp-$VERSION.tar.gz"
SRC_DIR="$BUILD_DIR/ananicy-cpp-$VERSION"

mkdir -p "$BUILD_DIR"

if [[ ! -f "$TARBALL" ]]; then
    echo "Downloading ananicy-cpp $VERSION..."
    curl -fsSL -o "$TARBALL" \
        "https://gitlab.com/ananicy-cpp/ananicy-cpp/-/archive/$VERSION/ananicy-cpp-$VERSION.tar.gz"
fi

rm -rf "$SRC_DIR"
tar -xzf "$TARBALL" -C "$BUILD_DIR"

CMAKE_ARGS=(
    -S "$SRC_DIR"
    -B "$SRC_DIR/build"
    -G Ninja
    -DCMAKE_BUILD_TYPE=Release
    -DENABLE_SYSTEMD=OFF
    -DENABLE_REGEX_SUPPORT=OFF
    -DENABLE_ANANICY_TESTS=OFF
    -DENABLE_ANANICY_BENCHMARKS=OFF
)

if cmake "${CMAKE_ARGS[@]}" -DSTATIC=ON && cmake --build "$SRC_DIR/build" -j"$(nproc)"; then
    echo "ananicy-cpp static build succeeded"
else
    echo "Static build failed, falling back to dynamic link" >&2
    rm -rf "$SRC_DIR/build"
    cmake "${CMAKE_ARGS[@]}" -DSTATIC=OFF
    cmake --build "$SRC_DIR/build" -j"$(nproc)"
fi

install -Dm755 "$SRC_DIR/build/ananicy-cpp" "$STAGE_DIR/ananicy-cpp"
echo "Staged $STAGE_DIR/ananicy-cpp"
