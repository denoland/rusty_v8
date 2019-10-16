use cc;

fn main() {
  cc::Build::new()
    .cpp(true)
    .flag("-std:c++17")
    .debug(true)
    .file("src/lib.cpp")
    .compile("v8-bindings");
}
