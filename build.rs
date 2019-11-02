// Copyright 2018-2019 the Deno authors. All rights reserved. MIT license.
use cargo_gn;
use std::env;
use std::path::Path;
use std::process::Command;
use which::which;

fn main() {
  env::set_var("DEPOT_TOOLS_UPDATE", "0");
  env::set_var("DEPOT_TOOLS_METRICS", "0");

  setup_depot_tools_path();
  if cfg!(windows) {
    init_depot_tools_windows();
  }
  if !Path::new("third_party/v8/src").is_dir()
    || env::var_os("GCLIENT_SYNC").is_some()
  {
    gclient_sync();
  }

  // On windows, rustc cannot link with a V8 debug build.
  let mut gn_args = if cargo_gn::is_debug() && !cfg!(target_os = "windows") {
    vec!["is_debug=true".to_string()]
  } else {
    vec!["is_debug=false".to_string()]
  };

  if let Some(p) = env::var_os("SCCACHE") {
    cc_wrapper(&mut gn_args, &Path::new(&p));
  } else if let Ok(p) = which("sccache") {
    cc_wrapper(&mut gn_args, &p);
  } else {
    println!("cargo:warning=Not using sccache");
  }

  // gn_root needs to be an absolute path.
  let gn_root = env::current_dir()
    .unwrap()
    .into_os_string()
    .into_string()
    .unwrap();

  let gn_out = cargo_gn::maybe_gen(&gn_root, gn_args);
  assert!(gn_out.exists());
  assert!(gn_out.join("args.gn").exists());
  cargo_gn::build("rusty_v8");

  println!("cargo:rustc-link-lib=static=rusty_v8");

  if cfg!(target_os = "windows") {
    println!("cargo:rustc-link-lib=dylib=winmm");
  }
}

fn setup_depot_tools_path() {
  let depot_tools = env::current_dir()
    .unwrap()
    .join("third_party")
    .join("depot_tools");

  // Add third_party/depot_tools and buildtools/win to PATH.
  // TODO: this should be done on all platforms.
  // TODO: buildtools/win should not be added; instead, cargo_gn should invoke
  // depot_tools/gn.bat.
  let buildtools_win =
    env::current_dir().unwrap().join("buildtools").join("win");

  // Bootstrap depot_tools.
  let path = env::var_os("PATH").unwrap();

  // "Add depot_tools to the start of your PATH (must be ahead of any installs
  // of Python)."
  // https://commondatastorage.googleapis.com/chrome-infra-docs/flat/depot_tools/docs/html/depot_tools_tutorial.html#_setting_up
  let paths = env::split_paths(&path)
    .into_iter()
    .chain(vec![depot_tools, buildtools_win])
    .collect::<Vec<_>>();
  let path = env::join_paths(paths).unwrap();
  env::set_var("PATH", &path);
}

fn init_depot_tools_windows() {
  let depot_tools = env::current_dir()
    .unwrap()
    .join("third_party")
    .join("depot_tools");
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

  // TODO: cargo_gn should do this.
  env::set_var("DEPOT_TOOLS_WIN_TOOLCHAIN", "0");
}

fn gclient_sync() {
  let root = env::current_dir().unwrap();
  let third_party = root.join("third_party");
  let depot_tools = third_party.join("depot_tools");
  let gclient_file = third_party.join("gclient_config.py");

  let gclient = depot_tools.join(if cfg!(windows) {
    "gclient.bat"
  } else {
    "gclient"
  });
  if !gclient.is_file() {
    panic!(
      "Could not find gclient {}. Maybe run git submodule update?",
      gclient.display()
    );
  }

  println!("Running gclient sync to download V8. This could take a while.");

  let status = Command::new(gclient)
    .current_dir(&third_party)
    .arg("sync")
    .arg("--no-history")
    .arg("--shallow")
    .env("GCLIENT_FILE", gclient_file)
    .status()
    .expect("gclient sync failed");
  assert!(status.success());
}

fn cc_wrapper(gn_args: &mut Vec<String>, sccache_path: &Path) {
  gn_args.push(format!("cc_wrapper={:?}", sccache_path));

  // Disable treat_warnings_as_errors until this sccache bug is fixed:
  // https://github.com/mozilla/sccache/issues/264
  if cfg!(target_os = "windows") {
    gn_args.push("treat_warnings_as_errors=false".to_string());
  }
}
