#!/bin/sh

apk add git \
    gcc \
    ninja \
    python3 \
    clang \
    g++ \
    pkgconfig \
    glib-dev \
    llvm13-dev \
    binutils-gold \
    sccache
ln -s /usr/bin/python3 /usr/bin/python

export V8_FROM_SOURCE="yes"
export CLANG_BASE_PATH="/usr"
export SCCACHE_DIR="./target/sccache"
export SCCACHE="/usr/bin/sccache"
export GN_ARGS="use_custom_libcxx=false use_lld=false v8_enable_backtrace=false v8_enable_debugging_features=false"
GN="$(pwd)/gn/out/gn"
export GN

# Bulid GN
if [ ! -d "./gn" ]; then
    git clone https://gn.googlesource.com/gn
    (
        cd gn || exit
        python3 build/gen.py
        ninja -C out
    )
fi

# Build rusty_v8 and pass through the arguments
cargo build -vv $@ --target=x86_64-unknown-linux-musl
