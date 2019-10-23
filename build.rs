use cc;

fn main() {
  cc::Build::new()
    .cpp(true)
    .flag("-std:c++17")
    .debug(true)
    .file("src/inspector/channel.cpp")
    .file("src/inspector/client.cpp")
    .file("src/platform/task.cpp")
    .file("src/string_buffer.cpp")
    .compile("v8-bindings");

  println!("cargo:rustc-link-lib=static=v8_monolith");
  println!("cargo:rustc-link-search=goog/v8/out/x64.release/obj");

  if cfg!(target_os = "windows") {
    println!("cargo:rustc-link-lib=dylib=winmm");
  }
}
