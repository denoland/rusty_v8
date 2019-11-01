// Copyright 2018-2019 the Deno authors. All rights reserved. MIT license.
use cargo_gn;
use std::env;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use which::which;

fn main() {
  if !Path::new("third_party/v8/src").is_dir()
    || env::var_os("GCLIENT_SYNC").is_some()
  {
    gclient_sync();
  }

  let mut gn_args = if cargo_gn::is_debug() {
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

fn git_submodule_update() {
  Command::new("git")
    .arg("submodule")
    .arg("update")
    .arg("--init")
    .status()
    .expect("git submodule update failed");
}

fn gclient_sync() {
  let root = env::current_dir().unwrap();
  let third_party = root.join("third_party");
  let gclient_rel = PathBuf::from("depot_tools/gclient.py");
  let gclient_file = third_party.join("gclient_config.py");
  assert!(gclient_file.exists());

  if !third_party.join(&gclient_rel).exists() {
    git_submodule_update();
  }

  println!("Running gclient sync to download V8. This could take a while.");

  let status = Command::new("python")
    .current_dir(&third_party)
    .arg(&gclient_rel)
    .arg("sync")
    .arg("--no-history")
    .arg("--shallow")
    .env("DEPOT_TOOLS_UPDATE", "0")
    .env("DEPOT_TOOLS_METRICS", "0")
    .env("GCLIENT_FILE", gclient_file)
    .env("DEPOT_TOOLS_WIN_TOOLCHAIN", "0")
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
