// Copyright 2018-2019 the Deno authors. All rights reserved. MIT license.
use fslock::LockFile;
use miniz_oxide::MZFlush;
use miniz_oxide::MZStatus;
use miniz_oxide::StreamResult;
use miniz_oxide::inflate::stream::InflateState;
use miniz_oxide::inflate::stream::inflate;
use std::collections::HashSet;
use std::env;
use std::fs;
use std::io;
use std::io::Read;
use std::io::Seek;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::process::Stdio;
use which::which;

fn clang_resource_dir(clang_bin: &Path) -> Result<String, String> {
  let output = Command::new(clang_bin)
    .arg("-print-resource-dir")
    .output()
    .map_err(|error| {
      format!("could not run {}: {error}", clang_bin.display())
    })?;
  if !output.status.success() {
    return Err(format!(
      "{} exited with {}",
      clang_bin.display(),
      output.status
    ));
  }
  let resource_dir = String::from_utf8(output.stdout).map_err(|error| {
    format!("{} returned non-UTF-8 output: {error}", clang_bin.display())
  })?;
  let resource_dir = resource_dir.trim();
  if resource_dir.is_empty() {
    return Err(format!(
      "{} returned an empty resource directory",
      clang_bin.display()
    ));
  }
  Ok(resource_dir.to_string())
}

fn main() {
  println!("cargo:rerun-if-changed=.gn");
  println!("cargo:rerun-if-changed=BUILD.gn");
  println!("cargo:rerun-if-changed=src/binding.cc");

  // These are all the environment variables that we check. This is
  // probably more than what is needed, but missing an important
  // variable can lead to broken links when switching rusty_v8
  // versions.
  let envs = vec![
    "CCACHE",
    "CLANG_BASE_PATH",
    "CXXSTDLIB",
    "DENO_TRYBUILD",
    "DOCS_RS",
    "GN",
    "GN_ARGS",
    "HOST",
    "LIBCLANG_PATH",
    "NINJA",
    "OUT_DIR",
    "RUSTY_V8_ARCHIVE",
    "RUSTY_V8_MIRROR",
    "RUSTY_V8_MIRROR_TAG",
    "RUSTY_V8_MIRROR_STRICT",
    "RUSTY_V8_SRC_BINDING_PATH",
    "SCCACHE",
    "V8_FORCE_DEBUG",
    "V8_FROM_SOURCE",
    "PYTHON",
    "DISABLE_CLANG",
    "EXTRA_GN_ARGS",
    "PRINT_GN_ARGS",
    "CARGO_ENCODED_RUSTFLAGS",
  ];
  for env in envs {
    println!("cargo:rerun-if-env-changed={env}");
  }

  // Detect if trybuild tests are being compiled.
  let is_trybuild = env::var_os("DENO_TRYBUILD").is_some();

  // Don't build V8 if "cargo doc" is being run. This is to support docs.rs.
  let is_cargo_doc = env::var_os("DOCS_RS").is_some();

  // Don't build V8 if the rust language server (RLS) is running.
  let is_rls = env::var_os("CARGO")
    .map(PathBuf::from)
    .as_ref()
    .and_then(|p| p.file_stem())
    .and_then(|f| f.to_str())
    .is_some_and(|s| s.starts_with("rls"));

  // Early exit
  if is_cargo_doc || is_rls {
    // Don't fetch a missing binding here: docs.rs/RLS may have no network, and
    // a published crate already ships it.
    print_prebuilt_src_binding_path(false);
    return;
  }

  print_link_flags();

  // Don't attempt rebuild but link
  if is_trybuild {
    println!(
      "cargo:rustc-env=RUSTY_V8_SRC_BINDING_PATH={}",
      env::var("RUSTY_V8_SRC_BINDING_PATH").unwrap()
    );
    return;
  }

  let is_asan = if let Some(rustflags) = env::var_os("CARGO_ENCODED_RUSTFLAGS")
  {
    let rustflags = rustflags.to_string_lossy();
    rustflags.find("-Z sanitizer=address").is_some()
      || rustflags.find("-Zsanitizer=address").is_some()
  } else {
    false
  };

  // Cargo likes to run multiple build scripts at once sometimes.
  // Nothing that follows is safe to run multiple times at once,
  // because we store everything in a parent directory of OUT_DIR.
  let _lockfile = acquire_lock();

  // Build from source
  if env_bool("V8_FROM_SOURCE") {
    if is_asan && env::var_os("OPT_LEVEL").unwrap_or_default() == "0" {
      panic!(
        "v8 crate cannot be compiled with OPT_LEVEL=0 and ASAN.\nTry `[profile.dev.package.v8] opt-level = 1`.\nAborting before miscompilations cause issues."
      );
    }

    // cargo publish doesn't like pyc files.
    unsafe {
      env::set_var("PYTHONDONTWRITEBYTECODE", "1");
    }

    build_v8(is_asan);
    build_binding();

    return;
  }

  warn_on_tag_override();

  print_prebuilt_src_binding_path(true);

  download_static_lib_binaries();
}

/// Warn when the resolved mirror tag differs from `v{CARGO_PKG_VERSION}`.
///
/// `RUSTY_V8_MIRROR_TAG` pairs this checkout's Rust sources with a prebuilt
/// `librusty_v8.a` and `src_binding_*.rs` generated from a *different* V8. The
/// layout `static_assert`s that would catch a mismatch live in the old `.a` and
/// cannot fire, so a mismatch can link and then corrupt memory at runtime. The
/// feature is legitimate (e.g. building an unpublished version against the last
/// released artifacts), but the risk should be surfaced.
fn warn_on_tag_override() {
  let tag = mirror_tag();
  let expected = format!("v{}", env::var("CARGO_PKG_VERSION").unwrap());
  if tag != expected {
    println!(
      "cargo:warning=Using prebuilt V8 artifacts from tag '{tag}' but this \
       crate is version {expected}. The prebuilt library and bindings were \
       generated from a different V8; a layout mismatch may link and then \
       corrupt memory at runtime. Ensure '{tag}' is ABI-compatible with this \
       checkout."
    );
  }
}

fn acquire_lock() -> LockFile {
  let root = env::current_dir().unwrap();
  let out_dir = env::var_os("OUT_DIR").unwrap();
  let lockfilepath = root
    .join(out_dir)
    .parent()
    .unwrap()
    .parent()
    .unwrap()
    .join("v8.fslock");
  let mut lockfile = LockFile::open(&lockfilepath)
    .expect("Couldn't open lib download lockfile.");
  lockfile.lock_with_pid().expect("Couldn't get lock");
  println!("lockfile: {lockfilepath:?}");
  lockfile
}

fn build_binding() {
  // Bindgen needs Clang 21.1+ for V8's libc++ builtin type traits.
  if env::var("LIBCLANG_PATH").is_err() {
    eprintln!("Warning: LIBCLANG_PATH not set. Bindgen requires Clang 21.1+.");
    eprintln!("Set LIBCLANG_PATH to your Clang 21 installation:");
    eprintln!("  Linux:  export LIBCLANG_PATH=/usr/lib/llvm-21/lib");
    eprintln!("  macOS:  export LIBCLANG_PATH=$(brew --prefix llvm)/lib");
    eprintln!(
      "  Windows: set LIBCLANG_PATH to the directory containing libclang.dll"
    );
  }

  let output = Command::new(python())
    .arg("./tools/get_bindgen_args.py")
    .arg("--gn-out")
    .arg(build_dir().join("gn_out"))
    .output()
    .unwrap();
  let args = String::from_utf8(output.stdout).unwrap();
  let args = args.split('\0').collect::<Vec<_>>();

  // Filter out V8's custom libc++ and module args from GN, we'll add them back
  // manually with correct ordering for bindgen
  let filtered_args: Vec<&str> = args
    .iter()
    .filter(|arg| {
      !arg.starts_with("-fmodule")
        && !arg.starts_with("-fno-implicit-module")
        && !arg.starts_with("-Xclang")
        && !arg.contains("DUSE_LIBCXX_MODULES")
        && !arg.contains("-nostdinc++")
        && !arg.contains("-isystem")
        && !arg.contains("libc++")
    })
    .copied()
    .collect();

  // Use V8's custom libc++ headers (requires Clang 21.1+ libclang via
  // LIBCLANG_PATH)
  // IMPORTANT: libc++ headers must come before clang builtins
  let mut clang_args = vec![
    "-x".to_string(),
    "c++".to_string(),
    "-std=c++20".to_string(),
    "-nostdinc++".to_string(),
    "-Iv8/include".to_string(),
    "-I.".to_string(),
    "-isystembuildtools/third_party/libc++".to_string(),
    "-isystemthird_party/libc++/src/include".to_string(),
    "-isystemthird_party/libc++abi/src/include".to_string(),
  ];

  let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap();
  if target_os == "macos" {
    let output = Command::new("xcrun")
      .args(["--show-sdk-path"])
      .output()
      .unwrap();
    let sdk_path = String::from_utf8(output.stdout).unwrap();
    clang_args.push("-isysroot".to_string());
    clang_args.push(sdk_path.trim().to_string());
  } else if target_os == "linux" {
    // Add clang resource directory for builtin headers (stddef.h, etc)
    if let Ok(libclang_path) = env::var("LIBCLANG_PATH") {
      let clang_dir = PathBuf::from(&libclang_path)
        .parent()
        .unwrap()
        .to_path_buf();
      let clang_bin = clang_dir.join("bin/clang");
      if let Ok(output) =
        Command::new(clang_bin).arg("-print-resource-dir").output()
      {
        let resource_dir = String::from_utf8(output.stdout).unwrap();
        clang_args.push(format!("-isystem{}/include", resource_dir.trim()));
      }
    }
    // Parse the V8 headers against the musl sysroot. bindgen already targets
    // the musl triple (from $TARGET), so without this it looks for the target
    // arch's glibc multiarch headers, which aren't installed when cross-
    // compiling (e.g. aarch64 glibc headers on an x86_64 runner).
    let target_env = env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
    if target_env == "musl"
      && let Ok(sysroot) = env::var("RUSTY_V8_MUSL_SYSROOT")
    {
      clang_args.push(format!("--sysroot={sysroot}"));
    }
  } else if target_os == "windows" {
    // libclang otherwise discovers the runner's system Clang resource
    // directory, which may not match the pinned Chromium libclang.
    if let Ok(libclang_path) = env::var("LIBCLANG_PATH") {
      let libclang_path = PathBuf::from(libclang_path);
      if let Some(clang_dir) = libclang_path.parent() {
        let clang_bin = clang_dir.join("bin/clang-cl.exe");
        match clang_resource_dir(&clang_bin) {
          Ok(resource_dir) => {
            println!("clang_resource_dir (from LIBCLANG_PATH) {resource_dir}");
            clang_args.push(format!("-resource-dir={resource_dir}"));
          }
          Err(error) => println!(
            "cargo:warning=Skipping Clang resource directory override: {error}"
          ),
        }
      } else {
        println!(
          "cargo:warning=Skipping Clang resource directory override: \
           LIBCLANG_PATH has no parent directory"
        );
      }
    }
  } else if target_os == "ios" {
    // iOS: point bindgen at the iOS (device) or iOS-simulator SDK and set the
    // matching clang target triple so the V8 headers parse correctly.
    let target_triple = env::var("TARGET").unwrap();
    let is_sim = target_triple.ends_with("-sim")
      || target_triple.starts_with("x86_64-apple-ios");
    let sdk = if is_sim {
      "iphonesimulator"
    } else {
      "iphoneos"
    };
    let output = Command::new("xcrun")
      .args(["--sdk", sdk, "--show-sdk-path"])
      .output()
      .unwrap();
    let sdk_path = String::from_utf8(output.stdout).unwrap();
    clang_args.push("-isysroot".to_string());
    clang_args.push(sdk_path.trim().to_string());
    let clang_target = if is_sim {
      "arm64-apple-ios-simulator"
    } else {
      "arm64-apple-ios"
    };
    clang_args.push(format!("--target={clang_target}"));
  }

  let bindings = bindgen::Builder::default()
    .header("src/binding.hpp")
    .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
    .clang_args(clang_args)
    .clang_args(filtered_args)
    .generate_cstr(true)
    .rustified_enum(".*UseCounterFeature")
    .rustified_enum(".*ModuleImportPhase")
    .rustified_enum(".*Intercepted")
    .bitfield_enum(".*GCType")
    .bitfield_enum(".*GCCallbackFlags")
    .allowlist_item("v8__.*")
    .allowlist_item("cppgc__.*")
    .allowlist_item("RustObj")
    .allowlist_item("memory_span_t")
    .allowlist_item("const_memory_span_t")
    .allowlist_item("ExternalConstOneByteStringResource")
    .blocklist_item("cppgc.*Visitor")
    .blocklist_item("RustObj.*Trace")
    .generate()
    .expect("Unable to generate bindings");

  let out_path = build_dir().join("gn_out").join("src_binding.rs");
  println!(
    "cargo:rustc-env=RUSTY_V8_SRC_BINDING_PATH={}",
    out_path.display()
  );
  bindings
    .write_to_file(out_path)
    .expect("Couldn't write bindings!");
}

fn build_v8(is_asan: bool) {
  unsafe {
    env::set_var("DEPOT_TOOLS_WIN_TOOLCHAIN", "0");
  }

  if need_gn_ninja_download() {
    download_ninja_gn_binaries();
  }

  download_rust_toolchain();

  // `#[cfg(...)]` attributes don't work as expected from build.rs -- they refer to the configuration
  // of the host system which the build.rs script will be running on. In short, `cfg!(target_<os/arch>)`
  // is actually the host os/arch instead of target os/arch while cross compiling. Instead, Environment variables
  // are the officially approach to get the target os/arch in build.rs.
  let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap();
  let target_arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap();
  // On windows, rustc cannot link with a V8 debug build.
  let mut gn_args = if is_debug() && target_os != "windows" {
    // Note: When building for Android aarch64-qemu, use release instead of debug.
    vec!["is_debug=true".to_string()]
  } else {
    vec!["is_debug=false".to_string()]
  };
  if is_asan {
    gn_args.push("is_asan=true".to_string());
  }
  gn_args.push(format!(
    "use_custom_libcxx={}",
    env::var("CARGO_FEATURE_USE_CUSTOM_LIBCXX").is_ok()
  ));

  let extra_args = {
    if env::var("CARGO_FEATURE_V8_ENABLE_SANDBOX").is_ok() {
      vec![
        // Enable pointer compression (along with its dependencies)
        "v8_enable_sandbox=true",
        "v8_enable_external_code_space=true", // Needed for sandbox
        "v8_enable_pointer_compression=true",
        // Note that sandbox requires shared_ro_heap and verify_heap
        // to be true/default
      ]
    } else {
      let mut opts = vec![
        // Disable sandbox
        "v8_enable_sandbox=false",
      ];

      if env::var("CARGO_FEATURE_V8_ENABLE_POINTER_COMPRESSION").is_ok() {
        opts.push("v8_enable_pointer_compression=true");
      } else {
        opts.push("v8_enable_pointer_compression=false");
      }

      opts
    }
  };

  for arg in extra_args {
    gn_args.push(arg.to_string());
  }

  gn_args.push(format!(
    "v8_enable_v8_checks={}",
    env::var("CARGO_FEATURE_V8_ENABLE_V8_CHECKS").is_ok()
  ));

  // Fix GN's host_cpu detection when using x86_64 bins on Apple Silicon
  if cfg!(target_os = "macos") && cfg!(target_arch = "aarch64") {
    gn_args.push("host_cpu=\"arm64\"".to_string());
  }

  if env::var_os("DISABLE_CLANG").is_some() {
    gn_args.push("is_clang=false".into());
    // -gline-tables-only is Clang-only
    gn_args.push("line_tables_only=false".into());
  } else if let Some(clang_base_path) = find_compatible_system_clang() {
    println!("clang_base_path (system): {}", clang_base_path.display());
    gn_args.push(format!("clang_base_path={clang_base_path:?}"));
    gn_args.push("treat_warnings_as_errors=false".to_string());
  } else {
    println!("using Chromium's clang");
    let clang_base_path = clang_download();
    gn_args.push(format!("clang_base_path={clang_base_path:?}"));

    if target_os == "android" && target_arch == "aarch64" {
      gn_args.push("treat_warnings_as_errors=false".to_string());
    }
  }

  if let Some(p) = env::var_os("SCCACHE") {
    cc_wrapper(&mut gn_args, Path::new(&p));
  } else if let Ok(p) = which("sccache") {
    cc_wrapper(&mut gn_args, &p);
  } else if let Some(p) = env::var_os("CCACHE") {
    cc_wrapper(&mut gn_args, Path::new(&p));
  } else if let Ok(p) = which("ccache") {
    cc_wrapper(&mut gn_args, &p);
  } else {
    println!("cargo:warning=Not using sccache or ccache");
  }

  // Forward caller-provided GN args verbatim.
  let gn_args_env = env::var("GN_ARGS").unwrap_or_default();
  if !gn_args_env.trim().is_empty() {
    gn_args.push(gn_args_env.clone());
  }

  // rusty_v8 ships a single static archive that downstream crates may link
  // into a shared library (cdylib). On Linux, V8's default "local-exec" TLS
  // model emits R_X86_64_TPOFF32 relocations against thread-locals such as
  // `g_current_isolate_`, which lld refuses to place in a `-shared` object,
  // so any cdylib that links the archive fails to link. Enabling this V8 GN
  // arg routes the `V8_TLS_USED_IN_LIBRARY` define into both `internal_config`
  // and the `features` config, switching V8 to the shared-library-safe TLS
  // path (local-dynamic model + out-of-line accessor) uniformly across V8's
  // own sources and rusty_v8's bindings.
  if target_os == "linux"
    && !gn_args_env.contains("v8_monolithic_for_shared_library")
  {
    gn_args.push("v8_monolithic_for_shared_library=true".to_string());
  }
  // cross-compilation setup
  if target_arch == "aarch64" {
    gn_args.push(r#"target_cpu="arm64""#.to_string());
    if target_os == "linux" {
      gn_args.push("use_sysroot=true".to_string());
      maybe_install_sysroot("arm64");
      maybe_install_sysroot("amd64");
    }
  }
  if target_arch == "arm" {
    gn_args.push(r#"target_cpu="arm""#.to_string());
    gn_args.push(r#"v8_target_cpu="arm""#.to_string());
    gn_args.push("use_sysroot=true".to_string());
    maybe_install_sysroot("i386");
    maybe_install_sysroot("arm");
  }
  if target_arch == "riscv64" {
    gn_args.push(r#"target_cpu="riscv64""#.to_string());
    // Cross compiling needs to set v8_target_cpu
    gn_args.push(r#"v8_target_cpu="riscv64""#.to_string());
    if target_os == "linux" {
      gn_args.push("use_sysroot=true".to_string());
      maybe_install_sysroot("riscv64");
      maybe_install_sysroot("amd64");
    }
  }

  // musl libc. V8's build targets glibc by default; the vendored build config
  // grows a target-scoped `use_musl` arg (see //build/config/rust.gni,
  // sysroot.gni, c++/BUILD.gn, toolchain/*). The final librusty_v8.a is a
  // static archive of musl-compiled objects (never linked here), so the target
  // toolchain needs only musl headers -- the executable build tools (torque,
  // mksnapshot, code generators) are built with a separate glibc toolchain so
  // they link and run on the (glibc) build host.
  let target_env = env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
  if target_env == "musl" && target_os == "linux" {
    gn_args.push("use_musl=true".to_string());
    // V8-as-a-library has no glib dependency; skip it so a musl target_sysroot
    // doesn't send pkg-config looking for glib inside the sysroot.
    gn_args.push("use_glib=false".to_string());
    // Build libstd + V8's internal Rust crates from source for the musl triple;
    // V8's vendored Rust toolchain only ships a glibc host std.
    gn_args.push("rust_prebuilt_stdlib=false".to_string());
    // Some V8 sources have glibc-only code paths (e.g. execinfo-based
    // backtraces in stack_trace_posix.cc) whose helpers are unused on musl,
    // tripping -Werror,-Wunused-const-variable. Like the iOS/Android cross
    // builds, don't treat warnings as errors here.
    gn_args.push("treat_warnings_as_errors=false".to_string());

    match target_arch.as_str() {
      "x86_64" => {
        // Host cpu == target cpu, so V8 would build the executable build tools
        // with the (musl) default toolchain. Force the host and snapshot
        // toolchains to a dedicated glibc toolchain so those tools stay glibc.
        let glibc = "//build/toolchain/linux:clang_x64_glibc";
        gn_args.push(format!("host_toolchain=\"{glibc}\""));
        gn_args.push(format!("v8_snapshot_toolchain=\"{glibc}\""));
        // That glibc toolchain builds against the amd64 sysroot, same as the
        // host side of the aarch64/riscv64 cross builds.
        maybe_install_sysroot("amd64");
      }
      "aarch64" => {
        // Cross build (x64 host -> arm64 target). The host (clang_x64) and
        // snapshot (clang_x64_v8_arm64) toolchains are already non-default, so
        // the target-scoped `use_musl` guard keeps them glibc automatically --
        // no toolchain overrides needed. target_cpu and the amd64/arm64
        // sysroots are set by the aarch64 cross-compilation block above.
      }
      other => panic!(
        "musl builds are only supported for x86_64 and aarch64 (got {other})"
      ),
    }

    // Cross-compiling on a glibc host needs a musl sysroot for the target's
    // headers/libs. Native musl builds (e.g. on Alpine) can leave this unset.
    if let Ok(sysroot) = env::var("RUSTY_V8_MUSL_SYSROOT") {
      gn_args.push(format!("target_sysroot=\"{sysroot}\""));
    }
  }

  let target_triple = env::var("TARGET").unwrap();
  // check if the target triple describes a non-native environment
  if target_triple != env::var("HOST").unwrap() && target_os == "android" {
    let arch = if target_arch == "x86_64" {
      "x64"
    } else if target_arch == "aarch64" {
      "arm64"
    } else {
      "unknown"
    };
    if target_arch == "x86_64" {
      maybe_install_sysroot("amd64");
    }
    gn_args.push(format!(r#"v8_target_cpu="{arch}""#).to_string());
    gn_args.push(format!(r#"target_cpu="{arch}""#).to_string());
    gn_args.push(r#"target_os="android""#.to_string());
    gn_args.push("treat_warnings_as_errors=false".to_string());
    gn_args.push("use_sysroot=true".to_string());

    // NDK 23 and above removes libgcc entirely.
    // https://github.com/rust-lang/rust/pull/85806
    if !Path::new("./third_party/android_ndk/toolchains/llvm/prebuilt/linux-x86_64/bin/aarch64-linux-android24-clang++").exists() {
        assert!(Command::new("curl")
        .arg("-L")
        .arg("-o").arg("./third_party/android-ndk-r26c-linux.zip")
        .arg("https://dl.google.com/android/repository/android-ndk-r26c-linux.zip")
        .status()
        .unwrap()
        .success());

        assert!(Command::new("unzip")
        .arg("-d").arg("./third_party/")
        .arg("-o")
        .arg("-q")
        .arg("./third_party/android-ndk-r26c-linux.zip")
        .status()
        .unwrap()
        .success());

        fs::rename("./third_party/android-ndk-r26c", "./third_party/android_ndk").unwrap();
        fs::remove_file("./third_party/android-ndk-r26c-linux.zip").unwrap();
      }
    static CHROMIUM_URI: &str = "https://chromium.googlesource.com";
    maybe_clone_repo(
      "./third_party/android_platform",
      &format!("{CHROMIUM_URI}/chromium/src/third_party/android_platform.git",),
    );
    maybe_clone_repo(
      "./third_party/catapult",
      &format!("{CHROMIUM_URI}/catapult.git"),
    );
  }

  // iOS / iOS-simulator. iOS denies the JIT entitlement to non-WebKit apps, so
  // a device build must be jitless -- which in turn requires V8's optimizing
  // tiers (Sparkplug/Maglev/Turbofan) and WebAssembly to be disabled. The
  // simulator runs on the host and could keep the JIT, but WebAssembly is
  // disabled there too because Torque can't generate the Wasm builtins in this
  // configuration. `target_cpu="arm64"` is already set above for aarch64.
  // Pass an explicit `target_os="ios"` in GN_ARGS to fully override this.
  if target_os == "ios" && !gn_args_env.contains(r#"target_os="ios""#) {
    let is_sim = target_triple.ends_with("-sim")
      || target_triple.starts_with("x86_64-apple-ios");
    gn_args.push(r#"target_os="ios""#.to_string());
    gn_args.push(format!(
      r#"target_environment="{}""#,
      if is_sim { "simulator" } else { "device" }
    ));
    gn_args.push(r#"ios_deployment_target="14.0""#.to_string());
    gn_args.push("ios_enable_code_signing=false".to_string());
    gn_args.push("treat_warnings_as_errors=false".to_string());
    gn_args.push("v8_enable_webassembly=false".to_string());
    if !is_sim {
      // Device: no JIT permitted -> jitless build, all tiers off.
      gn_args.push("v8_jitless=true".to_string());
      gn_args.push("v8_enable_sparkplug=false".to_string());
      gn_args.push("v8_enable_maglev=false".to_string());
      gn_args.push("v8_enable_turbofan=false".to_string());
    }
  }

  if target_triple.starts_with("i686-") {
    gn_args.push(r#"target_cpu="x86""#.to_string());
  }

  let gn_out = run_gn_gen(&gn_args);
  assert!(gn_out.exists());
  assert!(gn_out.join("args.gn").exists());
  if env_bool("PRINT_GN_ARGS") {
    print_gn_args(&gn_out);
  }
  build("rusty_v8", None);
}

fn print_gn_args(gn_out_dir: &Path) {
  assert!(
    Command::new(gn())
      .arg(format!("--script-executable={}", python()))
      .arg("args")
      .arg(gn_out_dir)
      .arg("--list")
      .status()
      .unwrap()
      .success()
  );
}

fn maybe_clone_repo(dest: &str, repo: &str) {
  if !Path::new(&dest).exists() {
    assert!(
      Command::new("git")
        .arg("clone")
        .arg("--depth=1")
        .arg(repo)
        .arg(dest)
        .status()
        .unwrap()
        .success()
    );
  }
}

fn maybe_install_sysroot(arch: &str) {
  let sysroot_path = format!("build/linux/debian_sid_{arch}-sysroot");
  if !PathBuf::from(sysroot_path).is_dir() {
    assert!(
      Command::new(python())
        .arg("./build/linux/sysroot_scripts/install-sysroot.py")
        .arg(format!("--arch={arch}"))
        .status()
        .unwrap()
        .success()
    );
  }
}

fn download_ninja_gn_binaries() {
  let target_dir = build_dir().join("ninja_gn_binaries");

  let gn = target_dir.join("gn").join("gn");
  let ninja = target_dir.join("ninja").join("ninja");
  #[cfg(windows)]
  let gn = gn.with_extension("exe");
  #[cfg(windows)]
  let ninja = ninja.with_extension("exe");

  if !gn.exists() || !ninja.exists() {
    assert!(
      Command::new(python())
        .arg("./tools/ninja_gn_binaries.py")
        .arg("--dir")
        .arg(&target_dir)
        .status()
        .unwrap()
        .success()
    );
  }
  assert!(gn.exists());
  assert!(ninja.exists());
  unsafe {
    env::set_var("GN", gn);
  }
  if env::var("NINJA").is_err() {
    unsafe {
      env::set_var("NINJA", ninja);
    }
  }
}

fn download_rust_toolchain() {
  assert!(
    Command::new(python())
      .arg("./tools/rust_toolchain.py")
      .status()
      .unwrap()
      .success()
  );
}

fn prebuilt_profile() -> &'static str {
  let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap();
  // Use v8 in release mode unless $V8_FORCE_DEBUG=true
  // Note: we always use the release build on windows.
  if target_os != "windows" && env_bool("V8_FORCE_DEBUG") {
    "debug"
  } else {
    "release"
  }
}

fn prebuilt_features_suffix() -> String {
  let mut features = String::new();
  if env::var("CARGO_FEATURE_V8_ENABLE_POINTER_COMPRESSION").is_ok() {
    features.push_str("_ptrcomp");
  }
  if env::var("CARGO_FEATURE_V8_ENABLE_SANDBOX").is_ok() {
    features.push_str("_sandbox");
  }
  features
}

fn static_lib_name(suffix: &str) -> String {
  let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap();
  if target_os == "windows" {
    format!("rusty_v8{suffix}.lib")
  } else {
    format!("librusty_v8{suffix}.a")
  }
}

const DEFAULT_MIRROR_BASE: &str =
  "https://github.com/denoland/rusty_v8/releases/download";

fn is_http_url(s: &str) -> bool {
  let lower = s.to_ascii_lowercase();
  lower.starts_with("http:") || lower.starts_with("https:")
}

/// The release tag (directory segment) that artifacts are looked up under.
///
/// `RUSTY_V8_MIRROR_TAG` overrides it verbatim (no `v` is prepended), so
/// non-tag-shaped directories such as `nightly` or `pr-1234` work. The default
/// is `v{CARGO_PKG_VERSION}`.
fn mirror_tag() -> String {
  match env::var("RUSTY_V8_MIRROR_TAG") {
    Ok(tag) => tag,
    Err(_) => format!("v{}", env::var("CARGO_PKG_VERSION").unwrap()),
  }
}

/// Expand a templated `RUSTY_V8_MIRROR` value. See [`artifact_urls`].
///
/// The substitution values are passed in rather than read from the environment
/// so this stays a pure function that can be unit tested.
fn expand_mirror_template(
  template: &str,
  tag: &str,
  version: &str,
  target: &str,
  profile: &str,
  features: &str,
  file: &str,
) -> String {
  template
    .replace("{tag}", tag)
    .replace("{version}", version)
    .replace("{target}", target)
    .replace("{profile}", profile)
    .replace("{features}", features)
    .replace("{file}", file)
}

/// A single artifact location to try.
#[derive(Debug)]
struct Candidate {
  /// An `http(s)://` URL or a filesystem path.
  url: String,
  /// Whether this is a best-effort *cache* (a flat directory of downloaded
  /// artifacts, or the local `~/.cargo/.rusty_v8` cache) rather than a
  /// deliberately chosen source. A corrupt artifact from a cache falls through
  /// to the next candidate; a corrupt artifact from a deliberate source (a
  /// pinned mirror, upstream) aborts the build.
  is_cache: bool,
}

/// Pure candidate-list builder. All inputs are explicit (nothing is read from
/// the environment) so the ordering matrix can be unit tested. See
/// [`artifact_urls`] for the env-reading wrapper.
///
/// Resolution order (see the `RUSTY_V8_MIRROR` section in the README):
///
/// 1. `mirror`, if set. When it contains a `{` placeholder it is treated as a
///    full template; otherwise it is a base and the historical
///    `<base>/<tag>/<file>` layout is used, plus a flat `<base>/<file>` cache
///    layout for filesystem mirrors.
/// 2. The upstream default releases base, unless `strict` is set (hermetic
///    builds that must never reach the network).
///
/// `strict` is honored unconditionally: with no mirror it yields an empty list,
/// which the caller reports as "strict mode set but nowhere to fetch from"
/// rather than silently reaching the network.
#[allow(clippy::too_many_arguments)]
fn artifact_urls_from(
  mirror: Option<&str>,
  tag: &str,
  strict: bool,
  version: &str,
  target: &str,
  profile: &str,
  features: &str,
  file: &str,
) -> Vec<Candidate> {
  let mut candidates = Vec::new();

  if let Some(mirror) = mirror {
    if mirror.contains('{') {
      // The mirror is a template; expand placeholders and use it verbatim.
      candidates.push(Candidate {
        url: expand_mirror_template(
          mirror, tag, version, target, profile, features, file,
        ),
        is_cache: false,
      });
    } else {
      // Historical layout: `<base>/<tag>/<file>` — a deliberate source.
      candidates.push(Candidate {
        url: format!("{mirror}/{tag}/{file}"),
        is_cache: false,
      });
      // Flat layout `<base>/<file>` for a plain directory of downloaded
      // artifacts used as a cache. Only meaningful for filesystem mirrors.
      if !is_http_url(mirror) {
        candidates.push(Candidate {
          url: format!("{mirror}/{file}"),
          is_cache: true,
        });
      }
    }
  }

  // Fall back to the upstream releases unless strict mode forbids the network.
  if !strict {
    candidates.push(Candidate {
      url: format!("{DEFAULT_MIRROR_BASE}/{tag}/{file}"),
      is_cache: false,
    });
  }

  candidates
}

/// Ordered list of candidate locations to try for artifact `file` (e.g.
/// `librusty_v8_release_<target>.a.gz` or `src_binding_release_<target>.rs`).
fn artifact_urls(file: &str) -> Vec<Candidate> {
  let version = env::var("CARGO_PKG_VERSION").unwrap();
  let target = env::var("TARGET").unwrap();
  artifact_urls_from(
    env::var("RUSTY_V8_MIRROR").ok().as_deref(),
    &mirror_tag(),
    env_bool("RUSTY_V8_MIRROR_STRICT"),
    &version,
    &target,
    prebuilt_profile(),
    &prebuilt_features_suffix(),
    file,
  )
}

/// Candidate URLs for the prebuilt static library archive.
fn static_lib_urls() -> Vec<Candidate> {
  // An explicit archive short-circuits everything (static library only).
  if let Ok(custom_archive) = env::var("RUSTY_V8_ARCHIVE") {
    return vec![Candidate {
      url: custom_archive,
      is_cache: false,
    }];
  }
  let target = env::var("TARGET").unwrap();
  let profile = prebuilt_profile();
  let features = prebuilt_features_suffix();
  let file = format!(
    "{}.gz",
    static_lib_name(&format!("{features}_{profile}_{target}"))
  );
  artifact_urls(&file)
}

fn static_lib_path() -> PathBuf {
  static_lib_dir().join(static_lib_name(""))
}

fn static_checksum_path(path: &Path) -> PathBuf {
  let mut path = path.to_path_buf();
  path.set_extension("sum");
  path
}

fn static_lib_dir() -> PathBuf {
  build_dir().join("gn_out").join("obj")
}

fn build_dir() -> PathBuf {
  let cwd = env::current_dir().unwrap();

  // target/debug//build/rusty_v8-d9e5a424d4f96994/out/
  let out_dir = env::var_os("OUT_DIR").expect(
    "The 'OUT_DIR' environment is not set (it should be something like \
     'target/debug/rusty_v8-{hash}').",
  );
  let out_dir_abs = cwd.join(out_dir);

  // This would be `target/debug` or `target/release`
  out_dir_abs
    .parent()
    .unwrap()
    .parent()
    .unwrap()
    .parent()
    .unwrap()
    .to_path_buf()
}

fn replace_non_alphanumeric(url: &str) -> String {
  url
    .chars()
    .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
    .collect()
}

/// The outcome of trying a single artifact candidate.
enum FetchError {
  /// The candidate is absent (missing file, failed download). Fall through to
  /// the next candidate.
  Miss(String),
  /// The candidate was found but could not be used (corrupt archive, disk
  /// full, unwritable destination). Abort — falling back would silently ignore
  /// a broken but deliberately chosen source, e.g. a pinned mirror.
  Fatal(String),
}

/// Fetch each candidate in `urls` into `filename`, stopping at the first that
/// succeeds. A [`FetchError::Fatal`] aborts immediately. If every candidate is
/// a [`FetchError::Miss`], panic with the full list of what was tried and the
/// available escape hatches. `what` names the artifact for the diagnostics
/// (e.g. "the V8 static library").
fn download_artifact(urls: &[Candidate], filename: &Path, what: &str) {
  if urls.is_empty() {
    // Only reachable with RUSTY_V8_MIRROR_STRICT set and no mirror configured:
    // strict suppresses upstream, leaving nothing to try.
    panic!(
      "No candidate locations to fetch {what} from. RUSTY_V8_MIRROR_STRICT is \
       set but RUSTY_V8_MIRROR is not, so the upstream fallback is disabled and \
       there is nowhere to fetch from. Set RUSTY_V8_MIRROR (and, for the \
       bindings, optionally RUSTY_V8_SRC_BINDING_PATH), or unset \
       RUSTY_V8_MIRROR_STRICT."
    );
  }

  let mut failures = Vec::new();
  for candidate in urls {
    let url = &candidate.url;
    println!("Trying artifact candidate: {url}");
    match try_download_file(candidate, filename) {
      Ok(()) => return,
      Err(FetchError::Fatal(reason)) => {
        panic!(
          "Found {what} at {url} but could not use it: {reason}\n\
           This source was chosen deliberately (a pinned mirror or the upstream \
           release), so the build aborts rather than silently using a different \
           one."
        );
      }
      Err(FetchError::Miss(reason)) => {
        println!("  skipped {url}: {reason}");
        failures.push(format!("  - {url}\n      {reason}"));
      }
    }
  }
  panic!(
    "Failed to obtain {what}.\n\
     Tried the following candidates, in order:\n{}\n\n\
     This is usually because no prebuilt archive is published for your target \
     or version. Ways to fix this:\n\
     \x20 RUSTY_V8_ARCHIVE=<path|url>  use a specific static library archive\n\
     \x20 RUSTY_V8_MIRROR=<base|template>  fetch artifacts from another location\n\
     \x20 RUSTY_V8_MIRROR_TAG=<tag>  override the release tag/directory \
     (default v<crate version>)\n\
     \x20 V8_FROM_SOURCE=1  build V8 from source instead of downloading",
    failures.join("\n"),
  );
}

/// Copy/decompress `src` into `filename`, demoting a fatal failure to a
/// [`FetchError::Miss`] when the candidate is a best-effort cache — a corrupt
/// cache entry should fall through, not abort.
fn use_archive(
  candidate: &Candidate,
  src: &str,
  filename: &Path,
) -> Result<(), FetchError> {
  match copy_archive(src, filename) {
    Err(FetchError::Fatal(reason)) if candidate.is_cache => {
      Err(FetchError::Miss(reason))
    }
    other => other,
  }
}

/// Try to fetch a single `candidate` (an `http(s)://` URL or a filesystem path)
/// into `filename`. A [`FetchError::Miss`] means the candidate was absent and
/// the caller should fall through; a [`FetchError::Fatal`] means it was found
/// but unusable and the build should abort.
fn try_download_file(
  candidate: &Candidate,
  filename: &Path,
) -> Result<(), FetchError> {
  let url = &candidate.url;

  // Checksum (i.e: url) to avoid re-downloading/re-copying. Compare against the
  // URL being requested (not a fixed one) so this works for every artifact and
  // for filesystem mirrors as well as HTTP downloads.
  if filename.exists()
    && let Ok(c) = fs::read_to_string(static_checksum_path(filename))
    && c == *url
  {
    println!("Already downloaded {url}");
    return Ok(());
  }

  if !is_http_url(url) {
    // A filesystem path: an explicit archive, a file mirror, or a flat cache.
    if !Path::new(url).exists() {
      return Err(FetchError::Miss(format!("file not found: {url}")));
    }
    use_archive(candidate, url, filename)?;
    write_checksum(filename, url)?;
    return Ok(());
  }

  // If there is a `.cargo/.rusty_v8/<escaped URL>` file, use that instead of
  // downloading. This is a local cache, so a corrupt entry is evicted and we
  // fall through to a fresh download rather than aborting.
  if let Ok(mut path) = home::cargo_home() {
    path = path.join(".rusty_v8").join(replace_non_alphanumeric(url));
    println!("Looking for download in '{path:?}'");
    if path.exists() {
      match copy_archive(&path.to_string_lossy(), filename) {
        Ok(()) => {
          write_checksum(filename, url)?;
          return Ok(());
        }
        Err(FetchError::Fatal(reason)) => {
          println!(
            "Cached download at {} is unusable ({reason}); removing and \
             re-downloading",
            path.display()
          );
          let _ = fs::remove_file(&path);
        }
        Err(e) => return Err(e),
      }
    }
  }

  let tmpfile = filename.with_extension("tmp");
  if tmpfile.exists() {
    println!("Deleting old tmpfile {}", tmpfile.display());
    fs::remove_file(&tmpfile).map_err(|e| {
      FetchError::Fatal(format!("could not remove {}: {e}", tmpfile.display()))
    })?;
  }

  // Try downloading with deno first, then python, then curl.
  println!("Downloading {url}");
  let downloaded = download_with_deno(url, &tmpfile)
    || download_with_python(url, &tmpfile)
    || download_with_curl(url, &tmpfile);

  // A failure here means the artifact could not be fetched from this URL (404,
  // network error, no downloader available), which is a miss — the next
  // candidate may still have it. Don't leave a partial tmpfile behind.
  if !downloaded || !tmpfile.exists() {
    let _ = fs::remove_file(&tmpfile);
    return Err(FetchError::Miss(format!(
      "could not download {url} (tried deno, python, and curl)"
    )));
  }

  // Move the file into place, then record the checksum only after the copy
  // succeeds so a failed/interrupted copy can't leave a `.sum` that points at a
  // truncated artifact.
  use_archive(candidate, &tmpfile.to_string_lossy(), filename)?;
  write_checksum(filename, url)?;
  let _ = fs::remove_file(&tmpfile);

  Ok(())
}

/// Download `url` to `tmpfile` with Deno. Returns whether it succeeded.
fn download_with_deno(url: &str, tmpfile: &Path) -> bool {
  let Ok(deno) = which("deno") else {
    return false;
  };
  println!("Trying with Deno...");
  Command::new(deno)
    .arg("eval")
    .arg(
      "const [url, path] = Deno.args; \
       const resp = await fetch(url); \
       if (!resp.ok) Deno.exit(1); \
       const file = await Deno.open(path, { write: true, create: true }); \
       await resp.body.pipeTo(file.writable);",
    )
    // Note: `deno eval` runs with all permissions implicitly granted and does
    // not accept `--allow-*` flags, so passing them here makes `deno eval`
    // error out ("unexpected argument '--allow-net'") and the download
    // silently falls back to Python/curl.
    .arg("--")
    .arg(url)
    .arg(tmpfile)
    .status()
    .is_ok_and(|s| s.success())
}

/// Download `url` to `tmpfile` with Python. Returns whether it succeeded.
/// Python is a V8 build dependency, so this saves us from adding a Rust HTTP
/// client dependency; it is not guaranteed to be present for prebuilt builds.
fn download_with_python(url: &str, tmpfile: &Path) -> bool {
  println!("Trying with Python...");
  Command::new(python())
    .arg("./tools/download_file.py")
    .arg("--url")
    .arg(url)
    .arg("--filename")
    .arg(tmpfile)
    .status()
    .is_ok_and(|s| s.success())
}

/// Download `url` to `tmpfile` with curl. Returns whether it succeeded. A
/// missing `curl` binary is just a `false`, not a panic.
fn download_with_curl(url: &str, tmpfile: &Path) -> bool {
  println!("Trying with curl...");
  Command::new("curl")
    .arg("-L")
    .arg("-f")
    .arg("-s")
    .arg("-o")
    .arg(tmpfile)
    .arg(url)
    .status()
    .is_ok_and(|s| s.success())
}

/// Record the source `url` of the artifact now sitting at `filename`, so a
/// later build can skip re-fetching it. Written only after the artifact is
/// fully in place.
fn write_checksum(filename: &Path, url: &str) -> Result<(), FetchError> {
  fs::write(static_checksum_path(filename), url).map_err(|e| {
    FetchError::Fatal(format!(
      "could not write checksum for {}: {e}",
      filename.display()
    ))
  })
}

fn download_static_lib_binaries() {
  let urls = static_lib_urls();
  println!("static lib candidates: {urls:?}");

  let dir = static_lib_dir();
  fs::create_dir_all(&dir).unwrap();
  println!("cargo:rustc-link-search={}", dir.display());

  download_artifact(&urls, &static_lib_path(), "the V8 static library");
}

fn decompress_to_writer<R, W>(input: &mut R, output: &mut W) -> io::Result<()>
where
  R: Read,
  W: Write,
{
  let mut inflate_state = InflateState::default();
  let mut input_buffer = [0; 16 * 1024];
  let mut output_buffer = [0; 16 * 1024];
  let mut input_offset = 0;

  // Skip the gzip header
  gzip_header::read_gz_header(input)?;

  loop {
    let bytes_read = input.read(&mut input_buffer[input_offset..])?;
    let bytes_avail = input_offset + bytes_read;

    let StreamResult {
      bytes_consumed,
      bytes_written,
      status,
    } = inflate(
      &mut inflate_state,
      &input_buffer[..bytes_avail],
      &mut output_buffer,
      MZFlush::None,
    );

    if status != Ok(MZStatus::Ok) && status != Ok(MZStatus::StreamEnd) {
      return Err(io::Error::other(format!("Decompression error {status:?}")));
    }

    output.write_all(&output_buffer[..bytes_written])?;

    // Move remaining bytes to the beginning of the buffer
    input_buffer.copy_within(bytes_consumed..bytes_avail, 0);
    input_offset = bytes_avail - bytes_consumed;

    if status == Ok(MZStatus::StreamEnd) {
      break; // End of decompression
    }
  }

  Ok(())
}

/// Copy the V8 archive at `url` to `filename`.
///
/// This function doesn't use [`fs::copy`] because that would
/// preserve the file attributes such as ownership and mode flags.
/// Instead, it copies the file contents to a new file.
/// This is necessary because the V8 archive could live inside a read-only
/// filesystem, and subsequent builds would fail to overwrite it.
///
/// The contents are written to a staging file and renamed into place only once
/// the copy/decompress fully succeeds, so a mid-way failure (corrupt gzip, disk
/// full) can't truncate a previously good `filename`.
fn copy_archive(url: &str, filename: &Path) -> Result<(), FetchError> {
  println!("Copying {url} to {filename:?}");
  let staging = filename.with_extension("copytmp");
  let result = copy_archive_to(url, &staging);
  if result.is_err() {
    let _ = fs::remove_file(&staging);
    return result;
  }
  fs::rename(&staging, filename).map_err(|e| {
    let _ = fs::remove_file(&staging);
    FetchError::Fatal(format!(
      "could not move {} into place at {}: {e}",
      staging.display(),
      filename.display()
    ))
  })
}

/// Copy/decompress the archive at `url` into the staging file `dst_path`.
fn copy_archive_to(url: &str, dst_path: &Path) -> Result<(), FetchError> {
  let mut src = fs::File::open(url).map_err(|e| {
    FetchError::Fatal(format!("could not open source archive {url}: {e}"))
  })?;
  let mut dst = fs::File::create(dst_path).map_err(|e| {
    FetchError::Fatal(format!(
      "could not create {} from {url}: {e}",
      dst_path.display()
    ))
  })?;

  // Allow both GZIP and non-GZIP downloads
  let mut header = [0; 2];
  src
    .read_exact(&mut header)
    .and_then(|()| src.seek(io::SeekFrom::Start(0)))
    .map_err(|e| {
      FetchError::Fatal(format!(
        "could not read archive header from {url}: {e}"
      ))
    })?;
  if header == [0x1f, 0x8b] {
    println!("Detected GZIP archive: {url}");
    decompress_to_writer(&mut src, &mut dst).map_err(|e| {
      FetchError::Fatal(format!(
        "could not decompress {url} into {}: {e}",
        dst_path.display()
      ))
    })
  } else {
    println!("Not a GZIP archive: {url}");
    io::copy(&mut src, &mut dst).map(|_| ()).map_err(|e| {
      FetchError::Fatal(format!(
        "could not copy {url} into {}: {e}",
        dst_path.display()
      ))
    })
  }
}

fn print_link_flags() {
  println!("cargo:rustc-link-lib=static=rusty_v8");
  let should_dyn_link_libcxx = env::var("CARGO_FEATURE_USE_CUSTOM_LIBCXX")
    .is_err()
    || env::var("GN_ARGS").is_ok_and(|gn_args| {
      gn_args
        .split_whitespace()
        .any(|ba| ba == "use_custom_libcxx=false")
    });

  if should_dyn_link_libcxx {
    // Based on https://github.com/alexcrichton/cc-rs/blob/fba7feded71ee4f63cfe885673ead6d7b4f2f454/src/lib.rs#L2462
    if let Ok(stdlib) = env::var("CXXSTDLIB") {
      if !stdlib.is_empty() {
        println!("cargo:rustc-link-lib=dylib={stdlib}");
      }
    } else {
      let target = env::var("TARGET").unwrap();
      if target.contains("msvc") {
        // nothing to link to
      } else if target.contains("apple")
        || target.contains("freebsd")
        || target.contains("openbsd")
      {
        println!("cargo:rustc-link-lib=dylib=c++");
      } else if target.contains("android") {
        println!("cargo:rustc-link-lib=dylib=c++_shared");
      } else {
        println!("cargo:rustc-link-lib=dylib=stdc++");
      }
    }
  }
  let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap();
  let target_env = env::var("CARGO_CFG_TARGET_ENV").unwrap();

  if target_os == "windows" {
    println!("cargo:rustc-link-lib=dylib=winmm");
    println!("cargo:rustc-link-lib=dylib=dbghelp");
  }

  if target_env == "msvc" {
    // On Windows, including libcpmt[d]/msvcprt[d] explicitly links the C++
    // standard library, which libc++ needs for exception_ptr internals.
    let crt_static = env::var("CARGO_CFG_TARGET_FEATURE")
      .unwrap_or_default()
      .contains("crt-static");
    if crt_static {
      println!("cargo:rustc-link-lib=libcpmt");
    } else {
      println!("cargo:rustc-link-lib=dylib=msvcprt");
    }
  }
}

/// Point `RUSTY_V8_SRC_BINDING_PATH` at the prebuilt bindings.
///
/// A published crate ships the binding in `gen/<name>.rs`; when it is present
/// and no mirror is configured, it is used as-is. Otherwise the binding is
/// fetched into `OUT_DIR` (never written back into the source tree) and the env
/// var points there:
///
/// - A mirror is configured — the binding is fetched from it (with the usual
///   fallback), regardless of `fetch_when_missing`.
/// - No mirror, but the binding is missing from this checkout (e.g. a git
///   checkout, which ships only `gen/.gitkeep`) — it is fetched from upstream,
///   but only when `fetch_when_missing` is true.
///
/// `fetch_when_missing` gates *only* that last, missing-file case. The
/// `DOCS_RS`/RLS early-exit passes `false`: those contexts may have no network,
/// and a published crate already ships the binding, so a missing one falls
/// through to the `include!` diagnostic in `src/binding.rs` rather than panic.
/// (A mirror set on that path still downloads, matching prior behaviour.)
fn print_prebuilt_src_binding_path(fetch_when_missing: bool) {
  if let Ok(binding) = env::var("RUSTY_V8_SRC_BINDING_PATH") {
    println!("cargo:rustc-env=RUSTY_V8_SRC_BINDING_PATH={binding}");
    return;
  }

  let target = env::var("TARGET").unwrap();
  let profile = prebuilt_profile();
  let features = prebuilt_features_suffix();
  let name = format!("src_binding{features}_{profile}_{target}.rs");

  // The binding shipped with a published crate.
  let shipped = get_dirs().root.join("gen").join(&name);
  let mirror_set = env::var_os("RUSTY_V8_MIRROR").is_some();

  let binding_path = if shipped.exists() && !mirror_set {
    shipped
  } else if mirror_set || fetch_when_missing {
    // Fetch into OUT_DIR rather than the (possibly read-only, registry-owned)
    // source tree. OUT_DIR always exists, so no directory creation is needed.
    let out = PathBuf::from(env::var("OUT_DIR").unwrap()).join(&name);
    let urls = artifact_urls(&name);
    download_artifact(&urls, &out, "the V8 bindings");
    out
  } else {
    // Not shipped, no mirror, and fetching is not allowed here (docs.rs/RLS).
    // Point at the (missing) shipped path so `include!` produces its own
    // diagnostic, exactly as before this change.
    shipped
  };

  println!(
    "cargo:rustc-env=RUSTY_V8_SRC_BINDING_PATH={}",
    binding_path.display()
  );
}

// Chromium depot_tools contains helpers
// which delegate to the "relevant" `buildtools`
// directory when invoked, so they don't count.
#[allow(clippy::needless_pass_by_value)]
fn not_in_depot_tools(p: PathBuf) -> bool {
  !p.to_str().unwrap().contains("depot_tools")
}

fn need_gn_ninja_download() -> bool {
  let has_ninja = which("ninja").is_ok_and(not_in_depot_tools)
    || env::var_os("NINJA").is_some();
  let has_gn =
    which("gn").is_ok_and(not_in_depot_tools) || env::var_os("GN").is_some();

  !has_ninja || !has_gn
}

// Chromiums gn arg clang_base_path is currently compatible with:
// * Apples clang and clang from homebrew's llvm@x packages
// * the official binaries from releases.llvm.org
// * unversioned (Linux) packages of clang (if recent enough)
// but unfortunately it doesn't work with version-suffixed packages commonly
// found in Linux packet managers
fn is_compatible_clang_version(clang_path: &Path) -> bool {
  if let Ok(o) = Command::new(clang_path).arg("--version").output() {
    let _output = String::from_utf8(o.stdout).unwrap();
    // TODO check version output to make sure it's supported.
    const _MIN_APPLE_CLANG_VER: f32 = 11.0;
    const _MIN_LLVM_CLANG_VER: f32 = 8.0;
    return true;
  }
  false
}

fn find_compatible_system_clang() -> Option<PathBuf> {
  if let Ok(p) = env::var("CLANG_BASE_PATH") {
    let base_path = Path::new(&p);
    let clang_path = base_path.join("bin").join("clang");
    if is_compatible_clang_version(&clang_path) {
      return Some(base_path.to_path_buf());
    }
  }

  None
}

// Download chromium's clang into OUT_DIR because Cargo will not allow us to
// modify the source directory.
fn clang_download() -> PathBuf {
  let clang_base_path = build_dir().join("clang");
  println!("clang_base_path (downloaded) {}", clang_base_path.display());
  assert!(
    Command::new(python())
      .arg("./tools/clang/scripts/update.py")
      .arg("--output-dir")
      .arg(&clang_base_path)
      .status()
      .unwrap()
      .success()
  );

  // Chromium ships libclang separately from the compiler on Windows. Use the
  // matching pinned package for bindgen instead of whichever libclang happens
  // to be installed on the host.
  #[cfg(target_os = "windows")]
  if env::var_os("LIBCLANG_PATH").is_none() {
    assert!(
      Command::new(python())
        .arg("./tools/clang/scripts/update.py")
        .arg("--output-dir")
        .arg(&clang_base_path)
        .arg("--package")
        .arg("libclang")
        .status()
        .unwrap()
        .success()
    );
    let libclang_path = clang_base_path.join("bin");
    assert!(libclang_path.join("libclang.dll").exists());
    println!("libclang_path (downloaded) {}", libclang_path.display());
    unsafe {
      env::set_var("LIBCLANG_PATH", libclang_path);
    }
  }

  assert!(clang_base_path.exists());
  clang_base_path
}

fn cc_wrapper(gn_args: &mut Vec<String>, sccache_path: &Path) {
  gn_args.push(format!("cc_wrapper={sccache_path:?}"));
}

struct Dirs {
  pub out: PathBuf,
  pub root: PathBuf,
}

fn get_dirs() -> Dirs {
  // The OUT_DIR is going to be a crate-specific directory like
  // "target/debug/build/cargo_gn_example-eee5160084460b2c"
  // But we want to share the GN build amongst all crates
  // and return the path "target/debug". So to find it, we walk up three
  // directories.
  // TODO(ry) This is quite brittle - if Cargo changes the directory structure
  // this could break.
  let out = env::var("OUT_DIR").map(PathBuf::from).unwrap();
  let out = out
    .parent()
    .unwrap()
    .parent()
    .unwrap()
    .parent()
    .unwrap()
    .to_owned();

  let root = env::var("CARGO_MANIFEST_DIR").map(PathBuf::from).unwrap();
  let mut dirs = Dirs { out, root };
  maybe_symlink_root_dir(&mut dirs);
  dirs
}

#[cfg(not(target_os = "windows"))]
fn maybe_symlink_root_dir(_: &mut Dirs) {}

#[cfg(target_os = "windows")]
fn maybe_symlink_root_dir(dirs: &mut Dirs) {
  // GN produces invalid paths if the source (a.k.a. root) directory is on a
  // different drive than the output. If this is the case we'll create a
  // symlink called 'gn_root' in the out directory, next to 'gn_out', so it
  // appears as if they're both on the same drive.
  use fs::{remove_dir_all, remove_file};
  use std::os::windows::fs::symlink_dir;

  let get_prefix = |p: &Path| {
    p.components()
      .find_map(|c| match c {
        std::path::Component::Prefix(p) => Some(p),
        _ => None,
      })
      .map(|p| p.as_os_str().to_owned())
  };

  let Dirs { out, root } = dirs;
  if get_prefix(out) != get_prefix(root) {
    let symlink = &*out.join("gn_root");
    let target = &*root.canonicalize().unwrap();

    println!("Creating symlink {symlink:?} to {root:?}");

    let mut retries = 0;
    loop {
      match symlink.canonicalize() {
        Ok(existing) if existing == target => break,
        Ok(_) => remove_dir_all(symlink).expect("remove_dir_all failed"),
        Err(err) => {
          println!("symlink.canonicalize failed: {err:?}");
          // we're having very strange issues on GHA when the cache
          // is restored, so trying this out temporarily
          if let Err(err) = remove_dir_all(symlink) {
            eprintln!("remove_dir_all failed: {err:?}");
            if let Err(err) = remove_file(symlink) {
              eprintln!("remove_file failed: {err:?}");
            }
          }
          match symlink_dir(target, symlink) {
            Ok(_) => break,
            Err(err) => {
              println!("symlink_dir failed: {err:?}");
              retries += 1;
              std::thread::sleep(std::time::Duration::from_millis(
                50 * retries,
              ));
              if retries > 4 {
                panic!("Failed to create symlink");
              }
            }
          }
        }
      }
    }

    dirs.root = symlink.to_path_buf();
  }
}

pub fn is_debug() -> bool {
  // Cargo sets PROFILE to either "debug" or "release", which conveniently
  // matches the build modes we support.
  let m = env::var("PROFILE").unwrap();
  if m == "release" {
    false
  } else if m == "debug" {
    true
  } else {
    panic!("unhandled PROFILE value {m}")
  }
}

fn gn() -> String {
  env::var("GN").unwrap_or_else(|_| "gn".to_owned())
}

/*
 * Get the system's python binary - specified via the PYTHON environment
 * variable or defaulting to `python3`.
 */
fn python() -> String {
  env::var("PYTHON").unwrap_or_else(|_| "python3".to_owned())
}

type NinjaEnv = Vec<(String, String)>;

fn ninja(gn_out_dir: &Path, maybe_env: Option<NinjaEnv>) -> Command {
  let cmd_string = env::var("NINJA").unwrap_or_else(|_| "ninja".to_owned());
  let mut cmd = Command::new(&cmd_string);
  cmd.arg("-C");
  cmd.arg(gn_out_dir);
  if !cmd_string.ends_with("autoninja")
    && let Ok(jobs) = env::var("NUM_JOBS")
  {
    cmd.arg("-j");
    cmd.arg(jobs);
  }
  if let Some(env) = maybe_env {
    for item in env {
      cmd.env(item.0, item.1);
    }
  }
  cmd
}

fn run_gn_gen(gn_args: &[String]) -> PathBuf {
  let dirs = get_dirs();
  let gn_out_dir = dirs.out.join("gn_out");

  let mut args = gn_args.join(" ");
  if let Ok(extra_args) = env::var("EXTRA_GN_ARGS") {
    args.push(' ');
    args.push_str(&extra_args);
  }

  let path = env::current_dir().unwrap();
  println!("The current directory is {}", path.display());
  println!(
    "gn gen --root={} {}",
    dirs.root.display(),
    gn_out_dir.display()
  );
  assert!(
    Command::new(gn())
      .arg(format!("--root={}", dirs.root.display()))
      .arg(format!("--script-executable={}", python()))
      .arg("gen")
      .arg(&gn_out_dir)
      .arg("--ide=json")
      .arg("--args=".to_owned() + &args)
      .stdout(Stdio::inherit())
      .stderr(Stdio::inherit())
      .envs(env::vars())
      .status()
      .expect("Could not run `gn`")
      .success()
  );

  gn_out_dir
}

pub fn build(target: &str, maybe_env: Option<NinjaEnv>) {
  let gn_out_dir = get_dirs().out.join("gn_out");

  rerun_if_changed(&gn_out_dir, maybe_env.clone(), target);

  // This helps Rust source files locate the snapshot, source map etc.
  println!("cargo:rustc-env=GN_OUT_DIR={}", gn_out_dir.display());

  assert!(
    ninja(&gn_out_dir, maybe_env)
      .arg(target)
      .status()
      .unwrap()
      .success()
  );

  // TODO This is not sufficient. We need to use "gn desc" to query the target
  // and figure out what else we need to add to the link.
  println!(
    "cargo:rustc-link-search=native={}/obj/",
    gn_out_dir.display()
  );
}

/// build.rs does not get re-run unless we tell cargo about what files we
/// depend on. This outputs a bunch of rerun-if-changed lines to stdout.
fn rerun_if_changed(out_dir: &Path, maybe_env: Option<NinjaEnv>, target: &str) {
  let deps = ninja_get_deps(out_dir, maybe_env, target);
  for d in deps {
    if let Ok(p) = out_dir.join(d).canonicalize() {
      println!("cargo:rerun-if-changed={}", p.display());
    }
  }
}

fn ninja_get_deps(
  out_dir: &Path,
  maybe_env: Option<NinjaEnv>,
  target: &str,
) -> HashSet<String> {
  let mut cmd = ninja(out_dir, maybe_env.clone());
  cmd.arg("-t");
  cmd.arg("graph");
  cmd.arg(target);
  let output = cmd.output().expect("ninja -t graph failed");
  let stdout = String::from_utf8(output.stdout).unwrap();
  let graph_files = parse_ninja_graph(&stdout);

  let mut cmd = ninja(out_dir, maybe_env);
  cmd.arg(target);
  cmd.arg("-t");
  cmd.arg("deps");
  let output = cmd.output().expect("ninja -t deps failed");
  let stdout = String::from_utf8(output.stdout).unwrap();
  let deps_files = parse_ninja_deps(&stdout);

  graph_files.union(&deps_files).map(String::from).collect()
}

pub fn parse_ninja_deps(s: &str) -> HashSet<String> {
  let mut out = HashSet::new();
  for line in s.lines() {
    if line.starts_with("  ") {
      let filename = line.trim().to_string();
      out.insert(filename);
    }
  }
  out
}

/// A parser for the output of "ninja -t graph". It returns all the input files.
pub fn parse_ninja_graph(s: &str) -> HashSet<String> {
  let mut out = HashSet::new();
  // This is extremely hacky and likely to break.
  for line in s.lines() {
    if line.starts_with('\"')
      && line.contains("label=")
      && !line.contains("shape=")
      && !line.contains(" -> ")
    {
      let filename = line.split('\"').nth(3).unwrap();
      if !filename.starts_with("..") {
        continue;
      }
      out.insert(filename.to_string());
    }
  }
  out
}

fn env_bool(key: &str) -> bool {
  matches!(
    env::var(key).unwrap_or_default().as_str(),
    "true" | "1" | "yes"
  )
}

#[cfg(test)]
mod test {
  use super::*;

  const MOCK_GRAPH: &str = r#"
digraph ninja {
rankdir="LR"
node [fontsize=10, shape=box, height=0.25]
edge [fontsize=10]
"0x7fc3c040c210" [label="default"]
"0x7fc3c040a7f0" -> "0x7fc3c040c210" [label=" phony"]
"0x7fc3c040a7f0" [label="obj/default.stamp"]
"0x7fc3c040a790" [label="stamp", shape=ellipse]
"0x7fc3c040a790" -> "0x7fc3c040a7f0"
"0x7fc3c040a6c0" -> "0x7fc3c040a790" [arrowhead=none]
"0x7fc3c040a8a0" -> "0x7fc3c040a790" [arrowhead=none]
"0x7fc3c040a920" -> "0x7fc3c040a790" [arrowhead=none]
"0x7fc3c040a6c0" [label="obj/count_bytes.stamp"]
"0x7fc3c040a4d0" -> "0x7fc3c040a6c0" [label=" stamp"]
"0x7fc3c040a4d0" [label="gen/output.txt"]
"0x7fc3c040a400" [label="___count_bytes___build_toolchain_mac_clang_x64__rule", shape=ellipse]
"0x7fc3c040a400" -> "0x7fc3c040a4d0"
"0x7fc3c040a580" -> "0x7fc3c040a400" [arrowhead=none]
"0x7fc3c040a620" -> "0x7fc3c040a400" [arrowhead=none]
"0x7fc3c040a580" [label="../../../example/src/count_bytes.py"]
"0x7fc3c040a620" [label="../../../example/src/input.txt"]
"0x7fc3c040a8a0" [label="foo"]
"0x7fc3c040b5e0" [label="link", shape=ellipse]
"0x7fc3c040b5e0" -> "0x7fc3c040a8a0"
"0x7fc3c040b5e0" -> "0x7fc3c040b6d0"
"0x7fc3c040b5e0" -> "0x7fc3c040b780"
"0x7fc3c040b5e0" -> "0x7fc3c040b820"
"0x7fc3c040b020" -> "0x7fc3c040b5e0" [arrowhead=none]
"0x7fc3c040a920" -> "0x7fc3c040b5e0" [arrowhead=none]
"0x7fc3c040b020" [label="obj/foo/foo.o"]
"0x7fc3c040b0d0" -> "0x7fc3c040b020" [label=" cxx"]
"0x7fc3c040b0d0" [label="../../../example/src/foo.cc"]
"0x7fc3c040a920" [label="obj/libhello.a"]
"0x7fc3c040be00" -> "0x7fc3c040a920" [label=" alink"]
"0x7fc3c040be00" [label="obj/hello/hello.o"]
"0x7fc3c040beb0" -> "0x7fc3c040be00" [label=" cxx"]
"0x7fc3c040beb0" [label="../../../example/src/hello.cc"]
}
  "#;

  #[test]
  fn test_parse_ninja_graph() {
    let files = parse_ninja_graph(MOCK_GRAPH);
    assert!(files.contains("../../../example/src/input.txt"));
    assert!(files.contains("../../../example/src/count_bytes.py"));
    assert!(!files.contains("obj/hello/hello.o"));
  }

  #[test]
  fn test_clang_resource_dir_missing_executable() {
    let clang_bin = env::temp_dir()
      .join(format!("rusty_v8_missing_clang_{}", std::process::id()));
    assert!(clang_resource_dir(&clang_bin).is_err());
  }

  #[test]
  fn test_expand_mirror_template() {
    let out = expand_mirror_template(
      "https://cache.example.com/{target}/{tag}/{profile}{features}/{version}/{file}",
      "v152.1.0",
      "152.1.0",
      "aarch64-apple-darwin",
      "release",
      "_ptrcomp",
      "librusty_v8_ptrcomp_release_aarch64-apple-darwin.a.gz",
    );
    assert_eq!(
      out,
      "https://cache.example.com/aarch64-apple-darwin/v152.1.0/release_ptrcomp/152.1.0/librusty_v8_ptrcomp_release_aarch64-apple-darwin.a.gz"
    );

    // Empty features, plain filesystem path (not a `file://` URL, which the
    // filesystem candidate does not support).
    let out = expand_mirror_template(
      "/mnt/{tag}/{file}",
      "nightly",
      "152.1.0",
      "x86_64-unknown-linux-gnu",
      "debug",
      "",
      "src_binding_debug_x86_64-unknown-linux-gnu.rs",
    );
    assert_eq!(
      out,
      "/mnt/nightly/src_binding_debug_x86_64-unknown-linux-gnu.rs"
    );
  }

  fn urls(candidates: &[Candidate]) -> Vec<(&str, bool)> {
    candidates
      .iter()
      .map(|c| (c.url.as_str(), c.is_cache))
      .collect()
  }

  const UP: &str = "https://github.com/denoland/rusty_v8/releases/download";

  #[test]
  fn test_artifact_urls_no_mirror() {
    // No mirror: exactly one candidate, the upstream default at the tag.
    let c = artifact_urls_from(
      None, "v1.2.3", false, "1.2.3", "t", "release", "", "lib.a.gz",
    );
    assert_eq!(
      urls(&c),
      [(format!("{UP}/v1.2.3/lib.a.gz").as_str(), false)]
    );
  }

  #[test]
  fn test_artifact_urls_http_mirror() {
    // HTTP mirror: `<base>/<tag>/<file>` then upstream. No flat candidate.
    let c = artifact_urls_from(
      Some("https://m.example.com"),
      "v1.2.3",
      false,
      "1.2.3",
      "t",
      "release",
      "",
      "lib.a.gz",
    );
    assert_eq!(
      urls(&c),
      [
        ("https://m.example.com/v1.2.3/lib.a.gz", false),
        (format!("{UP}/v1.2.3/lib.a.gz").as_str(), false),
      ]
    );
  }

  #[test]
  fn test_artifact_urls_fs_mirror_has_flat_cache() {
    // Filesystem mirror: tag layout, then a flat cache candidate, then upstream.
    let c = artifact_urls_from(
      Some("/srv/mirror"),
      "v1.2.3",
      false,
      "1.2.3",
      "t",
      "release",
      "",
      "lib.a.gz",
    );
    assert_eq!(
      urls(&c),
      [
        ("/srv/mirror/v1.2.3/lib.a.gz", false),
        ("/srv/mirror/lib.a.gz", true),
        (format!("{UP}/v1.2.3/lib.a.gz").as_str(), false),
      ]
    );
  }

  #[test]
  fn test_artifact_urls_template_mirror() {
    // Template mirror: exactly the expanded template, then upstream. No flat.
    let c = artifact_urls_from(
      Some("https://c/{target}/{tag}/{file}"),
      "v1.2.3",
      false,
      "1.2.3",
      "aarch64",
      "release",
      "",
      "lib.a.gz",
    );
    assert_eq!(
      urls(&c),
      [
        ("https://c/aarch64/v1.2.3/lib.a.gz", false),
        (format!("{UP}/v1.2.3/lib.a.gz").as_str(), false),
      ]
    );
  }

  #[test]
  fn test_artifact_urls_strict_with_mirror_drops_upstream() {
    let c = artifact_urls_from(
      Some("/srv/mirror"),
      "v1.2.3",
      true,
      "1.2.3",
      "t",
      "release",
      "",
      "lib.a.gz",
    );
    assert_eq!(
      urls(&c),
      [
        ("/srv/mirror/v1.2.3/lib.a.gz", false),
        ("/srv/mirror/lib.a.gz", true),
      ]
    );
  }

  #[test]
  fn test_artifact_urls_strict_without_mirror_is_empty() {
    // Strict is honored unconditionally: no mirror + strict => nowhere to fetch.
    // The caller turns this empty list into an explicit error.
    let c = artifact_urls_from(
      None, "v1.2.3", true, "1.2.3", "t", "release", "", "lib.a.gz",
    );
    assert!(c.is_empty());
  }
}
