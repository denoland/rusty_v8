# Rusty V8 Binding

V8 Version: 15.2.124.1

[![ci](https://github.com/denoland/rusty_v8/actions/workflows/ci.yml/badge.svg)](https://github.com/denoland/rusty_v8/actions)
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

Binary builds are the default: `cargo build` will initiate a download from
github to get the static lib. To build V8 from source instead, set the
`V8_FROM_SOURCE` environment variable to `1` (`true` and `yes` also work). Any
other value, or leaving it unset, uses the prebuilt lib.

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

For every artifact (the static lib and the generated `src_binding` file), the
build script tries an ordered list of locations and uses the first one that
works:

1. `RUSTY_V8_ARCHIVE` (static lib) or `RUSTY_V8_SRC_BINDING_URL` (binding
   file), if set; either short-circuits everything else for its artifact.
2. The mirror, if `RUSTY_V8_MIRROR` is set. A plain base is expanded to
   `<base>/<tag>/<file>`; a value containing `{` placeholders is treated as a
   full URL template (see below).
3. For plain filesystem mirrors only: the flat layout `<base>/<file>`, so a
   directory of downloaded artifacts works without tag subdirectories.
4. The upstream GitHub release,
   `https://github.com/denoland/rusty_v8/releases/download/<tag>/<file>`.
   With a mirror configured this is only tried when
   `RUSTY_V8_MIRROR_FALLBACK=1` is set: a mirror fails closed by default and
   never silently reaches the network.

If every candidate fails, the build script panics with the full list of URLs
it tried.

`<tag>` defaults to `v<version>` (the crate version). Set
`RUSTY_V8_MIRROR_TAG` to override it; the value is used verbatim (no `v` is
prepended), so `RUSTY_V8_MIRROR_TAG=v152.0.0 cargo build` fetches the
artifacts of the last published release when building a checkout whose
`Cargo.toml` version is unpublished.

If the `RUSTY_V8_MIRROR` value contains a `{` placeholder, the whole value is
used as a URL template instead of a base. Supported placeholders: `{tag}`,
`{version}` (no `v` prefix), `{target}`, `{profile}` (`release`/`debug`),
`{features}` (e.g. `_ptrcomp`), and `{file}` (the full artifact filename).
For example:

    export RUSTY_V8_MIRROR=https://example.com/rusty_v8/{version}/{file}

Set `RUSTY_V8_MIRROR_FALLBACK=1` to fall back to the upstream GitHub release
when the mirror is missing an artifact, e.g. for partially populated caches.

File-based mirrors are good for using cached downloads. First, point the
environment variable to a suitable location:

    # you might want to add this to your .bashrc
    $ export RUSTY_V8_MIRROR=$HOME/.cache/rusty_v8

Then populate the cache:

```bash
#!/bin/bash

# see https://github.com/denoland/rusty_v8/releases

for REL in v152.1.0 v152.0.0; do
  mkdir -p $RUSTY_V8_MIRROR/$REL
  for FILE in \
    librusty_v8_release_x86_64-unknown-linux-gnu.a.gz \
    src_binding_release_x86_64-unknown-linux-gnu.rs \
  ; do
    if [ ! -f $RUSTY_V8_MIRROR/$REL/$FILE ]; then
      wget -O $RUSTY_V8_MIRROR/$REL/$FILE \
        https://github.com/denoland/rusty_v8/releases/download/$REL/$FILE
    fi
  done
done
```

## The `~/.cargo/.rusty_v8` download cache

Before downloading an artifact, the build script looks for a copy in the
`.rusty_v8` directory inside your Cargo home (usually `~/.cargo/.rusty_v8`).
Entries are keyed on the release tag plus the artifact filename, with every
non-alphanumeric character replaced by `_` — for example
`v152.1.0/librusty_v8_release_x86_64-unknown-linux-gnu.a.gz` becomes
`v152_1_0_librusty_v8_release_x86_64_unknown_linux_gnu_a_gz`. The escaped
full source URL, the key used by older versions of the build script, is
still checked as a fallback, so existing caches keep working.

Because the key does not include the source, a cache entry populated for one
mirror also satisfies a build configured for a different mirror (or for the
upstream release) under the same tag. If you need the archive bytes
themselves verified, pin them with `RUSTY_V8_ARCHIVE_SHA256` (below).

## The `RUSTY_V8_ARCHIVE` environment variable

Tell the build script to use a specific v8 library. This can be an URL or a
path. This is useful when you have a prebuilt archive somewhere:

```bash
export RUSTY_V8_ARCHIVE=/path/to/custom_archive.a
cargo build
```

The value may also name a directory, in which case the expected artifact
filename (e.g. `librusty_v8_release_x86_64-unknown-linux-gnu.a.gz`, gzipped
or plain) is looked up inside it. A directory is also the authoritative
source for the generated `src_binding` file: it is never fetched from the
mirror or the upstream release, so an offline setup that configured only the
directory never reaches the network. If the directory lacks the binding, a
usable binding left on disk by a previous build is reused with a warning;
otherwise the build fails:

```bash
export RUSTY_V8_ARCHIVE=/path/to/downloaded/artifacts
cargo build
```

Set `RUSTY_V8_ARCHIVE_SHA256` to the SHA-256 of the archive to pin its
content. A cached or previously downloaded archive that does not match is
re-fetched, and the build fails if the fresh download does not match either.
The pin covers the archive bytes as fetched, i.e. what `sha256sum` reports
on the `.gz` release asset (or on the plain file when the archive is not
gzipped). Independently of the pin, the build script records the SHA-256 of
every downloaded artifact and re-fetches it if the file on disk no longer
matches.

## The `RUSTY_V8_SRC_BINDING_PATH` and `RUSTY_V8_SRC_BINDING_URL` environment variables

The build also needs a generated `src_binding_..._<target>.rs` file, published
alongside the static library. `RUSTY_V8_SRC_BINDING_PATH` points the build at
a local binding file that is used directly, with no download at all.
`RUSTY_V8_SRC_BINDING_URL` instead gives a URL or path to fetch the binding
from, mirroring what `RUSTY_V8_ARCHIVE` does for the static library. If both
are set, `RUSTY_V8_SRC_BINDING_PATH` wins.

## The `RUSTY_V8_SKIP_DOWNLOAD` environment variable

Set `RUSTY_V8_SKIP_DOWNLOAD=1` to skip downloading the prebuilt static
library. The small generated binding file is still fetched, so `cargo check`
and rust-analyzer work without the (large) prebuilt artifact. Producing a
binary still requires the static library: `cargo build` fails at link time
until the crate is built again with the variable unset.

This variable takes precedence over `RUSTY_V8_ARCHIVE` and `RUSTY_V8_MIRROR`
(the static library is not fetched from anywhere, not even from a local
archive), and it has no effect on `V8_FROM_SOURCE=1` builds. If the binding
file cannot be fetched (for example, the configured mirror does not carry it)
but a previously downloaded binding exists on disk, that file is reused with
a warning instead of failing the build.

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

Additionally, building from source requires libclang 21.1+ for bindgen:

```bash
sudo apt install libclang-21-dev
export LIBCLANG_PATH=/usr/lib/llvm-21/lib
```

Linux cross-builds normally discover Clang's builtin headers and the target
libc headers from the host toolchain. For hermetic toolchains where those files
are not installed in host search paths, set the explicit bindgen inputs:

```bash
export LIBCLANG_PATH=/path/to/libclang/lib
export RUSTY_V8_BINDGEN_RESOURCE_DIR=/path/to/lib/clang/21
export RUSTY_V8_GLIBC_PREFIX=/path/to/aarch64-linux-gnu
V8_FROM_SOURCE=1 cargo build -vv --target aarch64-unknown-linux-gnu
```

`RUSTY_V8_BINDGEN_RESOURCE_DIR` takes the directory printed by
`clang -print-resource-dir`. `RUSTY_V8_GLIBC_PREFIX` takes a GNU target prefix
whose `include` child contains the target libc headers. Musl cross-builds use
`RUSTY_V8_MUSL_SYSROOT` instead; it is passed to Clang with `--sysroot`.

For Windows builds: the 64-bit toolchain needs to be used. 32-bit targets are
not supported. The default source build downloads Chromium's pinned libclang
automatically. If `$CLANG_BASE_PATH` is set to a custom LLVM installation,
`$LIBCLANG_PATH` must point to the directory containing `libclang.dll`.
The `tools/win` submodule is skipped because its standalone mirror is
unreliable, so source builds must populate its pinned debugger visualizers:

```bash
mkdir -p tools/win
curl -fL https://chromium.googlesource.com/chromium/src/tools/win/+archive/faefd1b6fa9eeb033ad6fe60368ccb9bf908cbd0.tar.gz |
  tar -xz -C tools/win
```

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

For iOS builds: cross compile from an arm64 macOS host. The simulator target
keeps the JIT; the device target (`aarch64-apple-ios`) is built jitless, since
iOS denies the JIT entitlement to non-WebKit apps (WebAssembly is also disabled
in this configuration). `build.rs` selects these settings automatically per
target — no extra GN args required:

```bash
rustup target add aarch64-apple-ios-sim  # simulator
rustup target add aarch64-apple-ios      # device (jitless)
V8_FROM_SOURCE=1 cargo build -vv --target aarch64-apple-ios-sim
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

For Linux targets, `rusty_v8` now defaults to defining
`V8_TLS_USED_IN_LIBRARY` via GN args when building from source so the produced
static archive can be linked into downstream `cdylib`/shared-library targets.
The default injected argument is:

```bash
GN_ARGS='extra_cflags=["-DV8_TLS_USED_IN_LIBRARY"]'
```

Linux prebuilt release archives published by this repository are built with
this shared-library-compatible TLS mode.

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

## Experimental Features

rusty_v8 includes experimental support for certain feature(s) that may be useful in security focused contexts but are not as well tested and do not undergo any sort of CI related testing or prebuilt archives. Due to their experimental status, these features require either ``V8_FROM_SOURCE=1`` to be set or the use of a custom-built archive of v8. 

- ``v8_enable_sandbox``: Enables v8 sandbox mode. The v8 sandbox enables improved safety while executing potentially malicious JavaScript code through the use of memory cages. Note that the v8 sandbox will allocate ~1TB of virtual memory (although this should not be an issue as many operating systems allow 128-256TB of virtual memory per process). Creating isolates with the sandbox enabled comes with API limitations and may have increased overhead. Note that enabling the V8 sandbox also implies pointer compression to be enabled as well.
