fn main() {
  let platform = v8::new_default_platform(0, false).make_shared();
  v8::V8::initialize_platform(platform);
  v8::V8::initialize();
  let isolate = &mut v8::Isolate::new(v8::CreateParams::default());
  let handle_scope = &mut v8::HandleScope::new(isolate);
  let context = v8::Context::new(handle_scope);
  let scope = &mut v8::ContextScope::new(handle_scope, context);
  let global = context.global(scope);

  extern "C" fn callback(info: *const v8::FunctionCallbackInfo) {
    let scope = unsafe { &mut v8::CallbackScope::new(&*info) };
    // let args = unsafe { v8::FunctionCallbackArguments::from_function_callback_info(info) };
    let mut rv = unsafe { v8::ReturnValue::from_function_callback_info(info) };
    rv.set(v8::Integer::new(scope, 42).into());
  }
  let func = v8::Function::new_raw(scope, callback).unwrap();
  let name = v8::String::new(scope, "function_new_raw").unwrap();
  global.set(scope, name.into(), func.into()).unwrap();

  let func = v8::Function::new(
    scope,
    |scope: &mut v8::HandleScope,
     _: v8::FunctionCallbackArguments,
     mut rv: v8::ReturnValue| {
      rv.set(v8::Integer::new(scope, 42).into());
    },
  )
  .unwrap();
  let name = v8::String::new(scope, "function_new").unwrap();
  global.set(scope, name.into(), func.into()).unwrap();

  let runs = 100_000_000;

  for x in [
    "function_new",
    "function_new_raw",
    "function_new",
    "function_new_raw",
  ] {
    let code = format!(
      "
        runs = {};
        start = Date.now();
        for (i = 0; i < runs; i++) {}();
        Date.now() - start;
      ",
      runs, x
    );

    let source = v8::String::new(scope, &code).unwrap();
    let script = v8::Script::compile(scope, source, None).unwrap();
    let r = script.run(scope).unwrap();
    let number = r.to_number(scope).unwrap();
    let total_ms = number.number_value(scope).unwrap();
    let total_ns = 1e6 * total_ms;
    let ns_per_run = total_ns / (runs as f64);
    let mops_per_sec = (runs as f64) / (total_ms / 1000.0) / 1e6;
    println!(
      "{:.1} ns per run {:.1} million ops/sec → {}",
      ns_per_run, mops_per_sec, x
    );
  }
}
