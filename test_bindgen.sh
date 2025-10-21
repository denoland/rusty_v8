#!/bin/bash
# Quick test script for bindgen without full V8 build
# Assumes V8 has been built at least once so clang and headers exist

set -e

# Ensure we have the clang toolchain
if [ ! -d "target/debug/clang" ]; then
    echo "Error: target/debug/clang not found. Run a full build first."
    exit 1
fi

# Find clang resource directory
CLANG_RESOURCE_DIR=$(target/debug/clang/bin/clang -print-resource-dir)/include
SDK_PATH=$(xcrun --show-sdk-path)

echo "Testing bindgen with V8's libc++ headers..."
echo "Clang resource dir: $CLANG_RESOURCE_DIR"
echo "SDK path: $SDK_PATH"

# Run bindgen with the same args as build.rs (including key defines)
# NOTE: Order matters! libc++ headers must come before clang resource dir
bindgen src/binding.hpp \
  -o /tmp/test_binding.rs \
  -- \
  -x c++ \
  -std=c++20 \
  -nostdinc++ \
  -Iv8/include \
  -I. \
  -isystembuildtools/third_party/libc++ \
  -isystemthird_party/libc++/src/include \
  -isystemthird_party/libc++abi/src/include \
  -isystem"$CLANG_RESOURCE_DIR" \
  -isysroot"$SDK_PATH" \
  -D_LIBCPP_HARDENING_MODE=_LIBCPP_HARDENING_MODE_EXTENSIVE \
  -D_LIBCPP_DISABLE_VISIBILITY_ANNOTATIONS \
  -D_LIBCXXABI_DISABLE_VISIBILITY_ANNOTATIONS

echo "Success! Bindings generated to /tmp/test_binding.rs"
