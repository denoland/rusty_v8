# Rusty V8 Binding

V8 Version: 13.8.258.22

[![ci](https://github.com/denoland/rusty_v8/workflows/ci/badge.svg?branch=main)](https://github.com/denoland/rusty_v8/actions)
[![crates](https://img.shields.io/crates/v/v8.svg)](https://crates.io/crates/v8)
[![docs](https://docs.rs/v8/badge.svg)](https://docs.rs/v8)

## Goals

1. Provide high quality Rust bindings to
   [V8's C++ API](https://cs.chromium.org/chromium/src/v8/include/v8.h). The API
   should match the original API as closely as possible.

2. Do not introduce additional call overhead. (For example, previous attempts at
   Rust V8 bindings forced the use of Persistent handles.)

3. Do not rely on a binary `libv8.a` built outside of cargo. V8 is a very large
   project (over 600,000 lines of C++) which often takes 30 minutes to compile.
   Furthermore, V8 relies on Chromium's bespoke build system (gn + ninja) which
   is not easy to use outside of Chromium. For this reason many attempts to bind
   to V8 rely on pre-built binaries that are built separately from the binding
   itself. While this is simple, it makes upgrading V8 difficult, it makes CI
   difficult, it makes producing builds with different configurations difficult,
   and it is a security concern since binary blobs can hide malicious code. For
   this reason we believe it is imperative to build V8 from source code during
   "cargo build".

4. Publish the crate on crates.io and allow docs.rs to generate documentation.
   Due to the complexity and size of V8's build, this is nontrivial. For example
   the crate size must be kept under 10 MiB in order to publish.

## Versioning

Rusty V8's major version aligns with Chrome's major version, which corresponds
to a specific V8 release. For example, Rusty V8 `129.0.0` maps to Chrome
`129.x.y.z`, which uses V8 `12.9.a.b`. While the minor and patch numbers between
Chrome and V8 may differ, Rusty V8 will follow Chrome's release schedule, with a
new major version every 4 weeks.

As a Rust crate, Rusty V8 follows semantic versioning (semver) and will not
introduce breaking changes within a major version. However, major version bumps
will occur regularly to stay in sync with Chrome's release cycle.

## Binary Build

V8 is very large and takes a long time to compile. Many users will prefer to use
a prebuilt version of V8. We publish static libs for every version of rusty v8
on [Github](https://github.com/denoland/rusty_v8/releases).

Binaries builds are turned on by default: `cargo build` will initiate a download
from github to get the static lib. To disable this build using the
`V8_FROM_SOURCE` environmental variable.

When making changes to rusty_v8 itself, it should be tested by build from
source. The CI always builds from source.

## The `V8_FORCE_DEBUG` environment variable

By default `rusty_v8` will link against release builds of `v8`, if you want to
use a debug build of `v8` set `V8_FORCE_DEBUG=true`.

We default to release builds of `v8` due to performance & CI reasons in `deno`.

## The `RUSTY_V8_MIRROR` environment variable

Tells the build script where to get binary builds from. Understands `http://`
and `https://` URLs, and file paths. The default is
https://github.com/denoland/rusty_v8/releases.

File-based mirrors are good for using cached downloads. First, point the
environment variable to a suitable location:

    # you might want to add this to your .bashrc
    $ export RUSTY_V8_MIRROR=$HOME/.cache/rusty_v8

Then populate the cache:

```bash
#!/bin/bash

# see https://github.com/denoland/rusty_v8/releases

for REL in v0.13.0 v0.12.0; do
  mkdir -p $RUSTY_V8_MIRROR/$REL
  for FILE in \
    librusty_v8_debug_x86_64-unknown-linux-gnu.a \
    librusty_v8_release_x86_64-unknown-linux-gnu.a \
  ; do
    if [ ! -f $RUSTY_V8_MIRROR/$REL/$FILE ]; then
      wget -O $RUSTY_V8_MIRROR/$REL/$FILE \
        https://github.com/denoland/rusty_v8/releases/download/$REL/$FILE
    fi
  done
done
```

## The `RUSTY_V8_ARCHIVE` environment variable

Tell the build script to use a specific v8 library. This can be an URL or a
path. This is useful when you have a prebuilt archive somewhere:

```bash
export RUSTY_V8_ARCHIVE=/path/to/custom_archive.a
cargo build
```

## Build V8 from Source

Use `V8_FROM_SOURCE=1 cargo build -vv` to build the crate completely from
source.

The build scripts require Python 3 to be available as `python3` in your `PATH`.
If you want to specify the exact binary of Python to use, you should use the
`PYTHON` environment variable.

The build also requires `curl` to be installed on your system.

For linux builds: glib-2.0 development files need to be installed such that
pkg-config can find them. On Ubuntu, run `sudo apt install libglib2.0-dev` to
install them.

For Windows builds: the 64-bit toolchain needs to be used. 32-bit targets are
not supported.

For Mac builds: You'll need Xcode and Xcode CLT installed. Recent macOS versions
will also require you to pass PYTHON=python3 because macOS no longer ships with
`python` simlinked to Python 3.

For Android builds: You'll need to cross compile from a x86_64 host to the
aarch64 or x64 android. You can use the following commands:

```bash
rustup target add aarch64-linux-android  # or x86_64-linux-android
V8_FROM_SOURCE=1 cargo build -vv --target aarch64-linux-android
# or with cross
docker build --build-arg CROSS_BASE_IMAGE=ghcr.io/cross-rs/aarch64-linux-android:0.2.5 -t cross-rusty_v8:aarch64-linux-android .
V8_FROM_SOURCE=1 cross build -vv --target aarch64-linux-android
```

The build depends on several binary tools: `gn`, `ninja` and `clang`. The tools
will automatically be downloaded, if they are not detected in the environment.

Specifying the `$GN` and `$NINJA` environmental variables can be used to skip
the download of gn and ninja. The clang download can be skipped by setting
`$CLANG_BASE_PATH` to the directory containing a `llvm`/`clang` installation. V8
is known to rely on bleeding edge features, so LLVM v8.0+ or Apple clang 11.0+
is recommended.

Arguments can be passed to `gn` by setting the `$GN_ARGS` environmental
variable.

Env vars used in when building from source: `SCCACHE`, `CCACHE`, `GN`, `NINJA`,
`CLANG_BASE_PATH`, `GN_ARGS`

## FAQ

**Building V8 takes over 30 minutes, this is too slow for me to use this crate.
What should I do?**

Install [sccache](https://github.com/mozilla/sccache) or
[ccache](https://ccache.dev/). Our build scripts will detect and use them. Set
the `$SCCACHE` or `$CCACHE` environmental variable if it's not in your path.

**What are all these random directories for like `build` and `buildtools` are
these really necessary?**

In order to build V8 from source code, we must provide a certain directory
structure with some git submodules from Chromium. We welcome any simplifications
to the code base, but this is a structure we have found after many failed
attempts that carefully balances the requirements of cargo crates and GN/Ninja.

**V8 has a very large API with hundreds of methods. Why don't you automate the
generation of this binding code?**

We have actually started down this route several times, however due to many
eccentric features of the V8 API, this has not proven successful. Therefore we
are proceeding in a brute-force fashion for now, focusing on solving our stated
goals first. We hope to auto-generate bindings in the future.

**Why are you building this?**

This is to support [the Deno project](https://deno.land/). We previously have
gotten away with a simpler high-level Rust binding to V8 called
[libdeno](https://github.com/denoland/deno/tree/32937251315493ef2c3b42dd29340e8a34501aa4/core/libdeno).
But as Deno has matured we've found ourselves continually needing access to an
increasing amount of V8's API in Rust.

**When building I get unknown argument: '-gno-inline-line-tables'**

Use `export GN_ARGS="no_inline_line_tables=false"` during build.

**My program crashes when initializing on non-main thread**

Initializing V8 on a non-main thread with the CPUs PKU feature enabled might
lead to crashes. You can work around this problem by using
`v8::new_unprotected_default_platform`.

See https://github.com/denoland/rusty_v8/issues/1381

## Download cache

The v8 archives used for linking in prebuilt mode can be cached to avoid
re-downloading archives when switching between branches that otherwise change
the current rusty_v8 version.

To populate the cache by hand, you'll need to place the files in the appropriate
location in your `.cargo` folder. Running `cargo build -v -v` will print two
lines that you can use to determine the correct file and cache location:

```
[v8 0.87.0] static lib URL: https://github.com/denoland/rusty_v8/releases/download/v0.87.0/librusty_v8_release_aarch64-apple-darwin.a.gz
[v8 0.87.0] Looking for download in '"/Users/<name>/.cargo/.rusty_v8/https___github_com_denoland_rusty_v8_releases_download_v0_87_0_librusty_v8_release_aarch64_apple_darwin_a_gz"'
```

Given the above log output, use `curl` to download the file like so:

```
curl -L https://github.com/denoland/rusty_v8/releases/download/v0.87.0/librusty_v8_release_aarch64-apple-darwin.a.gz >
  /Users/<name>/.cargo/.rusty_v8/https___github_com_denoland_rusty_v8_releases_download_v0_87_0_librusty_v8_release_aarch64_apple_darwin_a_gz
```

## For maintainers

**Cut a release**

Create a PR to bump the release version (e.g.
https://github.com/denoland/rusty_v8/pull/1415).

Create a new release/tag after the bump PR is landed. CI will publish the crate
and upload release binaries. You will need to manually upload binary archives
for M1 build.

```
$ V8_FROM_SOURCE=1 cargo build
$ V8_FROM_SOURCE=1 cargo build --release
```
