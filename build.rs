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
    "RUSTY_V8_BINDGEN_RESOURCE_DIR",
    "RUSTY_V8_GLIBC_PREFIX",
    "RUSTY_V8_MIRROR",
    "RUSTY_V8_MIRROR_TAG",
    "RUSTY_V8_MIRROR_FALLBACK",
    "RUSTY_V8_MUSL_SYSROOT",
    "RUSTY_V8_SKIP_DOWNLOAD",
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

  // Early exit
  if is_cargo_doc {
    print_prebuilt_src_binding_path();
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

  print_prebuilt_src_binding_path();

  download_static_lib_binaries();
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
    let resource_dir = env::var("RUSTY_V8_BINDGEN_RESOURCE_DIR").ok();
    if resource_dir.is_none()
      && let Ok(libclang_path) = env::var("LIBCLANG_PATH")
    {
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
    // Add target libc headers for Linux cross-builds. bindgen already targets
    // Cargo's triple (from $TARGET), but target headers may not be installed in
    // the host's search paths.
    let target_env = env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
    let musl_sysroot = env::var("RUSTY_V8_MUSL_SYSROOT").ok();
    let glibc_prefix = env::var("RUSTY_V8_GLIBC_PREFIX").ok();
    clang_args.extend(explicit_linux_bindgen_args(
      &target_env,
      resource_dir.as_deref(),
      musl_sysroot.as_deref(),
      glibc_prefix.as_deref(),
    ));
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

/// Builds the explicit Linux header-search arguments for bindgen.
///
/// An explicit Clang resource directory uses `-resource-dir` so Clang resolves
/// the complete builtin resource layout. The auto-discovery path only adds the
/// discovered `include` directory because it is supplementing libclang's
/// existing resource setup.
///
/// Musl receives `--sysroot` because its input is a complete sysroot. A GNU
/// cross prefix instead stores target headers directly under `<prefix>/include`;
/// treating it as a sysroot would make Clang search `<prefix>/usr/include`.
/// bindgen supplies the Clang target from Cargo's `TARGET` environment variable.
fn explicit_linux_bindgen_args(
  target_env: &str,
  resource_dir: Option<&str>,
  musl_sysroot: Option<&str>,
  glibc_prefix: Option<&str>,
) -> Vec<String> {
  let mut args = Vec::new();
  if let Some(resource_dir) = resource_dir {
    args.push(format!("-resource-dir={resource_dir}"));
  }

  match target_env {
    "musl" => {
      if let Some(sysroot) = musl_sysroot {
        args.push(format!("--sysroot={sysroot}"));
      }
    }
    "gnu" => {
      if let Some(prefix) = glibc_prefix {
        args.push(format!("-isystem{prefix}/include"));
      }
    }
    _ => {}
  }
  args
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

const DEFAULT_ARTIFACT_BASE: &str =
  "https://github.com/denoland/rusty_v8/releases/download";

/// Values substituted into artifact URLs and URL templates.
struct UrlVars<'a> {
  /// Release tag path segment, e.g. `v152.0.0` (see [`resolved_tag`]).
  tag: &'a str,
  /// Raw `CARGO_PKG_VERSION`, without the `v` prefix.
  version: &'a str,
  /// The `TARGET` triple.
  target: &'a str,
  /// `release` or `debug`, from [`prebuilt_profile`].
  profile: &'a str,
  /// Feature suffix from [`prebuilt_features_suffix`].
  features: &'a str,
  /// The full artifact filename.
  file: &'a str,
}

/// The tag path segment under which prebuilt artifacts are looked up.
/// `RUSTY_V8_MIRROR_TAG` overrides the default `v{version}` verbatim (no `v`
/// is prepended), so mirrors can serve artifacts from e.g. the last published
/// tag while `Cargo.toml` carries an unpublished version.
fn resolved_tag(tag_override: Option<&str>, version: &str) -> String {
  match tag_override {
    Some(tag) => tag.to_string(),
    None => format!("v{version}"),
  }
}

/// Expand `{tag}`, `{version}`, `{target}`, `{profile}`, `{features}` and
/// `{file}` placeholders in a `RUSTY_V8_MIRROR` template.
fn expand_url_template(template: &str, vars: &UrlVars) -> String {
  template
    .replace("{tag}", vars.tag)
    .replace("{version}", vars.version)
    .replace("{target}", vars.target)
    .replace("{profile}", vars.profile)
    .replace("{features}", vars.features)
    .replace("{file}", vars.file)
}

fn is_remote_url(url: &str) -> bool {
  url.starts_with("http:") || url.starts_with("https:")
}

/// The ordered list of URLs an artifact is fetched from:
///
/// 1. The mirror, if `RUSTY_V8_MIRROR` is set. A value containing a `{`
///    placeholder is treated as a full URL template; otherwise it is a base
///    and the artifact is looked up at `{base}/{tag}/{file}`.
/// 2. For plain (non-template) filesystem mirrors only: the flat layout
///    `{base}/{file}`, so a directory of downloaded artifacts works as a
///    cache without tag subdirectories.
/// 3. The upstream GitHub release. When a mirror is configured this entry is
///    only present with `fallback` (`RUSTY_V8_MIRROR_FALLBACK=1`): a mirror
///    fails closed by default and never silently reaches the network.
fn candidate_urls(
  mirror: Option<&str>,
  fallback: bool,
  vars: &UrlVars,
) -> Vec<String> {
  let mut urls = Vec::new();
  if let Some(mirror) = mirror {
    if mirror.contains('{') {
      urls.push(expand_url_template(mirror, vars));
    } else {
      urls.push(format!("{mirror}/{}/{}", vars.tag, vars.file));
      if !is_remote_url(mirror) {
        urls.push(format!("{mirror}/{}", vars.file));
      }
    }
  }
  if mirror.is_none() || fallback {
    urls.push(format!(
      "{DEFAULT_ARTIFACT_BASE}/{}/{}",
      vars.tag, vars.file
    ));
  }
  urls
}

/// [`candidate_urls`] with all inputs read from the environment.
fn artifact_url_candidates(file: &str) -> Vec<String> {
  let version = env::var("CARGO_PKG_VERSION").unwrap();
  let tag_override = env::var("RUSTY_V8_MIRROR_TAG").ok();
  let tag = resolved_tag(tag_override.as_deref(), &version);
  let target = env::var("TARGET").unwrap();
  let features = prebuilt_features_suffix();
  let vars = UrlVars {
    tag: &tag,
    version: &version,
    target: &target,
    profile: prebuilt_profile(),
    features: &features,
    file,
  };
  let mirror = env::var("RUSTY_V8_MIRROR").ok();
  let fallback = env_bool("RUSTY_V8_MIRROR_FALLBACK");
  if mirror.is_some() && !fallback {
    println!(
      "RUSTY_V8_MIRROR is set; not falling back to {DEFAULT_ARTIFACT_BASE} \
       (set RUSTY_V8_MIRROR_FALLBACK=1 to enable)"
    );
  }
  candidate_urls(mirror.as_deref(), fallback, &vars)
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

/// Fetch an artifact into `filename`, trying each candidate URL in order.
/// Panics with the full list of attempted URLs, and the escape hatches, if
/// every candidate fails.
fn download_artifact(urls: &[String], filename: &Path) {
  if let Err(error) = try_download_artifact(urls, filename) {
    panic!("{error}");
  }
}

/// Like [`download_artifact`], but returns the diagnostic instead of
/// panicking, so callers can fall back on failure (e.g.
/// `RUSTY_V8_SKIP_DOWNLOAD` reusing an existing binding).
fn try_download_artifact(
  urls: &[String],
  filename: &Path,
) -> Result<(), String> {
  // Checksum (i.e: source URL) to avoid re-downloads: reuse the existing file
  // if it was fetched from any URL we would fetch from now.
  if filename.exists()
    && let Ok(recorded) = fs::read_to_string(static_checksum_path(filename))
    && urls.contains(&recorded)
  {
    return Ok(());
  }

  let mut errors = Vec::new();
  for url in urls {
    println!("Trying to fetch from {url}");
    match download_file(url, filename) {
      Ok(()) => return Ok(()),
      Err(error) => {
        println!("Failed to fetch from {url}: {error}");
        errors.push(format!("- {url}: {error}"));
      }
    }
  }
  // A configured mirror fails closed: mention the opt-in that would have
  // allowed falling back to the upstream release.
  let fallback_hint = if env::var("RUSTY_V8_MIRROR").is_ok()
    && !env_bool("RUSTY_V8_MIRROR_FALLBACK")
  {
    " Set RUSTY_V8_MIRROR_FALLBACK=1 to fall back to the upstream GitHub \
     release when the mirror misses."
  } else {
    ""
  };
  Err(format!(
    "Failed to fetch the V8 prebuilt artifact {}. Tried:\n{}\n\
     If no prebuilt artifact is published for your target or version, \
     compile V8 from source by setting V8_FROM_SOURCE=1. You can also point \
     the build at an artifact via RUSTY_V8_ARCHIVE (static lib only), a \
     local binding via RUSTY_V8_SRC_BINDING_PATH (src binding only), a \
     mirror via RUSTY_V8_MIRROR, or another release tag via \
     RUSTY_V8_MIRROR_TAG.{fallback_hint}",
    filename.display(),
    errors.join("\n")
  ))
}

fn download_file(url: &str, filename: &Path) -> Result<(), String> {
  if !is_remote_url(url) {
    copy_archive(url, filename)?;
    // Local copies are not recorded in the checksum file; remove any stale
    // record of a previous http(s) download so it cannot mask this copy.
    let _ = fs::remove_file(static_checksum_path(filename));
    return Ok(());
  }

  // If there is a `.cargo/.rusty_v8/<escaped URL>` file, use that instead
  // of downloading.
  if let Ok(mut path) = home::cargo_home() {
    path = path.join(".rusty_v8").join(replace_non_alphanumeric(url));
    println!("Looking for download in '{path:?}'");
    if path.exists() {
      match copy_archive(&path.to_string_lossy(), filename) {
        Ok(()) => {
          // Local copies are not recorded in the checksum file; remove any
          // stale record of a previous http(s) download.
          let _ = fs::remove_file(static_checksum_path(filename));
          return Ok(());
        }
        // A bad cache entry should not disable this URL; fall through and
        // download it.
        Err(error) => println!("Failed to copy {path:?}: {error}"),
      }
    }
  }

  let tmpfile = filename.with_extension("tmp");
  if tmpfile.exists() {
    println!("Deleting old tmpfile {}", tmpfile.display());
    fs::remove_file(&tmpfile)
      .map_err(|e| format!("failed to delete {}: {e}", tmpfile.display()))?;
  }

  // Try downloading with deno first, then python, then curl.
  println!("Downloading {url}");
  let status = which("deno").ok().and_then(|deno| {
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
      .arg(&tmpfile)
      .status()
      .ok()
      .filter(|s| s.success())
  });

  // Try downloading with python. Python is a V8 build dependency,
  // so this saves us from adding a Rust HTTP client dependency.
  let status = match status {
    Some(status) => Some(status),
    _ => {
      println!("Trying with Python...");
      let python_status = Command::new(python())
        .arg("./tools/download_file.py")
        .arg("--url")
        .arg(url)
        .arg("--filename")
        .arg(&tmpfile)
        .status();

      // Python is only a required dependency for `V8_FROM_SOURCE` builds.
      // If python is not available, try falling back to curl. A missing curl
      // binary is not fatal either; it counts as a failed download and falls
      // through to the diagnostic below.
      match python_status {
        Ok(status) if status.success() => Some(status),
        _ => {
          println!("Python downloader failed, trying with curl.");
          Command::new("curl")
            .arg("-L")
            .arg("-f")
            .arg("-s")
            .arg("-o")
            .arg(&tmpfile)
            .arg(url)
            .status()
            .ok()
        }
      }
    }
  };

  // Check the download was successful; clean up the partial download if not.
  if !status.is_some_and(|status| status.success()) {
    if tmpfile.exists() {
      let _ = fs::remove_file(&tmpfile);
    }
    return Err(
      "no downloader (deno, python, curl) could fetch the URL".to_string(),
    );
  }
  if !tmpfile.exists() {
    return Err("downloader reported success but produced no file".to_string());
  }

  // Move file & write checksum (i.e url)
  if let Err(error) = copy_archive(&tmpfile.to_string_lossy(), filename) {
    let _ = fs::remove_file(&tmpfile);
    return Err(error);
  }
  fs::remove_file(&tmpfile)
    .map_err(|e| format!("failed to delete {}: {e}", tmpfile.display()))?;
  fs::write(static_checksum_path(filename), url).unwrap_or_else(|e| {
    panic!(
      "failed to write {}: {e}",
      static_checksum_path(filename).display()
    )
  });

  assert!(filename.exists());
  assert!(static_checksum_path(filename).exists());
  assert!(!tmpfile.exists());
  Ok(())
}

fn download_static_lib_binaries() {
  let dir = static_lib_dir();
  fs::create_dir_all(&dir)
    .unwrap_or_else(|e| panic!("failed to create {}: {e}", dir.display()));
  println!("cargo:rustc-link-search={}", dir.display());

  // RUSTY_V8_SKIP_DOWNLOAD skips fetching the static library (including from
  // RUSTY_V8_ARCHIVE) so that `cargo check` and rust-analyzer can resolve the
  // crate without the prebuilt artifact. The (small) src binding file is
  // still fetched; only linking requires the static library. A library left
  // behind by a previous build is linked as usual.
  if env_bool("RUSTY_V8_SKIP_DOWNLOAD") {
    if static_lib_path().exists() {
      println!(
        "cargo:warning=RUSTY_V8_SKIP_DOWNLOAD is set; using the existing {} \
         as-is (it may be stale)",
        static_lib_path().display()
      );
    } else {
      println!(
        "cargo:warning=RUSTY_V8_SKIP_DOWNLOAD is set; the V8 static library \
         was not downloaded. `cargo check` will work, but linking will fail \
         with 'could not find native static library `rusty_v8`' until this \
         is built again without RUSTY_V8_SKIP_DOWNLOAD"
      );
    }
    return;
  }

  // RUSTY_V8_ARCHIVE points at exactly one archive and short-circuits the
  // mirror/upstream candidates entirely.
  let urls = if let Ok(custom_archive) = env::var("RUSTY_V8_ARCHIVE") {
    vec![custom_archive]
  } else {
    let target = env::var("TARGET").unwrap();
    let profile = prebuilt_profile();
    let features = prebuilt_features_suffix();
    let file = format!(
      "{}.gz",
      static_lib_name(&format!("{features}_{profile}_{target}"))
    );
    artifact_url_candidates(&file)
  };
  println!("static lib URLs: {urls:?}");

  download_artifact(&urls, &static_lib_path());
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
fn copy_archive(url: &str, filename: &Path) -> Result<(), String> {
  println!("Copying {url} to {filename:?}");
  // Write to a scratch file and rename into place on success, so a source
  // that fails partway through cannot truncate a previously good artifact.
  let partfile = filename.with_extension("part");
  let result = copy_archive_to(url, &partfile).and_then(|()| {
    fs::rename(&partfile, filename).map_err(|e| {
      format!(
        "failed to rename {} to {}: {e}",
        partfile.display(),
        filename.display()
      )
    })
  });
  if result.is_err() && partfile.exists() {
    let _ = fs::remove_file(&partfile);
  }
  result
}

fn copy_archive_to(url: &str, dst_path: &Path) -> Result<(), String> {
  let mut src = fs::File::open(url)
    .map_err(|e| format!("failed to open source archive {url}: {e}"))?;
  let mut dst = fs::File::create(dst_path)
    .map_err(|e| format!("failed to create {}: {e}", dst_path.display()))?;

  // Allow both GZIP and non-GZIP downloads
  let mut header = [0; 2];
  src
    .read_exact(&mut header)
    .map_err(|e| format!("failed to read source archive {url}: {e}"))?;
  src
    .seek(io::SeekFrom::Start(0))
    .map_err(|e| format!("failed to seek source archive {url}: {e}"))?;
  if header == [0x1f, 0x8b] {
    println!("Detected GZIP archive: {url}");
    decompress_to_writer(&mut src, &mut dst).map_err(|e| {
      format!(
        "failed to decompress {url} into {}: {e}",
        dst_path.display()
      )
    })?;
  } else {
    println!("Not a GZIP archive: {url}");
    io::copy(&mut src, &mut dst).map_err(|e| {
      format!("failed to copy {url} to {}: {e}", dst_path.display())
    })?;
  }
  Ok(())
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

fn print_prebuilt_src_binding_path() {
  if let Ok(binding) = env::var("RUSTY_V8_SRC_BINDING_PATH") {
    println!("cargo:rustc-env=RUSTY_V8_SRC_BINDING_PATH={binding}");
    return;
  }

  let target = env::var("TARGET").unwrap();
  let profile = prebuilt_profile();
  let features = prebuilt_features_suffix();
  let name = format!("src_binding{features}_{profile}_{target}.rs");

  let src_binding_path = get_dirs().root.join("gen").join(name.clone());

  // The generated binding ships in the published crate under `gen/`. Download
  // it when a mirror is configured, or when the file does not exist (e.g. a
  // git checkout), so a missing binding surfaces as a build script error
  // rather than a confusing `include!` failure.
  if env::var("RUSTY_V8_MIRROR").is_ok() || !src_binding_path.exists() {
    if let Some(parent) = src_binding_path.parent() {
      fs::create_dir_all(parent).unwrap_or_else(|e| {
        panic!("failed to create {}: {e}", parent.display())
      });
    }
    if let Err(error) =
      try_download_artifact(&artifact_url_candidates(&name), &src_binding_path)
    {
      // Under RUSTY_V8_SKIP_DOWNLOAD a stale-but-usable binding beats a
      // failed refresh: dev contexts pointed at an incomplete mirror still
      // need `cargo check` to pass. A failed fetch never truncates the
      // existing file (downloads land in a scratch file that is only renamed
      // into place on success).
      if env_bool("RUSTY_V8_SKIP_DOWNLOAD") && src_binding_path.exists() {
        println!(
          "cargo:warning=RUSTY_V8_SKIP_DOWNLOAD is set; could not refresh \
           the src binding, using the existing {} as-is (it may be stale)",
          src_binding_path.display()
        );
      } else {
        panic!("{error}");
      }
    }
  }

  println!(
    "cargo:rustc-env=RUSTY_V8_SRC_BINDING_PATH={}",
    src_binding_path.display()
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

  const TEST_VARS: UrlVars = UrlVars {
    tag: "v139.0.0",
    version: "140.0.0",
    target: "x86_64-unknown-linux-gnu",
    profile: "release",
    features: "_ptrcomp",
    file: "librusty_v8_ptrcomp_release_x86_64-unknown-linux-gnu.a.gz",
  };

  #[test]
  fn test_resolved_tag() {
    assert_eq!(resolved_tag(None, "140.0.0"), "v140.0.0");
    // Overrides are used verbatim: no `v` is prepended.
    assert_eq!(resolved_tag(Some("nightly"), "140.0.0"), "nightly");
    assert_eq!(resolved_tag(Some("v139.0.0"), "140.0.0"), "v139.0.0");
  }

  #[test]
  fn test_expand_url_template() {
    assert_eq!(
      expand_url_template(
        "https://ex.com/{tag}/{version}/{target}/{profile}{features}/{file}",
        &TEST_VARS
      ),
      "https://ex.com/v139.0.0/140.0.0/x86_64-unknown-linux-gnu/\
       release_ptrcomp/librusty_v8_ptrcomp_release_x86_64-unknown-linux-gnu.a.gz"
    );
  }

  #[test]
  fn test_candidate_urls_no_mirror() {
    assert_eq!(
      candidate_urls(None, false, &TEST_VARS),
      vec![format!(
        "{DEFAULT_ARTIFACT_BASE}/v139.0.0/{}",
        TEST_VARS.file
      )]
    );
  }

  #[test]
  fn test_candidate_urls_http_mirror() {
    // A remote base mirror keeps the `{base}/{tag}/{file}` layout and fails
    // closed: no flat layout for remote mirrors, no upstream fallback.
    assert_eq!(
      candidate_urls(Some("https://mirror.example"), false, &TEST_VARS),
      vec![format!(
        "https://mirror.example/v139.0.0/{}",
        TEST_VARS.file
      )]
    );
  }

  #[test]
  fn test_candidate_urls_filesystem_mirror() {
    // Filesystem mirrors additionally try the flat `{base}/{file}` layout.
    assert_eq!(
      candidate_urls(Some("/opt/cache"), false, &TEST_VARS),
      vec![
        format!("/opt/cache/v139.0.0/{}", TEST_VARS.file),
        format!("/opt/cache/{}", TEST_VARS.file),
      ]
    );
  }

  #[test]
  fn test_candidate_urls_template_mirror() {
    // A template mirror is used verbatim after substitution; no tag/flat
    // variants are derived from it.
    assert_eq!(
      candidate_urls(
        Some("https://mirror.example/artifacts/{version}/{file}"),
        false,
        &TEST_VARS
      ),
      vec![format!(
        "https://mirror.example/artifacts/140.0.0/{}",
        TEST_VARS.file
      )]
    );
  }

  #[test]
  fn test_candidate_urls_mirror_fallback() {
    // RUSTY_V8_MIRROR_FALLBACK=1 appends the upstream release after the
    // mirror entries.
    assert_eq!(
      candidate_urls(Some("/opt/cache"), true, &TEST_VARS),
      vec![
        format!("/opt/cache/v139.0.0/{}", TEST_VARS.file),
        format!("/opt/cache/{}", TEST_VARS.file),
        format!("{DEFAULT_ARTIFACT_BASE}/v139.0.0/{}", TEST_VARS.file),
      ]
    );
    // Without a mirror the flag changes nothing: upstream is the only
    // candidate either way.
    assert_eq!(
      candidate_urls(None, true, &TEST_VARS),
      candidate_urls(None, false, &TEST_VARS)
    );
  }

  #[test]
  fn test_explicit_linux_bindgen_args_for_glibc_cross_compile() {
    assert_eq!(
      explicit_linux_bindgen_args(
        "gnu",
        Some("/opt/llvm/lib/clang/21"),
        None,
        Some("/opt/aarch64-linux-gnu"),
      ),
      vec![
        "-resource-dir=/opt/llvm/lib/clang/21",
        "-isystem/opt/aarch64-linux-gnu/include",
      ]
    );
  }

  #[test]
  fn test_explicit_linux_bindgen_args_keep_libc_inputs_target_scoped() {
    assert_eq!(
      explicit_linux_bindgen_args(
        "musl",
        Some("/opt/llvm/lib/clang/21"),
        Some("/opt/musl-sysroot"),
        Some("/opt/glibc-prefix"),
      ),
      vec![
        "-resource-dir=/opt/llvm/lib/clang/21",
        "--sysroot=/opt/musl-sysroot",
      ]
    );
    assert!(
      explicit_linux_bindgen_args(
        "uclibc",
        None,
        Some("/opt/musl-sysroot"),
        Some("/opt/glibc-prefix"),
      )
      .is_empty()
    );
  }

  #[test]
  fn test_explicit_linux_bindgen_args_for_resource_dir_only() {
    assert_eq!(
      explicit_linux_bindgen_args(
        "gnu",
        Some("/opt/llvm/lib/clang/21"),
        None,
        None,
      ),
      vec!["-resource-dir=/opt/llvm/lib/clang/21"]
    );
  }
}
