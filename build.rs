// Copyright 2018-2019 the Deno authors. All rights reserved. MIT license.
use fslock::LockFile;
use std::env;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::process::exit;
use std::process::Command;
use which::which;

fn main() {
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
    .map(|s| s.starts_with("rls"))
    .unwrap_or(false);

  if !(is_trybuild || is_cargo_doc | is_rls) {
    if env::var_os("V8_FROM_SOURCE").is_some() {
      build_v8()
    } else {
      // utilize a lockfile to prevent linking of
      // only partially downloaded static library.
      let root = env::current_dir().unwrap();
      let out_dir = env::var_os("OUT_DIR").unwrap();
      let lockfilepath = root
        .join(out_dir)
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("lib_download.fslock");
      println!("download lockfile: {:?}", &lockfilepath);
      let mut lockfile = LockFile::open(&lockfilepath)
        .expect("Couldn't open lib download lockfile.");
      lockfile.lock().expect("Couldn't get lock");
      download_static_lib_binaries();
      lockfile.unlock().expect("Couldn't unlock lockfile");
    }
  }

  if !(is_cargo_doc || is_rls) {
    print_link_flags()
  }
}

fn build_v8() {
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

  if need_gn_ninja_download() {
    download_ninja_gn_binaries();
  }

  // On windows, rustc cannot link with a V8 debug build.
  let mut gn_args = if cargo_gn::is_debug() && !cfg!(target_os = "windows") {
    vec!["is_debug=true".to_string()]
  } else {
    vec!["is_debug=false".to_string()]
  };

  if !cargo_gn::is_debug() {
    gn_args.push("v8_enable_handle_zapping=false".to_string());
  }

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
  } else if let Some(p) = env::var_os("CCACHE") {
    cc_wrapper(&mut gn_args, &Path::new(&p));
  } else if let Ok(p) = which("ccache") {
    cc_wrapper(&mut gn_args, &p);
  } else {
    println!("cargo:warning=Not using sccache or ccache");
  }

  if let Ok(args) = env::var("GN_ARGS") {
    for arg in args.split_whitespace() {
      gn_args.push(arg.to_string());
    }
  }

  let target_triple = env::var("TARGET").unwrap();
  // check if the target triple describes a non-native environment
  if target_triple != env::var("HOST").unwrap() {
    // cross-compilation setup
    if target_triple == "aarch64-unknown-linux-gnu" {
      gn_args.push(r#"target_cpu="arm64""#.to_string());
      gn_args.push("use_sysroot=true".to_string());
      maybe_install_sysroot("arm64");
      maybe_install_sysroot("amd64");
    };
  }

  let gn_root = env::var("CARGO_MANIFEST_DIR").unwrap();

  let gn_out = cargo_gn::maybe_gen(&gn_root, gn_args);
  assert!(gn_out.exists());
  assert!(gn_out.join("args.gn").exists());
  cargo_gn::build("rusty_v8", None);
}

fn maybe_install_sysroot(arch: &str) {
  let sysroot_path = format!("build/linux/debian_sid_{}-sysroot", arch);
  if !PathBuf::from(sysroot_path).is_dir() {
    let status = Command::new("python")
      .arg("./build/linux/sysroot_scripts/install-sysroot.py")
      .arg(format!("--arch={}", arch))
      .status()
      .unwrap_or_else(|_| panic!("sysroot download failed: {}", arch));
    assert!(status.success());
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

fn download_ninja_gn_binaries() {
  let target_dir = build_dir();
  let bin_dir = target_dir
    .join("ninja_gn_binaries-20200827")
    .join(platform());
  let gn = bin_dir.join("gn");
  let ninja = bin_dir.join("ninja");
  #[cfg(windows)]
  let gn = gn.with_extension("exe");
  #[cfg(windows)]
  let ninja = ninja.with_extension("exe");

  if !gn.exists() || !ninja.exists() {
    let status = Command::new("python")
      .arg("./tools/ninja_gn_binaries.py")
      .arg("--dir")
      .arg(&target_dir)
      .status()
      .expect("ninja_gn_binaries.py download failed");
    assert!(status.success());
  }
  assert!(gn.exists());
  assert!(ninja.exists());
  env::set_var("GN", gn);
  env::set_var("NINJA", ninja);
}

fn static_lib_url() -> String {
  let default_base = "https://github.com/denoland/rusty_v8/releases/download";
  let base =
    env::var("RUSTY_V8_MIRROR").unwrap_or_else(|_| default_base.into());
  let version = env::var("CARGO_PKG_VERSION").unwrap();
  let target = env::var("TARGET").unwrap();
  if cfg!(target_os = "windows") {
    // Note: we always use the release build on windows.
    format!("{}/v{}/rusty_v8_release_{}.lib", base, version, target)
  } else {
    let profile = env::var("PROFILE").unwrap();
    assert!(profile == "release" || profile == "debug");
    format!("{}/v{}/librusty_v8_{}_{}.a", base, version, profile, target)
  }
}

fn static_lib_name() -> &'static str {
  match cfg!(target_os = "windows") {
    true => "rusty_v8.lib",
    false => "librusty_v8.a",
  }
}

fn static_lib_path() -> PathBuf {
  static_lib_dir().join(static_lib_name())
}

fn static_lib_dir() -> PathBuf {
  build_dir().join("gn_out").join("obj")
}

fn build_dir() -> PathBuf {
  let root = env::current_dir().unwrap();

  // target/debug//build/rusty_v8-d9e5a424d4f96994/out/
  let out_dir = env::var_os("OUT_DIR").expect(
    "The 'OUT_DIR' environment is not set (it should be something like \
     'target/debug/rusty_v8-{hash}').",
  );
  let out_dir_abs = root.join(out_dir);

  // This would be target/debug or target/release
  out_dir_abs
    .parent()
    .unwrap()
    .parent()
    .unwrap()
    .parent()
    .unwrap()
    .to_path_buf()
}

fn download_file(url: String, filename: PathBuf) {
  if !url.starts_with("http:") && !url.starts_with("https:") {
    fs::copy(&url, filename).unwrap();
    return;
  }

  // Try downloading with python first. Python is a V8 build dependency,
  // so this saves us from adding a Rust HTTP client dependency.
  println!("Downloading {}", url);
  let status = Command::new("python")
    .arg("./tools/download_file.py")
    .arg("--url")
    .arg(&url)
    .arg("--filename")
    .arg(&filename)
    .status();

  // Python is only a required dependency for `V8_FROM_SOURCE` builds.
  // If python is not available, try falling back to curl.
  let status = match status {
    Ok(status) if status.success() => status,
    _ => {
      println!("Python downloader failed, trying with curl.");
      Command::new("curl")
        .arg("-L")
        .arg("-s")
        .arg("-o")
        .arg(&filename)
        .arg(&url)
        .status()
        .unwrap()
    }
  };

  assert!(status.success());
  assert!(filename.exists());
}

fn download_static_lib_binaries() {
  let url = static_lib_url();
  println!("static lib URL: {}", url);

  let dir = static_lib_dir();
  std::fs::create_dir_all(&dir).unwrap();
  println!("cargo:rustc-link-search={}", dir.display());

  let filename = static_lib_path();
  if filename.exists() {
    println!("static lib already exists {}", filename.display());
    println!("To re-download this file, it must be manually deleted.");
  } else {
    download_file(url, filename);
  }
}

fn print_link_flags() {
  println!("cargo:rustc-link-lib=static=rusty_v8");

  if cfg!(target_os = "windows") {
    println!("cargo:rustc-link-lib=dylib=winmm");
    println!("cargo:rustc-link-lib=dylib=dbghelp");
  }
}

fn need_gn_ninja_download() -> bool {
  let has_ninja = which("ninja").is_ok() || env::var_os("NINJA").is_some();
  let has_gn = which("gn").is_ok() || env::var_os("GN").is_some();

  return !has_ninja || !has_gn;
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

  println!("using Chromiums clang");
  None
}

// Download chromium's clang into OUT_DIR because Cargo will not allow us to
// modify the source directory.
fn clang_download() -> PathBuf {
  let clang_base_path = build_dir().join("clang");
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
