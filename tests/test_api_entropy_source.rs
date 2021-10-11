// Tests from the same file run in a single process. That's why this test
// is in its own file, because changing the entropy source affects the
// whole process.
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;

static CALLS: AtomicUsize = AtomicUsize::new(0);

#[test]
fn set_entropy_source() {
  const N: usize = 3;

  v8::V8::set_entropy_source(|buf| {
    CALLS.fetch_add(1, Ordering::SeqCst);
    for c in buf {
      *c = 42;
    }
    true
  });

  v8::V8::initialize_platform(v8::new_default_platform(0, false).make_shared());
  v8::V8::initialize();

  // Assumes that every isolate creates a PRNG from scratch, which is currently true.
  let mut results = vec![];
  for _ in 0..N {
    let isolate = &mut v8::Isolate::new(Default::default());
    let scope = &mut v8::HandleScope::new(isolate);
    let context = v8::Context::new(scope);
    let scope = &mut v8::ContextScope::new(scope, context);
    let source = v8::String::new(scope, "Math.random()").unwrap();
    let script = v8::Script::compile(scope, source, None).unwrap();
    let result = script.run(scope).unwrap();
    let result = result.to_string(scope).unwrap();
    let result = result.to_rust_string_lossy(scope);
    results.push(result);
  }

  unsafe {
    v8::V8::dispose();
  }
  v8::V8::shutdown_platform();

  // All runs should have produced the same value.
  assert_eq!(results.len(), N);
  results.dedup();
  assert_eq!(results.len(), 1);

  // Doesn't have to be exactly N because V8 also calls
  // the EntropySource for things like hash seeds and ASLR.
  assert!(CALLS.load(Ordering::SeqCst) >= N);
}
