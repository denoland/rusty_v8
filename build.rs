use std::env;
use std::env::consts;
use std::ffi::OsString;
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

  build_v8(crate_dir, out_dir, gn_out_dir);
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

fn build_v8(crate_dir: PathBuf, out_dir: PathBuf, gn_out_dir: PathBuf) {
  env::set_var("DEPOT_TOOLS_WIN_TOOLCHAIN", "0"); // google uses internal visual studio by default
  env::set_var("PYTHONDONTWRITEBYTECODE", "1"); // disable __pycache__

  let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap();
  let target_arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap();
  let gn_host_os = gn_os(consts::OS);
  let gn_host_arch = gn_arch(consts::ARCH);
  let gn_target_os = gn_os(&target_os);
  let gn_target_arch = gn_arch(&target_arch);

  let is_debug = if cfg!(target_os = "windows") {
    false
  } else {
    match env::var("PROFILE").unwrap().as_str() {
      "debug" => true,
      _ => false,
    }
  };

  let use_custom_libcxx = cfg!(feature = "use_custom_libcxx");

  let python = find_python().expect("Can't find python");
  system!(
    &python,
    [
      crate_dir
        .join("scripts/v8_build.py")
        .to_string_lossy()
        .as_ref(),
      "--crate-root",
      crate_dir.to_string_lossy().as_ref(),
      "--root",
      out_dir.to_string_lossy().as_ref(),
      "--gn-root",
      gn_out_dir.to_string_lossy().as_ref(),
      "--host-os",
      gn_host_os,
      "--host-cpu",
      gn_host_arch,
      "--target-os",
      gn_target_os,
      "--target-cpu",
      gn_target_arch,
      "--is-debug",
      is_debug.to_string().as_ref(),
      "--use-custom-libcxx",
      use_custom_libcxx.to_string().as_ref(),
    ]
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
