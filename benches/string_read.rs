// V8 -> Rust string read benchmark: to_rust_string_lossy / to_rust_cow_lossy
// across sizes and content kinds. Used to measure the ValueView field-read
// change (proposal 1) and later read-path proposals.
use std::time::Instant;

fn make(kind: &str, n: usize) -> String {
  match kind {
    "ascii" => "a".repeat(n),
    "latin1" => "\u{00e9}".repeat(n), // é, one-byte non-ASCII
    "twobyte" => "\u{4e16}".repeat(n), // 世, two-byte
    _ => unreachable!(),
  }
}

fn main() {
  // Skip running benchmarks in debug or CI (cargo-nextest lists test binaries
  // by running them; this harness=false bench must produce no output there).
  if cfg!(debug_assertions) || std::env::var("CI").is_ok() {
    return;
  }
  let platform = v8::new_default_platform(0, false).make_shared();
  v8::V8::initialize_platform(platform);
  v8::V8::initialize();
  let isolate = &mut v8::Isolate::new(v8::CreateParams::default());
  v8::scope!(let handle_scope, isolate);
  let context = v8::Context::new(handle_scope, Default::default());
  let scope = &mut v8::ContextScope::new(handle_scope, context);

  let sizes = [4usize, 16, 64, 256, 4096];
  let kinds = ["ascii", "latin1", "twobyte"];
  let runs = 2_000_000u64;

  // Correctness gate.
  for kind in kinds {
    for n in sizes {
      let reference = make(kind, n);
      let local = v8::String::new(scope, &reference).unwrap();
      assert_eq!(local.to_rust_string_lossy(scope), reference, "{kind}/{n}");
    }
  }
  println!("correctness OK");

  for kind in kinds {
    for n in sizes {
      let reference = make(kind, n);
      let local = v8::String::new(scope, &reference).unwrap();
      let iters = if n >= 4096 { runs / 20 } else { runs };
      for _ in 0..(iters / 10) {
        std::hint::black_box(local.to_rust_string_lossy(scope));
      }
      let t = Instant::now();
      for _ in 0..iters {
        std::hint::black_box(local.to_rust_string_lossy(scope));
      }
      let ns = t.elapsed().as_nanos() as f64 / iters as f64;
      println!("  lossy   {kind:8} {n:5}  {ns:9.2} ns/op");
    }
  }
}
