// Copyright 2018-2019 the Deno authors. All rights reserved. MIT license.
use cargo_gn;
use std::env;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use which::which;

fn main() {
  init_depot_tools();

  set_gn_ninja_vars();

  // On windows, rustc cannot link with a V8 debug build.
  let mut gn_args = if cargo_gn::is_debug() && !cfg!(target_os = "windows") {
    vec!["is_debug=true".to_string()]
  } else {
    vec!["is_debug=false".to_string()]
  };

  clang_download(&mut gn_args);

  if let Some(p) = env::var_os("SCCACHE") {
    cc_wrapper(&mut gn_args, &Path::new(&p));
  } else if let Ok(p) = which("sccache") {
    cc_wrapper(&mut gn_args, &p);
  } else {
    println!("cargo:warning=Not using sccache");
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

#[cfg(target_os = "macos")]
fn get_buildtools() -> PathBuf {
  let root = env::current_dir().unwrap();
  root.join("buildtools").join("mac")
}

#[cfg(target_os = "linux")]
fn get_buildtools() -> PathBuf {
  let root = env::current_dir().unwrap();
  root.join("buildtools").join("linux64")
}

fn set_gn_ninja_vars() {
  let buildtools = get_buildtools();
  let gn = buildtools.join("gn");
  let ninja = buildtools.join("ninja");
  env::set_var("GN", gn);
  env::set_var("NINJA", ninja);
}

// Download chromium's clang into OUT_DIR because Cargo will not allow us to
// modify the source directory.
fn clang_download(gn_args: &mut Vec<String>) {
  let root = env::current_dir().unwrap();
  let out_dir = env::var_os("OUT_DIR").unwrap();
  let clang_base_path = root.join(out_dir).join("clang");
  println!("clang_base_path {}", clang_base_path.display());
  let status = Command::new("python")
    .arg("./tools/clang/scripts/update.py")
    .arg("--clang-dir")
    .arg(&clang_base_path)
    .status()
    .expect("clang download failed");
  assert!(status.success());

  gn_args.push(format!("clang_base_path={:?}", clang_base_path));
}

fn init_depot_tools() {
  env::set_var("DEPOT_TOOLS_WIN_TOOLCHAIN", "0");
  env::set_var("DEPOT_TOOLS_UPDATE", "0");
  env::set_var("DEPOT_TOOLS_METRICS", "0");

  let depot_tools = env::current_dir()
    .unwrap()
    .join("third_party")
    .join("depot_tools");

  if cfg!(windows) {
    // Bootstrap depot_tools.
    if !depot_tools.join("git.bat").is_file() {
      let status = Command::new("cmd.exe")
        .arg("/c")
        .arg("bootstrap\\win_tools.bat")
        .current_dir(&depot_tools)
        .status()
        .expect("bootstrapping depot_tools failed");
      assert!(status.success());
    }
  }
}

fn cc_wrapper(gn_args: &mut Vec<String>, sccache_path: &Path) {
  gn_args.push(format!("cc_wrapper={:?}", sccache_path));
  // Disable treat_warnings_as_errors until this sccache bug is fixed:
  // https://github.com/mozilla/sccache/issues/264
  if cfg!(target_os = "windows") {
    gn_args.push("treat_warnings_as_errors=false".to_string());
  }
}
