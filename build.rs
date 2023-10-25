use std::collections::HashMap;
use std::env;
use std::env::consts;
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::process::Command;
use which::which;

macro_rules! system {
  ($bin:expr, $args:expr) => {
    println!("[*] Executing {:?} {:?}", $bin, $args);
    assert!(Command::new($bin)
      .args($args)
      .spawn()
      .unwrap()
      .wait()
      .unwrap()
      .success());
  };
}
macro_rules! s {
  ($expr:expr) => {
    $expr.to_string()
  };
}
macro_rules! q {
  ($expr:expr) => {
    format!("\"{}\"", $expr)
  };
}

fn main() {
  set_cargo_rerun();

  if env::var_os("DOCS_RS").is_some()
    || env::var_os("CARGO")
      .map(PathBuf::from)
      .as_ref()
      .and_then(|p| p.file_stem())
      .and_then(|f| f.to_str())
      .map(|s| s.starts_with("rls"))
      .unwrap_or(false)
  {
    println!("[*] Docs.rs or RLS build, exiting");
    return;
  }

  let crate_dir = env::var_os("CARGO_MANIFEST_DIR")
    .map(PathBuf::from)
    .unwrap();
  let out_dir = env::var_os("OUT_DIR").map(PathBuf::from).unwrap();
  let gn_out_dir = out_dir.join("out");

  set_cargo_link_flags(&gn_out_dir);

  if env::var_os("DENO_TRYBUILD").is_some() {
    println!("[*] Deno TRYBUILD, exiting");
    return;
  }

  build_v8(
    crate_dir,
    out_dir,
    gn_out_dir.as_os_str().try_into().unwrap(),
  );
}

fn gn_os<'a>(os: &'a str) -> &'a str {
  match os {
    "windows" => "win",
    "macos" => "mac",
    _ => os,
  }
}

fn gn_arch<'a>(arch: &'a str) -> &'a str {
  match arch {
    "x86_64" => "x64",
    "x86" => "x86",
    "aarch64" => "arm64",
    "arm" => "arm",
    "i686" => "x86",
    _ => arch,
  }
}

fn build_v8(crate_dir: PathBuf, out_dir: PathBuf, gn_out_dir: &str) {
  env::set_var("DEPOT_TOOLS_WIN_TOOLCHAIN", "0"); // google uses internal visual studio by default
  env::set_var("PYTHONDONTWRITEBYTECODE", "1"); // disable __pycache__

  let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap();
  let target_arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap();
  let gn_host_os = gn_os(consts::OS);
  let gn_host_arch = gn_arch(consts::ARCH);
  let gn_target_os = gn_os(&target_os);
  let gn_target_arch = gn_arch(&target_arch);

  let mut gn_args = HashMap::<&str, String>::from([
    ("clang_use_chrome_plugins", s!(false)),
    ("is_component_build", s!(false)),
    ("linux_use_bundled_binutils", s!(false)),
    ("use_dummy_lastchange", s!(true)),
    ("use_sysroot", s!(false)),
    ("win_crt_flavor_agnostic", s!(true)),
    // Minimize size of debuginfo in distributed static library.
    ("line_tables_only", s!(true)),
    ("no_inline_line_tables", s!(true)),
    ("symbol_level", s!(1)),
    ("use_debug_fission", s!(false)),
    ("v8_enable_sandbox", s!(false)),
    ("v8_enable_snapshot_compression", s!(false)),
    ("v8_enable_javascript_promise_hooks", s!(true)),
    ("v8_promise_internal_field_count", s!(1)),
    ("v8_use_external_startup_data", s!(false)),
    ("v8_use_snapshot", s!(true)),
    // Disable handle zapping for performance
    ("v8_enable_handle_zapping", s!(false)),
    // Ensure allocation of typed arrays and arraybuffers always goes through
    // the embedder's ArrayBufferAllocator, otherwise small buffers get moved
    // around by the garbage collector but embedders normally want them to have
    // fixed addresses.
    ("v8_typed_array_max_size_in_heap", s!(0)),
    // Enabling the shared read-only heap comes with a restriction that all
    // isolates running at the same time must be created from the same snapshot.
    // This is problematic for Deno, which has separate "runtime" and "typescript
    // compiler" snapshots, and sometimes uses them both at the same time.
    ("v8_enable_shared_ro_heap", s!(false)),
    // V8 11.6 hardcoded an assumption in `mksnapshot` that shared RO heap
    // is enabled. In our case it's disabled so without this flag we can't
    // compile.
    ("v8_enable_verify_heap", s!(false)),
    // V8 introduced a bug in 11.1 that causes the External Pointer Table to never
    // be cleaned which causes resource exhaustion. Disabling pointer compression
    // makes sure that the EPT is not used.
    // https://bugs.chromium.org/p/v8/issues/detail?id=13640&q=garbage%20collection&can=2
    ("v8_enable_pointer_compression", s!(false)),
    // Maglev *should* be supported when pointer compression is disabled as per
    // https://chromium-review.googlesource.com/c/v8/v8/+/4753150, but it still
    // fails to compile.
    ("v8_enable_maglev", s!(false)),
  ]);

  let is_debug = if target_os == "windows" {
    false
  } else {
    match env::var("PROFILE").unwrap().as_str() {
      "debug" => true,
      _ => false,
    }
  };

  #[cfg(not(feature = "use_custom_libcxx"))]
  gn_args.insert("use_custom_libcxx", s!(false));

  gn_args.insert("is_debug", s!(is_debug));
  gn_args.insert("target_cpu", q!(gn_target_arch));
  gn_args.insert("v8_target_cpu", q!(gn_target_arch));

  if let Some(cc_wrapper) = find_cc_wrapper() {
    gn_args.insert("cc_wrapper", q!(cc_wrapper.to_string_lossy()));
  }

  if target_os != consts::OS {
    gn_args.insert("target_os", q!(gn_target_os));
  }

  let use_sysroot = target_arch != consts::ARCH;
  gn_args.insert("use_sysroot", s!(use_sysroot));

  if env::var_os("DISABLE_CLANG").is_some() {
    gn_args.insert("is_clang", s!(false));
    gn_args.insert("line_tables_only", s!(false));
  } else if let Ok(clang_base_path) = env::var("CLANG_BASE_PATH") {
    gn_args.insert("clang_base_path", q!(clang_base_path));
    gn_args.insert("treat_warnings_as_errors", s!(false));
  }

  let extra_gn_args = env::var("GN_ARGS").unwrap_or_default();

  for pair in extra_gn_args
    .split_whitespace()
    .map(|pair| pair.split_once("="))
  {
    if let Some((k, v)) = pair {
      gn_args.insert(k, v.to_string());
    }
  }

  let gn_args = gn_args
    .iter()
    .map(|(key, value)| format!("{key}={value}"))
    .collect::<Vec<String>>()
    .join(" ");

  env::set_var("GN_ARGS", &gn_args);

  println!("[*] crate_dir  : {}", crate_dir.display());
  println!("[*] out_dir    : {}", out_dir.display());
  println!("[*] gn_out_dir : {}", gn_out_dir);
  println!("[*] gn_args    : {}", gn_args);

  env::set_current_dir(&out_dir).unwrap();

  let python = find_python().expect("Can't find python");
  system!(
    &python,
    [
      crate_dir.join("scripts/download_v8.py").as_os_str(),
      crate_dir.as_os_str(),
      out_dir.as_os_str(),
      OsStr::new(gn_host_os),
      OsStr::new(gn_host_arch),
      OsStr::new(gn_target_os),
      OsStr::new(gn_target_arch),
      OsStr::new(&format!("--host-os={gn_host_os}")),
      OsStr::new(&format!("--host-cpu={gn_host_arch}")),
    ]
  );

  let gn = find_gn(&out_dir).expect("Can't find gn");
  let gn_se = format!("--script-executable={}", python.to_string_lossy());

  system!(
    &gn,
    [&gn_se, "gen", gn_out_dir, &format!("--args={gn_args}")]
  );

  if env::var_os("PRINT_GN_ARGS").is_some() {
    system!(&gn, [&gn_se, "args", gn_out_dir, "--list"]);
  }

  system!(
    find_ninja(&out_dir).expect("Can't find ninja"),
    ["-C", gn_out_dir, "rusty_v8"]
  );
}

fn set_cargo_rerun() {
  println!("cargo:rerun-if-changed=.gn");
  println!("cargo:rerun-if-changed=BUILD.gn");
  println!("cargo:rerun-if-changed=src");

  let envs = vec![
    "RUSTY_V8_ARCHIVE",
    "RUSTY_V8_MIRROR",
    "V8_FROM_SOURCE",
    "V8_FORCE_DEBUG",
    "DENO_TRYBUILD",
    "DOCS_RS",
    "CCACHE",
    "SCCACHE",
    "HOST",
    "OUT_DIR",
    "DISABLE_CLANG",
    "CLANG_BASE_PATH",
    "CXXSTDLIB",
    "NINJA",
    "PYTHON",
    "GN",
    "GN_ARGS",
    "PRINT_GN_ARGS",
  ];
  for env in envs {
    println!("cargo:rerun-if-env-changed={}", env);
  }
}

fn set_cargo_link_flags(gn_out_dir: &Path) {
  println!("cargo:rustc-link-lib=static=rusty_v8");

  let should_dyn_link_libcxx = cfg!(not(feature = "use_custom_libcxx"))
    || env::var("GN_ARGS").map_or(false, |gn_args| {
      gn_args
        .split_whitespace()
        .any(|ba| ba == "use_custom_libcxx=false")
    });

  if should_dyn_link_libcxx {
    // Based on https://github.com/alexcrichton/cc-rs/blob/fba7feded71ee4f63cfe885673ead6d7b4f2f454/src/lib.rs#L2462
    if let Ok(stdlib) = env::var("CXXSTDLIB") {
      if !stdlib.is_empty() {
        println!("cargo:rustc-link-lib=dylib={}", stdlib);
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

  if cfg!(target_os = "windows") {
    println!("cargo:rustc-link-lib=dylib=winmm");
    println!("cargo:rustc-link-lib=dylib=dbghelp");
  }

  if cfg!(target_env = "msvc") {
    // On Windows, including libcpmt[d]/msvcprt[d] explicitly links the C++
    // standard library, which libc++ needs for exception_ptr internals.
    if cfg!(target_feature = "crt-static") {
      println!("cargo:rustc-link-lib=libcpmt");
    } else {
      println!("cargo:rustc-link-lib=dylib=msvcprt");
    }
  }

  println!(
    "cargo:rustc-link-search=native={}",
    gn_out_dir.join("obj").display()
  );
}

fn find_python() -> Option<OsString> {
  Some(if let Some(path) = env::var_os("PYTHON") {
    path
  } else if let Ok(path) = which("vpython3") {
    path.into()
  } else if let Ok(path) = which("python3") {
    path.into()
  } else if let Ok(path) = which("python") {
    path.into()
  } else {
    return None;
  })
}

fn find_ninja(out_dir: &Path) -> Option<OsString> {
  let mut ninja_path = out_dir.join("third_party").join("ninja").join("ninja");
  ninja_path.set_extension(consts::EXE_EXTENSION);

  Some(if let Some(path) = env::var_os("NINJA") {
    path
  } else if ninja_path.exists() {
    ninja_path.into()
  } else if let Ok(path) = which("ninja") {
    path.into()
  } else {
    return None;
  })
}

fn find_gn(out_dir: &Path) -> Option<OsString> {
  let platform = match consts::OS {
    "linux" => "linux64",
    "windows" => "win",
    "macos" => "mac",
    _ => "unsupported",
  };
  let mut gn_path = out_dir.join("buildtools").join(platform).join("gn");
  gn_path.set_extension(consts::EXE_EXTENSION);

  Some(if let Some(path) = env::var_os("GN") {
    path
  } else if gn_path.exists() {
    gn_path.into()
  } else {
    return None;
  })
}

fn find_cc_wrapper() -> Option<OsString> {
  Some(if let Some(path) = env::var_os("SCCACHE") {
    path
  } else if let Ok(path) = which("sccache") {
    path.into()
  } else if let Some(path) = env::var_os("CCACHE") {
    path
  } else if let Ok(path) = which("ccache") {
    path.into()
  } else {
    return None;
  })
}
