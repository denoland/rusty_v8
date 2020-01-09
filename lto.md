# Magic command

```
RUSTC_WRAPPER=sccache \
GN_ARGS="use_thin_lto=true thin_lto_enable_optimizations=false" \
RUSTFLAGS="-Clinker-plugin-lto -Clinker=$HOME/d/v8l/target/debug/build/rusty_v8-432ac2fac61ecd18/out/clang/bin/clang -Clink-arg=-fuse-ld=$HOME/d/v8l/target/debug/build/rusty_v8-432ac2fac61ecd18/out/clang/bin/ld.lld  -Clink-arg=-Wl,--thinlto-cache-dir=`pwd`/target/release/gn_out/thinlto-cache -Clink-arg=-Wl,-O5 -Clink-arg=-Wl,--icf=safe -Clink-arg=-Wl,--compress-debug-sections=zlib" \
cargo build --release -vv
```

# Patch BUILD.gn

Add somwhere where it gets picked up:

```
if (use_lld) {
  configs += [ "//build/config/compiler:thin_archive" ]
}
```

# Patch //build

In `//build/config/compiler/BUILD.gn`, replace:

```
cflags += [ "-fsplit-lto-unit" ]`
```

with

```
cflags += [ "-fno-split-lto-unit" ]`
```
