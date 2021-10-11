#[test]
fn atomics_pump_message_loop() {
  v8::V8::set_flags_from_string("--harmony-top-level-await --allow-natives-syntax --harmony-sharedarraybuffer");
  v8::V8::initialize_platform(v8::new_default_platform(0, false).make_shared());
  v8::V8::initialize();
  let isolate = &mut v8::Isolate::new(Default::default());
  let scope = &mut v8::HandleScope::new(isolate);
  let context = v8::Context::new(scope);
  let scope = &mut v8::ContextScope::new(scope, context);
  let source = r#"
    function assertEquals(a, b) {
      if (a === b) return;
      throw a + " does not equal " + b;
    }

    const sab = new SharedArrayBuffer(16);
    const i32a = new Int32Array(sab);

    let resolved = false;
    (function() {
      const result = Atomics.waitAsync(i32a, 0, 0);
      result.value.then(
        (value) => { assertEquals("ok", value); resolved = true; },
        () => { assertUnreachable();
      });
    })();

    const notify_return_value = Atomics.notify(i32a, 0, 1);
    assertEquals(1, notify_return_value);
    assertEquals(0, %AtomicsNumWaitersForTesting(i32a, 0));
    assertEquals(1, %AtomicsNumUnresolvedAsyncPromisesForTesting(i32a, 0));
  "#;
  let source = v8::String::new(scope, source).unwrap();
  let script = v8::Script::compile(scope, source, None).unwrap();
  script.run(scope).unwrap();

  while v8::Platform::pump_message_loop(
    &v8::V8::get_current_platform(),
    scope,
    false,
  ) {
    // do nothing
  }

  let source2 = r#"
    assertEquals(0, %AtomicsNumUnresolvedAsyncPromisesForTesting(i32a, 0));
  "#;
  let source2 = v8::String::new(scope, source2).unwrap();
  let script2 = v8::Script::compile(scope, source2, None).unwrap();
  script2.run(scope).unwrap();
}
