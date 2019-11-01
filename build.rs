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

  // gn_root should be absolute.
  let gn_root = env::current_dir()
    .unwrap()
    .into_os_string()
    .into_string()
    .unwrap();;

  let gn_out = cargo_gn::maybe_gen(gn_root, gn_args);
  assert!(gn_out.exists());
  assert!(gn_out.join("args.gn").exists());
  cargo_gn::build("rusty_v8");
  println!("cargo:rustc-link-lib=static=rusty_v8");
  if cfg!(target_os = "windows") {
    println!("cargo:rustc-link-lib=dylib=winmm");
  }
}

// TODO(ry) Remove
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
    .arg("--init")
    .status()
    .expect("git submodule update failed");
}

fn gclient_sync() {
  let root = env::current_dir().unwrap();
  let third_party = root.join("third_party");
  let gclient_rel = PathBuf::from("depot_tools/gclient.py");

  if !third_party.join(&gclient_rel).exists() {
    git_submodule_update();
  }
  disable_depot_tools_update();
  // Command::new(gclient config http://src.chromium.org/svn/trunk/src

  println!(
    "cargo:warning=Running gclient sync to download V8. This could take a while."
  );

  let mut cmd = Command::new("python");
  cmd.current_dir(&third_party);
  cmd.arg(&gclient_rel);
  cmd.arg("sync");
  cmd.arg("--no-history");
  cmd.arg("--shallow");
  // cmd.arg("--verbose");
  cmd.env("DEPOT_TOOLS_UPDATE", "0");
  cmd.env("DEPOT_TOOLS_METRICS", "0");
  cmd.env("GCLIENT_FILE", third_party.join("gclient_config.py"));
  // We're not using Google's internal infrastructure.
  cmd.env("DEPOT_TOOLS_WIN_TOOLCHAIN", "0");

  println!("running: {:?}", cmd);
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
