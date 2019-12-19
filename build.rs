// Copyright 2018-2019 the Deno authors. All rights reserved. MIT license.
use cargo_gn;
use regex::Regex;
use std::env;
use std::path::Path;
use std::path::PathBuf;
use std::process::exit;
use std::process::Command;
use which::which;

const MIN_APPLE_CLANG_VER: f32 = 11.0;
const MIN_LLVM_CLANG_VER: f32 = 9.0;

fn main() {
  env::set_var("DEPOT_TOOLS_WIN_TOOLCHAIN", "0");

  // cargo publish doesn't like pyc files.
  env::set_var("PYTHONDONTWRITEBYTECODE", "1");

  // git submodule update --init --recursive
  let libcxx_src = PathBuf::from("buildtools/third_party/libc++/trunk/src");
  if !libcxx_src.is_dir() {
    eprintln!(
      "missing source code. Run 'git submodule update --init --recursive'"
    );
    exit(1);
  }

  // Don't build if "cargo doc" is being run. This is to support docs.rs.
  if env::var_os("RUSTDOCFLAGS").is_some() {
    exit(0);
  }

  if need_gn_ninja_download() {
    download_gn_ninja_binaries();
  }

  // On windows, rustc cannot link with a V8 debug build.
  let mut gn_args = if cargo_gn::is_debug() && !cfg!(target_os = "windows") {
    vec!["is_debug=true".to_string()]
  } else {
    vec!["is_debug=false".to_string()]
  };

  if let Some(clang_base_path) = find_compatible_system_clang() {
    println!("clang_base_path {}", clang_base_path.display());
    gn_args.push(format!("clang_base_path={:?}", clang_base_path));
    // TODO: Dedupe this with the one from cc_wrapper()
    gn_args.push("treat_warnings_as_errors=false".to_string());
    // we can't use chromiums clang plugins with a system clang
    gn_args.push("clang_use_chrome_plugins=false".to_string());
  } else {
    let clang_base_path = clang_download();
    gn_args.push(format!("clang_base_path={:?}", clang_base_path));
  }

  if let Some(p) = env::var_os("SCCACHE") {
    cc_wrapper(&mut gn_args, &Path::new(&p));
  } else if let Ok(p) = which("sccache") {
    cc_wrapper(&mut gn_args, &p);
  } else {
    println!("cargo:warning=Not using sccache");
  }

  if let Ok(args) = env::var("GN_ARGS") {
    for arg in args.split_whitespace() {
      gn_args.push(arg.to_string());
    }
  }

  let gn_root = env::var("CARGO_MANIFEST_DIR").unwrap();

  let gn_out = cargo_gn::maybe_gen(&gn_root, gn_args);
  assert!(gn_out.exists());
  assert!(gn_out.join("args.gn").exists());
  cargo_gn::build("rusty_v8");

  println!("cargo:rustc-link-lib=static=rusty_v8");

  if cfg!(target_os = "windows") {
    println!("cargo:rustc-link-lib=dylib=winmm");
    println!("cargo:rustc-link-lib=dylib=dbghelp");
  }
}

fn platform() -> &'static str {
  #[cfg(target_os = "windows")]
  {
    "win"
  }
  #[cfg(target_os = "linux")]
  {
    "linux64"
  }
  #[cfg(target_os = "macos")]
  {
    "mac"
  }
}

fn download_gn_ninja_binaries() {
  let root = env::current_dir().unwrap();
  let out_dir = root.join(env::var_os("OUT_DIR").unwrap());
  let d = out_dir.join("gn_ninja_binaries").join(platform());
  let gn = d.join("gn");
  let ninja = d.join("ninja");
  #[cfg(windows)]
  let gn = gn.with_extension("exe");
  #[cfg(windows)]
  let ninja = ninja.with_extension("exe");

  if !gn.exists() || !ninja.exists() {
    let status = Command::new("python")
      .arg("./tools/gn_ninja_binaries.py")
      .arg("--dir")
      .arg(&out_dir)
      .status()
      .expect("gn_ninja_binaries.py download failed");
    assert!(status.success());
  }
  assert!(gn.exists());
  assert!(ninja.exists());
  env::set_var("GN", gn);
  env::set_var("NINJA", ninja);
}

fn need_gn_ninja_download() -> bool {
  !((which("ninja").is_ok() || env::var_os("NINJA").is_some())
    && env::var_os("GN").is_some())
}

fn is_compatible_clang_version(clang_path: &Path) -> bool {
  let apple_clang_re =
    Regex::new(r"(^Apple (?:clang|LLVM) version) ([0-9]+\.[0-9]+)").unwrap();
  let llvm_clang_re =
    Regex::new(r"(^(?:FreeBSD )?clang version|based on LLVM) ([0-9]+\.[0-9]+)")
      .unwrap();

  if let Ok(o) = Command::new(clang_path).arg("--version").output() {
    let output = String::from_utf8(o.stdout).unwrap();
    if let Some(clang_ver) = apple_clang_re.captures(&output) {
      let ver: f32 = clang_ver.get(2).unwrap().as_str().parse().unwrap();
      if ver >= MIN_APPLE_CLANG_VER {
        println!("using Apple clang v{}", ver);
        return true;
      }
    }
    if let Some(clang_ver) = llvm_clang_re.captures(&output) {
      let ver: f32 = clang_ver.get(2).unwrap().as_str().parse().unwrap();
      if ver >= MIN_LLVM_CLANG_VER {
        println!("using LLVM clang v{}", ver);
        return true;
      }
    }
  }
  false
}

fn find_compatible_system_clang() -> Option<PathBuf> {
  if let Ok(p) = env::var("CLANG_BASE_PATH") { // check environment variable
    let base_path = Path::new(&p);
    let clang_path = base_path.join("bin").join("clang");
    if is_compatible_clang_version(&clang_path) {
      return Some(base_path.to_path_buf());
    }
  }

  if let Ok(clang_path) = which("clang") { // check clang in PATH
    if is_compatible_clang_version(&clang_path) {
      return Some(
        clang_path.parent().unwrap().parent().unwrap().to_path_buf(),
      );
    }
  }

  println!("using Chromiums clang");
  None
}

// Download chromium's clang into OUT_DIR because Cargo will not allow us to
// modify the source directory.
fn clang_download() -> PathBuf {
  let root = env::current_dir().unwrap();
  let out_dir = env::var_os("OUT_DIR").unwrap();
  let clang_base_path = root.join(out_dir).join("clang");
  println!("clang_base_path {}", clang_base_path.display());
  let status = Command::new("python")
    .arg("./tools/clang/scripts/update.py")
    .arg("--output-dir")
    .arg(&clang_base_path)
    .status()
    .expect("clang download failed");
  assert!(status.success());
  assert!(clang_base_path.exists());
  clang_base_path
}

fn cc_wrapper(gn_args: &mut Vec<String>, sccache_path: &Path) {
  gn_args.push(format!("cc_wrapper={:?}", sccache_path));
  // Disable treat_warnings_as_errors until this sccache bug is fixed:
  // https://github.com/mozilla/sccache/issues/264
  if cfg!(target_os = "windows") {
    gn_args.push("treat_warnings_as_errors=false".to_string());
  }
}
