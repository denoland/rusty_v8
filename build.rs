use cargo_gn;
use std::env;
use std::path::Path;
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

  let gn_out = cargo_gn::maybe_gen(".", gn_args);
  assert!(gn_out.exists());
  assert!(gn_out.join("args.gn").exists());
  cargo_gn::build("rusty_v8");
  println!("cargo:rustc-link-lib=static=rusty_v8");
  if cfg!(target_os = "windows") {
    println!("cargo:rustc-link-lib=dylib=winmm");
  }
}

fn disable_depot_tools_update() {
  Command::new("python")
    .arg("third_party/depot_tools/update_depot_tools_toggle.py")
    .arg("--disable")
    .status()
    .expect("update_depot_tools_toggle.py failed");
}

fn git_submodule_update() {
  println!("cargo:warning=Running git submodule update");
  Command::new("git")
    .arg("submodule")
    .arg("update")
    .status()
    .expect("git submodule update failed");
}

// ./depot_tools/gclient sync --gclientfile=gclient_config.py
fn gclient_sync() {
  if !Path::new("third_party/depot_tools/gclient").is_file() {
    // Need to run git submodule update.
    git_submodule_update();
  }
  disable_depot_tools_update();

  println!("cargo:warning=Running gcient sync to download V8. This could take a while.");
  let mut cmd = Command::new("./depot_tools/gclient");
  cmd.arg("sync");
  cmd.arg("--gclientfile=gclient_config.py");
  cmd.current_dir("third_party");
  let status = cmd.status().expect("gclient sync failed");
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
