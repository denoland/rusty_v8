// Measures Rust->V8 string creation: new_from_utf8 (validates UTF-8 + picks
// one-byte/two-byte) vs new_from_one_byte (skips validation) for ASCII input.
use std::time::Instant;

fn main() {
  let platform = v8::new_default_platform(0, false).make_shared();
  v8::V8::initialize_platform(platform);
  v8::V8::initialize();
  let isolate = &mut v8::Isolate::new(v8::CreateParams::default());
  v8::scope!(let handle_scope, isolate);
  let context = v8::Context::new(handle_scope, Default::default());
  let scope = &mut v8::ContextScope::new(handle_scope, context);

  let cases: &[(&str, String)] = &[
    ("ascii_8", "abcdefgh".to_string()),
    ("ascii_16", "a".repeat(16)),
    ("ascii_32", "a".repeat(32)),
    ("ascii_64", "a".repeat(64)),
  ];

  // Correctness gate: new_from_utf8 (now with the ASCII fast path) must
  // round-trip every input, including non-ASCII (which must NOT take the fast
  // path).
  for reference in [
    "hello world",
    "café \u{00e9}\u{00ff}", // Latin-1
    "Hello 🦕 世界!",        // multi-byte / two-byte
    "mixed ascii and 日本語 text",
  ] {
    v8::scope!(let hs, scope);
    let local = v8::String::new_from_utf8(
      hs,
      reference.as_bytes(),
      v8::NewStringType::Normal,
    )
    .unwrap();
    let got = local.to_rust_string_lossy(hs);
    assert_eq!(&got, reference, "new_from_utf8 round-trip mismatch");
  }
  println!("correctness: new_from_utf8 round-trips OK");

  let runs = 1_000_000u64;
  for (name, s) in cases {
    let bytes = s.as_bytes();
    // new_from_utf8
    let t0 = Instant::now();
    for _ in 0..runs {
      v8::scope!(let hs, scope);
      let local =
        v8::String::new_from_utf8(hs, bytes, v8::NewStringType::Normal)
          .unwrap();
      std::hint::black_box(local);
    }
    let utf8 = t0.elapsed().as_nanos() as f64 / runs as f64;
    // new_from_one_byte
    let t1 = Instant::now();
    for _ in 0..runs {
      v8::scope!(let hs, scope);
      let local =
        v8::String::new_from_one_byte(hs, bytes, v8::NewStringType::Normal)
          .unwrap();
      std::hint::black_box(local);
    }
    let one = t1.elapsed().as_nanos() as f64 / runs as f64;
    println!(
      "  {name:10}  utf8={utf8:6.2}ns  one_byte={one:6.2}ns  ({:+.1}%)",
      (one - utf8) / utf8 * 100.0
    );
  }
}
