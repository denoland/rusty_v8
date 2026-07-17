// Rust -> V8 string creation benchmark: new_from_utf8 across sizes and content.
use std::time::Instant;

fn make(kind: &str, n: usize) -> String {
  match kind {
    "ascii" => "a".repeat(n),
    "latin1" => "\u{00e9}".repeat(n),
    "twobyte" => "\u{4e16}".repeat(n),
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

  // Correctness: created string must round-trip.
  for kind in kinds {
    for n in sizes {
      let reference = make(kind, n);
      let local = v8::String::new(scope, &reference).unwrap();
      assert_eq!(local.to_rust_string_lossy(scope), reference, "{kind}/{n}");
    }
  }
  println!("correctness OK");

  let runs = 1_000_000u64;
  for kind in kinds {
    for n in sizes {
      let s = make(kind, n);
      let bytes = s.as_bytes();
      let iters = if n >= 4096 { runs / 10 } else { runs };
      for _ in 0..(iters / 10) {
        v8::scope!(let hs, scope);
        std::hint::black_box(
          v8::String::new_from_utf8(hs, bytes, v8::NewStringType::Normal)
            .unwrap(),
        );
      }
      let t = Instant::now();
      for _ in 0..iters {
        v8::scope!(let hs, scope);
        std::hint::black_box(
          v8::String::new_from_utf8(hs, bytes, v8::NewStringType::Normal)
            .unwrap(),
        );
      }
      let ns = t.elapsed().as_nanos() as f64 / iters as f64;
      println!("  new     {kind:8} {n:5}  {ns:9.2} ns/op");
    }
  }
}
