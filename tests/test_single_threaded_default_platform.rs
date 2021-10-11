#[test]
fn single_threaded_default_platform() {
  v8::V8::set_flags_from_string("--single_threaded");
  v8::V8::initialize_platform(
    v8::new_single_threaded_default_platform(false).make_shared(),
  );
  v8::V8::initialize();

  {
    let isolate = &mut v8::Isolate::new(Default::default());
    let scope = &mut v8::HandleScope::new(isolate);
    let context = v8::Context::new(scope);
    let scope = &mut v8::ContextScope::new(scope, context);
    let source = v8::String::new(scope, "Math.random()").unwrap();
    let script = v8::Script::compile(scope, source, None).unwrap();
    let result = script.run(scope).unwrap();
    let _ = result.to_string(scope).unwrap();
  }

  unsafe { v8::V8::dispose() };
  v8::V8::shutdown_platform();
}
