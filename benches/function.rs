fn main() {
  // Skip running benchmarks in debug or CI.
  if cfg!(debug_assertions) || std::env::var("CI").is_ok() {
    return;
  }
  v8::V8::set_flags_from_string(
    "--turbo_fast_api_calls --allow_natives_syntax",
  );
  let platform = v8::new_default_platform(0, false).make_shared();
  v8::V8::initialize_platform(platform);
  v8::V8::initialize();
  let isolate = &mut v8::Isolate::new(v8::CreateParams::default());
  let handle_scope = &mut v8::HandleScope::new(isolate);
  let context = v8::Context::new(handle_scope);
  let scope = &mut v8::ContextScope::new(handle_scope, context);
  let global = context.global(scope);
  {
    let func = v8::Function::new(
      scope,
      |scope: &mut v8::HandleScope,
       _: v8::FunctionCallbackArguments,
       mut rv: v8::ReturnValue| {
        rv.set(v8::Integer::new(scope, 42).into());
      },
    )
    .unwrap();
    let name = v8::String::new(scope, "new_").unwrap();
    global.set(scope, name.into(), func.into()).unwrap();
  }
  {
    extern "C" fn callback(info: *const v8::FunctionCallbackInfo) {
      let info = unsafe { &*info };
      let scope = unsafe { &mut v8::CallbackScope::new(info) };
      let mut rv = v8::ReturnValue::from_function_callback_info(info);
      rv.set(v8::Integer::new(scope, 42).into());
    }
    let func = v8::Function::new_raw(scope, callback).unwrap();
    let name = v8::String::new(scope, "new_raw").unwrap();
    global.set(scope, name.into(), func.into()).unwrap();
  }
  {
    extern "C" fn callback(info: *const v8::FunctionCallbackInfo) {
      let info = unsafe { &*info };
      let mut rv = v8::ReturnValue::from_function_callback_info(info);
      rv.set_uint32(42);
    }
    let func = v8::Function::new_raw(scope, callback).unwrap();
    let name = v8::String::new(scope, "new_raw_set_uint32").unwrap();
    global.set(scope, name.into(), func.into()).unwrap();
  }
  {
    let func = v8::Function::new(
      scope,
      |_: &mut v8::HandleScope,
       _: v8::FunctionCallbackArguments,
       mut rv: v8::ReturnValue| {
        rv.set_uint32(42);
      },
    )
    .unwrap();
    let name = v8::String::new(scope, "new_set_uint32").unwrap();
    global.set(scope, name.into(), func.into()).unwrap();
  }
  {
    fn fast_fn() -> i32 {
      42
    }
    const FAST_CALL: v8::fast_api::FastFunction =
      v8::fast_api::FastFunction::new(
        &[v8::fast_api::Type::V8Value],
        v8::fast_api::CType::Int32,
        fast_fn as _,
      );
    let template = v8::FunctionTemplate::builder(
      |scope: &mut v8::HandleScope,
       _: v8::FunctionCallbackArguments,
       mut rv: v8::ReturnValue| {
        rv.set(v8::Integer::new(scope, 42).into());
      },
    )
    .build_fast(scope, &FAST_CALL, None, None, None);
    let name = v8::String::new(scope, "new_fast").unwrap();
    let value = template.get_function(scope).unwrap();

    global.set(scope, name.into(), value.into()).unwrap();
  }

  {
    extern "C" fn callback(info: *const v8::FunctionCallbackInfo) {
      let info = unsafe { &*info };
      let scope = unsafe { &mut v8::CallbackScope::new(info) };
      let mut rv = v8::ReturnValue::from_function_callback_info(info);
      rv.set(v8::undefined(scope).into());
    }
    let func = v8::Function::new_raw(scope, callback).unwrap();
    let name = v8::String::new(scope, "undefined_from_scope").unwrap();
    global.set(scope, name.into(), func.into()).unwrap();
  }

  {
    extern "C" fn callback(info: *const v8::FunctionCallbackInfo) {
      let info = unsafe { &*info };
      let mut rv = v8::ReturnValue::from_function_callback_info(info);
      let mut args =
        v8::FunctionCallbackArguments::from_function_callback_info(info);
      rv.set(v8::undefined(unsafe { args.get_isolate() }).into());
    }
    let func = v8::Function::new_raw(scope, callback).unwrap();
    let name = v8::String::new(scope, "undefined_from_isolate").unwrap();
    global.set(scope, name.into(), func.into()).unwrap();
  }

  let runs = 100_000_000;

  for (group_name, benches) in [
    (
      "function_overhead",
      &[
        "new_",
        "new_raw",
        "new_set_uint32",
        "new_raw_set_uint32",
        "new_fast",
      ][..],
    ),
    (
      "primitives",
      &["undefined_from_scope", "undefined_from_isolate"][..],
    ),
  ] {
    println!("Running {} ...", group_name);
    for x in benches {
      let code = format!(
        "
            function bench() {{ return {}(); }};
            runs = {};
            start = Date.now();
            for (i = 0; i < runs; i++) bench();
            Date.now() - start;
          ",
        x, runs
      );

      let r = eval(scope, &code).unwrap();
      let number = r.to_number(scope).unwrap();
      let total_ms = number.number_value(scope).unwrap();
      let total_ns = 1e6 * total_ms;
      let ns_per_run = total_ns / (runs as f64);
      let mops_per_sec = (runs as f64) / (total_ms / 1000.0) / 1e6;
      println!(
        "  {:.1} ns per run {:.1} million ops/sec → {}",
        ns_per_run, mops_per_sec, x
      );
    }
  }
}

fn eval<'s>(
  scope: &mut v8::HandleScope<'s>,
  code: &str,
) -> Option<v8::Local<'s, v8::Value>> {
  let scope = &mut v8::EscapableHandleScope::new(scope);
  let source = v8::String::new(scope, code).unwrap();
  let script = v8::Script::compile(scope, source, None).unwrap();
  let r = script.run(scope);
  r.map(|v| scope.escape(v))
}
