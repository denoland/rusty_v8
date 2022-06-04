#!/bin/sh

apk add git
apk add gcc
apk add ninja
apk add python3
apk add clang
apk add g++
apk add pkgconfig
apk add glib-dev
apk add llvm13-dev
apk add binutils-gold
ln -s /usr/bin/python3 /usr/bin/python

export V8_FROM_SOURCE=yes
GN="$(pwd)/gn/out/gn"
export GN
export CLANG_BASE_PATH=/usr
export GN_ARGS='use_custom_libcxx=false use_lld=false v8_enable_backtrace=false v8_enable_debugging_features=false'

# Bulid GN
if [ ! -d "./gn" ]; then
    git clone https://gn.googlesource.com/gn
    (
        cd gn || exit
        python3 build/gen.py
        ninja -C out
    )
fi

# Build rusty_v8
cargo build
