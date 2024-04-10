// Don't run UI tests on emulated environment or nightly build.
#[cfg(not(target_os = "android"))]
#[rustversion::attr(not(nightly), test)]
fn ui() {
  // This environment variable tells build.rs that we're running trybuild tests,
  // so it won't rebuild V8.
  std::env::set_var("DENO_TRYBUILD", "1");

  let t = trybuild::TestCases::new();
  t.compile_fail("tests/compile_fail/*.rs");
}
