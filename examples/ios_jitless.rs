// Minimal jitless smoke test for the iOS spike: force V8 into jitless mode
// (the mode required on a real iOS device, which denies the JIT entitlement)
// and run a little recursive JS to confirm the Ignition interpreter executes.

fn main() {
  let platform = v8::new_default_platform(0, false).make_shared();
  v8::V8::initialize_platform(platform);
  // The flag a real device build needs: no runtime codegen / no RWX pages.
  v8::V8::set_flags_from_string("--jitless");
  v8::V8::initialize();

  {
    let isolate = &mut v8::Isolate::new(v8::CreateParams::default());
    v8::scope!(let handle_scope, isolate);
    let context = v8::Context::new(handle_scope, Default::default());
    let scope = &v8::ContextScope::new(handle_scope, context);

    let src = "const fib=(n)=>n<2?n:fib(n-1)+fib(n-2); `fib(25)=${fib(25)}`";
    let code = v8::String::new(scope, src).unwrap();
    let script = v8::Script::compile(scope, code, None).unwrap();
    let result = script.run(scope).unwrap();
    let result = result.to_string(scope).unwrap();
    println!("JITLESS OK: {}", result.to_rust_string_lossy(scope));
  }

  unsafe {
    v8::V8::dispose();
  }
  v8::V8::dispose_platform();
}
