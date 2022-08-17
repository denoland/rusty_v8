fn main() {
  // Initialize V8.
  let platform = v8::new_default_platform(0, false).make_shared();
  v8::V8::initialize_platform(platform);
  v8::V8::initialize();

  // Create a new Isolate and make it the current one.
  let isolate = &mut v8::Isolate::new(v8::CreateParams::default());

  // Create a stack-allocated handle scope.
  let handle_scope = &mut v8::HandleScope::new(isolate);

  // Create a new context.
  let context = v8::Context::new(handle_scope);

  // Enter the context for compiling and running the hello world script.
  let scope = &mut v8::ContextScope::new(handle_scope, context);

  let global = context.global(scope);

  extern "C" fn callback(info: *const v8::FunctionCallbackInfo) {
    let scope = unsafe { &mut v8::CallbackScope::new(&*info) };
    // let args = unsafe { v8::FunctionCallbackArguments::from_function_callback_info(info) };
    let mut rv = unsafe { v8::ReturnValue::from_function_callback_info(info) };
    rv.set(v8::Integer::new(scope, 42).into());
  }
  let func = v8::Function::new_raw(scope, callback).unwrap();

  let name = v8::String::new(scope, "f").unwrap();
  global.set(scope, name.into(), func.into()).unwrap();

  let runs = 100_000_000;
  let code = format!(
    "
    const runs = {};
    const start = Date.now();
    for (let i = 0; i < runs; i++) f();
    Date.now() - start;
  ",
    runs
  );

  let source = v8::String::new(scope, &code).unwrap();
  let script = v8::Script::compile(scope, source, None).unwrap();

  //let start = std::time::Instant::now();
  let r = script.run(scope).unwrap();
  //let elapsed = start.elapsed().as_nanos();
  let number = r.to_number(scope).unwrap();
  let total_ms = number.number_value(scope).unwrap();
  let total_ns = 1e6 * total_ms;
  //println!("elapsed {} ns", total_ns);
  //println!("elapsed {} ns", elapsed);

  let ns_per_run = total_ns / (runs as f64);
  println!("{} ns", ns_per_run);
}
