// Benchmark + correctness check for String::to_rust_string_lossy across the
// representations it special-cases: short/long ASCII (one-byte), Latin-1
// (one-byte non-ASCII), and two-byte (UTF-16).
use std::time::Instant;

fn main() {
  let platform = v8::new_default_platform(0, false).make_shared();
  v8::V8::initialize_platform(platform);
  v8::V8::initialize();
  let isolate = &mut v8::Isolate::new(v8::CreateParams::default());
  v8::scope!(let handle_scope, isolate);
  let context = v8::Context::new(handle_scope, Default::default());
  let scope = &mut v8::ContextScope::new(handle_scope, context);

  let long_latin1 = "\u{00e9}".repeat(128); // é, one-byte non-ASCII
  let cases: &[(&str, String)] = &[
    ("short_ascii", "hello world".to_string()),
    ("ascii_32", "a".repeat(32)),
    ("ascii_64", "a".repeat(64)),
    ("ascii_128", "a".repeat(128)),
    ("ascii_256", "a".repeat(256)),
    ("latin1", "café \u{00a0}\u{00ff}".to_string()),
    ("long_latin1", long_latin1),
    ("twobyte", "Hello 🦕 世界!".to_string()),
  ];

  // Correctness gate: every case must round-trip exactly.
  for (name, reference) in cases {
    let local = v8::String::new(scope, reference).unwrap();
    let got = local.to_rust_string_lossy(scope);
    assert_eq!(&got, reference, "round-trip mismatch for {name}");
  }
  println!("correctness: all cases round-trip OK");

  let runs = 2_000_000u64;
  for (name, reference) in cases {
    let local = v8::String::new(scope, reference).unwrap();
    // warmup
    for _ in 0..100_000 {
      std::hint::black_box(local.to_rust_string_lossy(scope));
    }
    let start = Instant::now();
    for _ in 0..runs {
      std::hint::black_box(local.to_rust_string_lossy(scope));
    }
    let ns = start.elapsed().as_nanos() as f64 / runs as f64;
    println!("  {ns:7.2} ns/op  {name}");
  }
}
