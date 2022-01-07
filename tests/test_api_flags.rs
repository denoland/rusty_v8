// Tests from the same file run in a single process. That's why this test
// is in its own file, because changing flags affects the whole process.

#[test]
fn set_flags_from_string() {
  v8::V8::set_flags_from_string("--use_strict");
  v8::V8::initialize_platform(v8::new_default_platform(0, false).make_shared());
  v8::V8::initialize();
  let isolate = &mut v8::Isolate::new(Default::default());
  let scope = &mut v8::HandleScope::new(isolate);
  let context = v8::Context::new(scope);
  let scope = &mut v8::ContextScope::new(scope, context);
  let source = "(function() { return this })()";
  let source = v8::String::new(scope, source).unwrap();
  let script = v8::Script::compile(scope, source, None).unwrap();
  let result = script.run(scope).unwrap();
  assert!(result.is_undefined()); // Because of --use_strict.
}
