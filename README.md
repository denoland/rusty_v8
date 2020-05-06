# Rusty V8 Binding

V8 Version: 8.4.300, 2020-05-05

[![ci](https://github.com/denoland/rusty_v8/workflows/ci/badge.svg?branch=master)](https://github.com/denoland/rusty_v8/actions)
[![crates](https://img.shields.io/crates/v/rusty_v8.svg)](https://crates.io/crates/rusty_v8)
[![docs](https://docs.rs/rusty_v8/badge.svg)](https://docs.rs/rusty_v8)

## Goals

1. Provide high quality Rust bindings to [V8's C++
   API](https://cs.chromium.org/chromium/src/v8/include/v8.h). The API should
   match the original API as closely as possible

2. Do not introduce additional call overhead. (For example, previous attempts at
   Rust V8 bindings forced the use of Persistent handles.)

3. Do not rely on a binary `libv8.a` built outside of cargo. V8 is a very large
   project (over 600,000 lines of C++) which often takes 30 minutes to compile.
   Furthermore, V8 relies on Chromium's bespoke build system (gn + ninja) which is
   not easy to use outside of Chromium. For this reason many attempts to bind to V8
   rely on pre-built binaries that are built separately from the binding itself.
   While this is simple, it makes upgrading V8 difficult, it makes CI difficult, it
   makes producing builds with different configurations difficult, and it is a
   security concern since binary blobs can hide malicious code. For this reason we
   believe it is imperative to build V8 from source code during "cargo build".

4. Publish the crate on crates.io and allow docs.rs to generate documentation.
   Due to the complexity and size of V8's build, this is nontrivial. For example
   the crate size must be kept under 10 MiB in order to publish.

## Binary Build

V8 is very large and takes a long time to compile. Many users will prefer to use
a prebuilt version of V8. We publish static libs for every version of rusty v8
on [Github](https://github.com/denoland/rusty_v8/releases).

Binaries builds are turned on by default: `cargo build` will initiate a download
from github to get the static lib. To disable this build using the
`V8_FROM_SOURCE` environmental variable.

When making changes to rusty_v8 itself, it should be tested by build from
source. The CI always builds from source.

## Build V8 from Source

Use `V8_FROM_SOURCE=1 cargo build -vv` to build the crate completely from
source.

The build scripts on Python 2.7, not Python 3. [Do not open issues with us
regarding Python 3; it is a non-trivial problem that must be fixed in
Chromium.](https://bugs.chromium.org/p/chromium/issues/detail?id=942720).

For linux builds: glib-2.0 development files need to be installed such that
pkg-config can find them. On Ubuntu, run `sudo apt install libglib2.0-dev` to
install them.

For Windows builds: the 64-bit toolchain needs to be used. 32-bit targets are
not supported.

The build depends on several binary tools: `gn`, `ninja` and a recent version
of `clang` (V8 relies on bleeding edge features). Because these are not
generally available they are automatically download during the build by default.
It should be possible to opt out of the gn and ninja download by specifying the
`$GN` and `$NINJA` environmental variables. The clang download can be skipped by
providing a `$CLANG_BASE_PATH` environmental variable pointing to a recent
`llvm`/`clang` installation (currently LLVM v8.0+ or Apple clang v11.0+).
You could also pass in additional arguments to `gn` by setting the `$GN_ARGS`
environmental variable.

Env vars used in when building from source: `SCCACHE`, `GN`, `NINJA`,
`CLANG_BASE_PATH`, `GN_ARGS`

## FAQ

**Building V8 takes over 30 minutes, this is too slow for me to use this crate.
What should I do?**

Install [sccache](https://github.com/mozilla/sccache). Our build scripts will
detect and use sccache. Set the `$SCCACHE` environmental variable if it's not in
your path.

**What are all these random directories for like `build` and `buildtools` are
these really necessary?**

In order to build V8 from source code, we must provide a certain directory
structure with some git submodules from Chromium. We welcome any simplifications
to the code base, but this is a structure we have found after many failed
attempts that carefully balances the requirements of cargo crates and
GN/Ninja.

**V8 has a very large API with hundreds of methods. Why don't you automate the
generation of this binding code?**

In the limit we would like to auto-generate bindings. We have actually started
down this route several times, however due to many eccentric features of the V8
API, this has not proven successful. Therefore we are proceeding in a
brute-force fashion for now, focusing on solving our stated goals first. We hope
to auto-generate bindings in the future.

**Why are you building this?**

This is to support [the Deno project](https://deno.land/). We previously have
gotten away with a simpler high-level Rust binding to V8 called
[libdeno](https://github.com/denoland/deno/tree/32937251315493ef2c3b42dd29340e8a34501aa4/core/libdeno).
But as Deno has matured we've found ourselves continually needing access to an
increasing amount of V8's API in Rust.

**When building I get unknown argument: '-gno-inline-line-tables'**

Use `export GN_ARGS="no_inline_line_tables=false"` during build.
