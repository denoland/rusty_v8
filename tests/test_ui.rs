use std::env;

#[test]
fn ui() {
  // This environment variable tells build.rs that we're running trybuild tests,
  // so it won't rebuild V8.
  env::set_var("DENO_TRYBUILD", "1");

  let t = trybuild::TestCases::new();
  t.compile_fail("tests/compile_fail/*.rs");
}
