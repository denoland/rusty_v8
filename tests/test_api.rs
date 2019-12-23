// Copyright 2018-2019 the Deno authors. All rights reserved. MIT license.

#[macro_use]
extern crate lazy_static;

use rusty_v8 as v8;
use rusty_v8::{new_null, FunctionCallbackInfo, HandleScope, Local};
use std::sync::Mutex;

lazy_static! {
  static ref INIT_LOCK: Mutex<u32> = Mutex::new(0);
}

struct TestGuard {}

impl Drop for TestGuard {
  fn drop(&mut self) {
    // TODO shutdown process cleanly.
    /*
    *g -= 1;
    if *g  == 0 {
      unsafe { v8::V8::dispose() };
      v8::V8::shutdown_platform();
    }
    drop(g);
    */
  }
}

fn setup() -> TestGuard {
  let mut g = INIT_LOCK.lock().unwrap();
  *g += 1;
  if *g == 1 {
    v8::V8::initialize_platform(v8::platform::new_default_platform());
    v8::V8::initialize();
  }
  drop(g);
  TestGuard {}
}

#[test]
fn handle_scope_nested() {
  let g = setup();
  let mut params = v8::Isolate::create_params();
  params.set_array_buffer_allocator(v8::Allocator::new_default_allocator());
  let isolate = v8::Isolate::new(params);
  let mut locker = v8::Locker::new(&isolate);
  v8::HandleScope::enter(&mut locker, |scope1| {
    v8::HandleScope::enter(scope1, |_scope2| {});
  });
  drop(locker);
  drop(g);
}

#[test]
#[allow(clippy::float_cmp)]
fn handle_scope_numbers() {
  let g = setup();
  let mut params = v8::Isolate::create_params();
  params.set_array_buffer_allocator(v8::Allocator::new_default_allocator());
  let isolate = v8::Isolate::new(params);
  let mut locker = v8::Locker::new(&isolate);
  v8::HandleScope::enter(&mut locker, |scope1| {
    let l1 = v8::Integer::new(scope1, -123);
    let l2 = v8::Integer::new_from_unsigned(scope1, 456);
    v8::HandleScope::enter(scope1, |scope2| {
      let l3 = v8::Number::new(scope2, 78.9);
      assert_eq!(l1.value(), -123);
      assert_eq!(l2.value(), 456);
      assert_eq!(l3.value(), 78.9);
      assert_eq!(v8::Number::value(&l1), -123f64);
      assert_eq!(v8::Number::value(&l2), 456f64);
    });
  });
  drop(locker);
  drop(g);
}

#[test]
fn global_handles() {
  let _g = setup();
  let mut params = v8::Isolate::create_params();
  params.set_array_buffer_allocator(v8::Allocator::new_default_allocator());
  let isolate = v8::Isolate::new(params);
  let mut locker = v8::Locker::new(&isolate);
  let mut g1 = v8::Global::<v8::String>::new();
  let mut g2 = v8::Global::<v8::Integer>::new();
  let mut g3 = v8::Global::<v8::Integer>::new();
  let mut g4 = v8::Global::<v8::Integer>::new();
  let g5 = v8::Global::<v8::Script>::new();
  v8::HandleScope::enter(&mut locker, |scope| {
    let l1 = v8::String::new(scope, "bla").unwrap();
    let l2 = v8::Integer::new(scope, 123);
    g1.set(scope, l1);
    g2.set(scope, l2);
    g3.set(scope, &g2);
    g4 = v8::Global::new_from(scope, l2);
  });
  v8::HandleScope::enter(&mut locker, |scope| {
    assert!(!g1.is_empty());
    assert_eq!(g1.get(scope).unwrap().to_rust_string_lossy(scope), "bla");
    assert!(!g2.is_empty());
    assert_eq!(g2.get(scope).unwrap().value(), 123);
    assert!(!g3.is_empty());
    assert_eq!(g3.get(scope).unwrap().value(), 123);
    assert!(!g4.is_empty());
    assert_eq!(g4.get(scope).unwrap().value(), 123);
    assert!(g5.is_empty());
  });
  g1.reset(&mut locker);
  assert!(g1.is_empty());
  g2.reset(&mut locker);
  assert!(g2.is_empty());
  g3.reset(&mut locker);
  assert!(g3.is_empty());
  g4.reset(&mut locker);
  assert!(g4.is_empty());
  assert!(g5.is_empty());
}

#[test]
fn test_string() {
  setup();
  let mut params = v8::Isolate::create_params();
  params.set_array_buffer_allocator(v8::Allocator::new_default_allocator());
  let isolate = v8::Isolate::new(params);
  let mut locker = v8::Locker::new(&isolate);
  v8::HandleScope::enter(&mut locker, |scope| {
    let reference = "Hello 🦕 world!";
    let local = v8::String::new(scope, reference).unwrap();
    assert_eq!(15, local.length());
    assert_eq!(17, local.utf8_length(scope));
    assert_eq!(reference, local.to_rust_string_lossy(scope));
  });
  drop(locker);
}

#[test]
#[allow(clippy::float_cmp)]
fn escapable_handle_scope() {
  let g = setup();
  let mut params = v8::Isolate::create_params();
  params.set_array_buffer_allocator(v8::Allocator::new_default_allocator());
  let mut isolate = v8::Isolate::new(params);
  let mut locker = v8::Locker::new(&isolate);
  isolate.enter();
  v8::HandleScope::enter(&mut locker, |scope1| {
    // After dropping EscapableHandleScope, we should be able to
    // read escaped values.
    let number_val = {
      let mut escapable_scope = v8::EscapableHandleScope::new(scope1);
      let number: Local<v8::Value> =
        cast(v8::Number::new(&mut escapable_scope, 78.9));
      escapable_scope.escape(number)
    };
    let number: Local<v8::Number> = cast(number_val);
    assert_eq!(number.value(), 78.9);

    let str_val = {
      let mut escapable_scope = v8::EscapableHandleScope::new(scope1);
      let string =
        v8::String::new(&mut escapable_scope, "Hello 🦕 world!").unwrap();
      escapable_scope.escape(cast(string))
    };
    let string: Local<v8::String> = cast(str_val);
    assert_eq!("Hello 🦕 world!", string.to_rust_string_lossy(scope1));

    let str_val = {
      let mut escapable_scope = v8::EscapableHandleScope::new(scope1);
      let nested_str_val = {
        let mut nested_escapable_scope =
          v8::EscapableHandleScope::new(&mut escapable_scope);
        let string =
          v8::String::new(&mut nested_escapable_scope, "Hello 🦕 world!")
            .unwrap();
        nested_escapable_scope.escape(cast(string))
      };
      escapable_scope.escape(nested_str_val)
    };
    let string: Local<v8::String> = cast(str_val);
    assert_eq!("Hello 🦕 world!", string.to_rust_string_lossy(scope1));
  });
  drop(locker);
  isolate.exit();
  drop(g);
}

#[test]
fn array_buffer() {
  setup();
  let mut params = v8::Isolate::create_params();
  params.set_array_buffer_allocator(v8::Allocator::new_default_allocator());
  let isolate = v8::Isolate::new(params);
  let mut locker = v8::Locker::new(&isolate);
  v8::HandleScope::enter(&mut locker, |scope| {
    let mut context = v8::Context::new(scope);
    context.enter();

    let ab = v8::ArrayBuffer::new(scope, 42);
    assert_eq!(42, ab.byte_length());

    let bs = v8::ArrayBuffer::new_backing_store(scope, 84);
    assert_eq!(84, bs.byte_length());
    assert_eq!(false, bs.is_shared());

    context.exit();
  });
  drop(locker);
}

fn v8_str<'sc>(
  scope: &mut HandleScope<'sc>,
  s: &str,
) -> v8::Local<'sc, v8::String> {
  v8::String::new(scope, s).unwrap()
}

#[test]
fn try_catch() {
  fn eval<'sc>(
    scope: &mut HandleScope<'sc>,
    context: Local<v8::Context>,
    code: &'static str,
  ) -> Option<Local<'sc, v8::Value>> {
    let source = v8_str(scope, code);
    let mut script =
      v8::Script::compile(&mut *scope, context, source, None).unwrap();
    script.run(scope, context)
  };

  let _g = setup();
  let mut params = v8::Isolate::create_params();
  params.set_array_buffer_allocator(v8::Allocator::new_default_allocator());
  let isolate = v8::Isolate::new(params);
  let mut locker = v8::Locker::new(&isolate);
  v8::HandleScope::enter(&mut locker, |scope| {
    let mut context = v8::Context::new(scope);
    context.enter();
    {
      // Error thrown - should be caught.
      let mut try_catch = v8::TryCatch::new(scope);
      let tc = try_catch.enter();
      let result = eval(scope, context, "throw new Error('foo')");
      assert!(result.is_none());
      assert!(tc.has_caught());
      assert!(tc.exception().is_some());
      assert!(tc.stack_trace(scope, context).is_some());
      assert!(tc.message().is_some());
      assert_eq!(
        tc.message().unwrap().get(scope).to_rust_string_lossy(scope),
        "Uncaught Error: foo"
      );
    };
    {
      // No error thrown.
      let mut try_catch = v8::TryCatch::new(scope);
      let tc = try_catch.enter();
      let result = eval(scope, context, "1 + 1");
      assert!(result.is_some());
      assert!(!tc.has_caught());
      assert!(tc.exception().is_none());
      assert!(tc.stack_trace(scope, context).is_none());
      assert!(tc.message().is_none());
      assert!(tc.rethrow().is_none());
    };
    {
      // Rethrow and reset.
      let mut try_catch_1 = v8::TryCatch::new(scope);
      let tc1 = try_catch_1.enter();
      {
        let mut try_catch_2 = v8::TryCatch::new(scope);
        let tc2 = try_catch_2.enter();
        eval(scope, context, "throw 'bar'");
        assert!(tc2.has_caught());
        assert!(tc2.rethrow().is_some());
        tc2.reset();
        assert!(!tc2.has_caught());
      }
      assert!(tc1.has_caught());
    };
    context.exit();
  });
}

#[test]
fn isolate_add_message_listener() {
  let g = setup();
  let mut params = v8::Isolate::create_params();
  params.set_array_buffer_allocator(v8::Allocator::new_default_allocator());
  let mut isolate = v8::Isolate::new(params);
  isolate.set_capture_stack_trace_for_uncaught_exceptions(true, 32);

  use std::sync::atomic::{AtomicUsize, Ordering};
  static CALL_COUNT: AtomicUsize = AtomicUsize::new(0);

  extern "C" fn check_message_0(
    message: Local<v8::Message>,
    _exception: Local<v8::Value>,
  ) {
    CALL_COUNT.fetch_add(1, Ordering::SeqCst);
    let isolate = unsafe { message.get_isolate() };
    v8::HandleScope::enter(isolate, |scope| {
      let message_str = message.get(scope);
      assert_eq!(message_str.to_rust_string_lossy(scope), "Uncaught foo");
    });
  }
  isolate.add_message_listener(check_message_0);

  let mut locker = v8::Locker::new(&isolate);
  v8::HandleScope::enter(&mut locker, |s| {
    let mut context = v8::Context::new(s);
    context.enter();
    let source = v8::String::new(s, "throw 'foo'").unwrap();
    let mut script = v8::Script::compile(s, context, source, None).unwrap();
    assert!(script.run(s, context).is_none());
    assert_eq!(CALL_COUNT.load(Ordering::SeqCst), 1);
    context.exit();
  });
  drop(locker);
  drop(g);
}

#[test]
fn script_compile_and_run() {
  setup();
  let mut params = v8::Isolate::create_params();
  params.set_array_buffer_allocator(v8::Allocator::new_default_allocator());
  let isolate = v8::Isolate::new(params);
  let mut locker = v8::Locker::new(&isolate);

  v8::HandleScope::enter(&mut locker, |s| {
    let mut context = v8::Context::new(s);
    context.enter();
    let source = v8::String::new(s, "'Hello ' + 13 + 'th planet'").unwrap();
    let mut script = v8::Script::compile(s, context, source, None).unwrap();
    source.to_rust_string_lossy(s);
    let result = script.run(s, context).unwrap();
    // TODO: safer casts.
    let result: v8::Local<v8::String> =
      unsafe { std::mem::transmute_copy(&result) };
    assert_eq!(result.to_rust_string_lossy(s), "Hello 13th planet");
    context.exit();
  });
  drop(locker);
}

#[test]
fn script_origin() {
  setup();
  let mut params = v8::Isolate::create_params();
  params.set_array_buffer_allocator(v8::Allocator::new_default_allocator());
  let isolate = v8::Isolate::new(params);
  let mut locker = v8::Locker::new(&isolate);

  v8::HandleScope::enter(&mut locker, |s| {
    let mut context = v8::Context::new(s);
    context.enter();

    let resource_name = v8::String::new(s, "foo.js").unwrap();
    let resource_line_offset = v8::Integer::new(s, 4);
    let resource_column_offset = v8::Integer::new(s, 5);
    let resource_is_shared_cross_origin = v8::new_true(s);
    let script_id = v8::Integer::new(s, 123);
    let source_map_url = v8::String::new(s, "source_map_url").unwrap();
    let resource_is_opaque = v8::new_true(s);
    let is_wasm = v8::new_false(s);
    let is_module = v8::new_false(s);

    let script_origin = v8::ScriptOrigin::new(
      resource_name.into(),
      resource_line_offset,
      resource_column_offset,
      resource_is_shared_cross_origin,
      script_id,
      source_map_url.into(),
      resource_is_opaque,
      is_wasm,
      is_module,
    );

    let source = v8::String::new(s, "1+2").unwrap();
    let mut script =
      v8::Script::compile(s, context, source, Some(&script_origin)).unwrap();
    source.to_rust_string_lossy(s);
    let _result = script.run(s, context).unwrap();
    context.exit();
  });
  drop(locker);
}

#[test]
fn get_version() {
  assert!(v8::V8::get_version().len() > 3);
}

#[test]
fn set_flags_from_command_line() {
  let r = v8::V8::set_flags_from_command_line(vec![
    "binaryname".to_string(),
    "--log-colour".to_string(),
    "--should-be-ignored".to_string(),
  ]);
  assert_eq!(
    r,
    vec!["binaryname".to_string(), "--should-be-ignored".to_string()]
  );
}

#[test]
fn inspector_string_view() {
  let chars = b"Hello world!";
  let view = v8::inspector::StringView::from(&chars[..]);

  assert_eq!(chars.len(), view.into_iter().len());
  assert_eq!(chars.len(), view.len());
  for (c1, c2) in chars.iter().copied().map(u16::from).zip(&view) {
    assert_eq!(c1, c2);
  }
}

#[test]
fn inspector_string_buffer() {
  let chars = b"Hello Venus!";
  let mut buf = {
    let src_view = v8::inspector::StringView::from(&chars[..]);
    v8::inspector::StringBuffer::create(&src_view)
  };
  let view = buf.as_mut().unwrap().string();

  assert_eq!(chars.len(), view.into_iter().len());
  assert_eq!(chars.len(), view.len());
  for (c1, c2) in chars.iter().copied().map(u16::from).zip(view) {
    assert_eq!(c1, c2);
  }
}

#[test]
fn test_primitives() {
  setup();
  let mut params = v8::Isolate::create_params();
  params.set_array_buffer_allocator(v8::Allocator::new_default_allocator());
  let isolate = v8::Isolate::new(params);
  let mut locker = v8::Locker::new(&isolate);
  v8::HandleScope::enter(&mut locker, |scope| {
    let null = v8::new_null(scope);
    assert!(!null.is_undefined());
    assert!(null.is_null());
    assert!(null.is_null_or_undefined());

    let undefined = v8::new_undefined(scope);
    assert!(undefined.is_undefined());
    assert!(!undefined.is_null());
    assert!(undefined.is_null_or_undefined());

    let true_ = v8::new_true(scope);
    assert!(!true_.is_undefined());
    assert!(!true_.is_null());
    assert!(!true_.is_null_or_undefined());

    let false_ = v8::new_false(scope);
    assert!(!false_.is_undefined());
    assert!(!false_.is_null());
    assert!(!false_.is_null_or_undefined());
  });
  drop(locker);
}

#[test]
fn exception() {
  setup();
  let mut params = v8::Isolate::create_params();
  params.set_array_buffer_allocator(v8::Allocator::new_default_allocator());
  let mut isolate = v8::Isolate::new(params);
  let mut locker = v8::Locker::new(&isolate);
  isolate.enter();
  v8::HandleScope::enter(&mut locker, |scope| {
    let mut context = v8::Context::new(scope);
    context.enter();
    let reference = "This is a test error";
    let local = v8::String::new(scope, reference).unwrap();
    v8::range_error(scope, local);
    v8::reference_error(scope, local);
    v8::syntax_error(scope, local);
    v8::type_error(scope, local);
    let exception = v8::error(scope, local);
    let msg = v8::create_message(scope, exception);
    let msg_string = msg.get(scope);
    let rust_msg_string = msg_string.to_rust_string_lossy(scope);
    assert_eq!(
      "Uncaught Error: This is a test error".to_string(),
      rust_msg_string
    );
    assert!(v8::get_stack_trace(scope, exception).is_none());
    context.exit();
  });
  drop(locker);
  isolate.exit();
}

#[test]
fn json() {
  setup();
  let mut params = v8::Isolate::create_params();
  params.set_array_buffer_allocator(v8::Allocator::new_default_allocator());
  let isolate = v8::Isolate::new(params);
  let mut locker = v8::Locker::new(&isolate);
  v8::HandleScope::enter(&mut locker, |s| {
    let mut context = v8::Context::new(s);
    context.enter();
    let json_string = v8_str(s, "{\"a\": 1, \"b\": 2}");
    let maybe_value = v8::json::parse(context, json_string);
    assert!(maybe_value.is_some());
    let value = maybe_value.unwrap();
    let maybe_stringified = v8::json::stringify(context, value);
    assert!(maybe_stringified.is_some());
    let stringified = maybe_stringified.unwrap();
    let rust_str = stringified.to_rust_string_lossy(s);
    assert_eq!("{\"a\":1,\"b\":2}".to_string(), rust_str);
    context.exit();
  });
  drop(locker);
}

// TODO Safer casts https://github.com/denoland/rusty_v8/issues/51
fn cast<U, T>(local: v8::Local<T>) -> v8::Local<U> {
  let cast_local: v8::Local<U> = unsafe { std::mem::transmute_copy(&local) };
  cast_local
}

#[test]
fn object() {
  setup();
  let mut params = v8::Isolate::create_params();
  params.set_array_buffer_allocator(v8::Allocator::new_default_allocator());
  let isolate = v8::Isolate::new(params);
  let mut locker = v8::Locker::new(&isolate);
  v8::HandleScope::enter(&mut locker, |scope| {
    let mut context = v8::Context::new(scope);
    context.enter();
    let null: v8::Local<v8::Value> = new_null(scope).into();
    let s1 = v8::String::new(scope, "a").unwrap();
    let s2 = v8::String::new(scope, "b").unwrap();
    let name1: Local<v8::Name> = cast(s1);
    let name2: Local<v8::Name> = cast(s2);
    let names = vec![name1, name2];
    let v1: v8::Local<v8::Value> = v8::Number::new(scope, 1.0).into();
    let v2: v8::Local<v8::Value> = v8::Number::new(scope, 2.0).into();
    let values = vec![v1, v2];
    let object = v8::Object::new(scope, null, names, values, 2);
    assert!(!object.is_null_or_undefined());
    context.exit();
  });
  drop(locker);
}

#[test]
fn promise_resolved() {
  setup();
  let mut params = v8::Isolate::create_params();
  params.set_array_buffer_allocator(v8::Allocator::new_default_allocator());
  let isolate = v8::Isolate::new(params);
  let mut locker = v8::Locker::new(&isolate);
  v8::HandleScope::enter(&mut locker, |scope| {
    let mut context = v8::Context::new(scope);
    context.enter();
    let maybe_resolver = v8::PromiseResolver::new(scope, context);
    assert!(maybe_resolver.is_some());
    let mut resolver = maybe_resolver.unwrap();
    let mut promise = resolver.get_promise(scope);
    assert!(!promise.has_handler());
    assert_eq!(promise.state(), v8::PromiseState::Pending);
    let str = v8::String::new(scope, "test").unwrap();
    let value: Local<v8::Value> = cast(str);
    resolver.resolve(context, value);
    assert_eq!(promise.state(), v8::PromiseState::Fulfilled);
    let result = promise.result(scope);
    let result_str: v8::Local<v8::String> = cast(result);
    assert_eq!(result_str.to_rust_string_lossy(scope), "test".to_string());
    // Resolve again with different value, since promise is already in `Fulfilled` state
    // it should be ignored.
    let str = v8::String::new(scope, "test2").unwrap();
    let value: Local<v8::Value> = cast(str);
    resolver.resolve(context, value);
    let result = promise.result(scope);
    let result_str: v8::Local<v8::String> = cast(result);
    assert_eq!(result_str.to_rust_string_lossy(scope), "test".to_string());
    context.exit();
  });
  drop(locker);
}

#[test]
fn promise_rejected() {
  setup();
  let mut params = v8::Isolate::create_params();
  params.set_array_buffer_allocator(v8::Allocator::new_default_allocator());
  let isolate = v8::Isolate::new(params);
  let mut locker = v8::Locker::new(&isolate);
  v8::HandleScope::enter(&mut locker, |scope| {
    let mut context = v8::Context::new(scope);
    context.enter();
    let maybe_resolver = v8::PromiseResolver::new(scope, context);
    assert!(maybe_resolver.is_some());
    let mut resolver = maybe_resolver.unwrap();
    let mut promise = resolver.get_promise(scope);
    assert!(!promise.has_handler());
    assert_eq!(promise.state(), v8::PromiseState::Pending);
    let str = v8::String::new(scope, "test").unwrap();
    let value: Local<v8::Value> = cast(str);
    let rejected = resolver.reject(context, value);
    assert!(rejected.unwrap());
    assert_eq!(promise.state(), v8::PromiseState::Rejected);
    let result = promise.result(scope);
    let result_str: v8::Local<v8::String> = cast(result);
    assert_eq!(result_str.to_rust_string_lossy(scope), "test".to_string());
    // Reject again with different value, since promise is already in `Rejected` state
    // it should be ignored.
    let str = v8::String::new(scope, "test2").unwrap();
    let value: Local<v8::Value> = cast(str);
    resolver.reject(context, value);
    let result = promise.result(scope);
    let result_str: v8::Local<v8::String> = cast(result);
    assert_eq!(result_str.to_rust_string_lossy(scope), "test".to_string());
    context.exit();
  });
  drop(locker);
}

extern "C" fn fn_callback(info: &FunctionCallbackInfo) {
  assert_eq!(info.length(), 0);
  let isolate = unsafe { info.get_isolate() };
  v8::HandleScope::enter(isolate, |scope| {
    let s = v8::String::new(scope, "Hello callback!").unwrap();
    let value: Local<v8::Value> = s.into();
    let rv = &mut info.get_return_value();
    let rv_value = rv.get(scope);
    assert!(rv_value.is_undefined());
    rv.set(value);
  });
}

#[test]
fn function() {
  setup();
  let mut params = v8::Isolate::create_params();
  params.set_array_buffer_allocator(v8::Allocator::new_default_allocator());
  let isolate = v8::Isolate::new(params);
  let mut locker = v8::Locker::new(&isolate);
  v8::HandleScope::enter(&mut locker, |scope| {
    let mut context = v8::Context::new(scope);
    context.enter();
    let global = context.global();
    let recv: Local<v8::Value> = global.into();
    // create function using template
    let mut fn_template = v8::FunctionTemplate::new(scope, fn_callback);
    let mut function = fn_template
      .get_function(scope, context)
      .expect("Unable to create function");
    let _value =
      v8::Function::call(&mut *function, scope, context, recv, 0, vec![]);
    // create function without a template
    let mut function = v8::Function::new(scope, context, fn_callback)
      .expect("Unable to create function");
    let maybe_value =
      v8::Function::call(&mut *function, scope, context, recv, 0, vec![]);
    let value = maybe_value.unwrap();
    let value_str: v8::Local<v8::String> = cast(value);
    let rust_str = value_str.to_rust_string_lossy(scope);
    assert_eq!(rust_str, "Hello callback!".to_string());
    context.exit();
  });
  drop(locker);
}

extern "C" fn promise_reject_callback(msg: v8::PromiseRejectMessage) {
  let event = msg.get_event();
  assert_eq!(event, v8::PromiseRejectEvent::PromiseRejectWithNoHandler);
  let mut promise = msg.get_promise();
  assert_eq!(promise.state(), v8::PromiseState::Rejected);
  let promise_obj: v8::Local<v8::Object> = cast(promise);
  let isolate = promise_obj.get_isolate();
  let value = msg.get_value();
  let mut locker = v8::Locker::new(isolate);
  v8::HandleScope::enter(&mut locker, |scope| {
    let value_str: v8::Local<v8::String> = cast(value);
    let rust_str = value_str.to_rust_string_lossy(scope);
    assert_eq!(rust_str, "promise rejected".to_string());
  });
  drop(locker);
}

#[test]
fn set_promise_reject_callback() {
  setup();
  let mut params = v8::Isolate::create_params();
  params.set_array_buffer_allocator(v8::Allocator::new_default_allocator());
  let mut isolate = v8::Isolate::new(params);
  isolate.set_promise_reject_callback(promise_reject_callback);
  isolate.enter();
  let mut locker = v8::Locker::new(&isolate);
  v8::HandleScope::enter(&mut locker, |scope| {
    let mut context = v8::Context::new(scope);
    context.enter();
    let mut resolver = v8::PromiseResolver::new(scope, context).unwrap();
    let str_ = v8::String::new(scope, "promise rejected").unwrap();
    let value: Local<v8::Value> = cast(str_);
    resolver.reject(context, value);
    context.exit();
  });
  drop(locker);
  isolate.exit();
}

fn mock_script_origin<'sc>(
  scope: &mut HandleScope<'sc>,
) -> v8::ScriptOrigin<'sc> {
  let resource_name = v8_str(scope, "foo.js");
  let resource_line_offset = v8::Integer::new(scope, 0);
  let resource_column_offset = v8::Integer::new(scope, 0);
  let resource_is_shared_cross_origin = v8::new_true(scope);
  let script_id = v8::Integer::new(scope, 123);
  let source_map_url = v8_str(scope, "source_map_url");
  let resource_is_opaque = v8::new_true(scope);
  let is_wasm = v8::new_false(scope);
  let is_module = v8::new_true(scope);
  v8::ScriptOrigin::new(
    resource_name.into(),
    resource_line_offset,
    resource_column_offset,
    resource_is_shared_cross_origin,
    script_id,
    source_map_url.into(),
    resource_is_opaque,
    is_wasm,
    is_module,
  )
}

#[test]
fn script_compiler_source() {
  let g = setup();
  let mut params = v8::Isolate::create_params();
  params.set_array_buffer_allocator(v8::Allocator::new_default_allocator());
  let mut isolate = v8::Isolate::new(params);
  isolate.set_promise_reject_callback(promise_reject_callback);
  isolate.enter();
  let mut locker = v8::Locker::new(&isolate);
  v8::HandleScope::enter(&mut locker, |scope| {
    let mut context = v8::Context::new(scope);
    context.enter();

    let source = "1+2";
    let script_origin = mock_script_origin(scope);
    let source =
      v8::script_compiler::Source::new(v8_str(scope, source), &script_origin);

    let result = v8::script_compiler::compile_module(
      &isolate,
      source,
      v8::script_compiler::CompileOptions::NoCompileOptions,
      v8::script_compiler::NoCacheReason::NoReason,
    );
    assert!(result.is_some());

    context.exit();
  });
  drop(locker);
  isolate.exit();
  drop(g);
}

#[test]
fn module_instantiation_failures1() {
  let g = setup();
  let mut params = v8::Isolate::create_params();
  params.set_array_buffer_allocator(v8::Allocator::new_default_allocator());
  let mut isolate = v8::Isolate::new(params);
  isolate.enter();
  let mut locker = v8::Locker::new(&isolate);
  v8::HandleScope::enter(&mut locker, |scope| {
    let mut context = v8::Context::new(scope);
    context.enter();

    let source_text = v8_str(
      scope,
      "import './foo.js';\n\
       export {} from './bar.js';",
    );
    let origin = mock_script_origin(scope);
    let source = v8::script_compiler::Source::new(source_text, &origin);

    let module = v8::script_compiler::compile_module(
      &isolate,
      source,
      v8::script_compiler::CompileOptions::NoCompileOptions,
      v8::script_compiler::NoCacheReason::NoReason,
    )
    .unwrap();
    assert_eq!(v8::ModuleStatus::Uninstantiated, module.get_status());
    assert_eq!(2, module.get_module_requests_length());

    assert_eq!(
      "./foo.js",
      module.get_module_request(0).to_rust_string_lossy(scope)
    );
    let loc = module.get_module_request_location(0);
    assert_eq!(0, loc.get_line_number());
    assert_eq!(7, loc.get_column_number());

    assert_eq!(
      "./bar.js",
      module.get_module_request(1).to_rust_string_lossy(scope)
    );
    let loc = module.get_module_request_location(1);
    assert_eq!(1, loc.get_line_number());
    assert_eq!(15, loc.get_column_number());

    // TODO(ry) Instantiation should fail.

    context.exit();
  });
  drop(locker);
  isolate.exit();
  drop(g);
}

#[test]
fn primitive_array() {
  let g = setup();
  let mut params = v8::Isolate::create_params();
  params.set_array_buffer_allocator(v8::Allocator::new_default_allocator());
  let mut isolate = v8::Isolate::new(params);
  isolate.enter();
  let mut locker = v8::Locker::new(&isolate);
  v8::HandleScope::enter(&mut locker, |scope| {
    let mut context = v8::Context::new(scope);
    context.enter();

    let length = 3;
    let array = v8::PrimitiveArray::new(scope, length);
    assert_eq!(length, array.length());

    for i in 0..length {
      let item = array.get(scope, i);
      assert!(item.is_undefined());
    }

    let string = v8_str(scope, "test");
    array.set(scope, 1, cast(string));
    assert!(array.get(scope, 0).is_undefined());
    assert!(array.get(scope, 1).is_string());

    let num = v8::Number::new(scope, 0.42);
    array.set(scope, 2, cast(num));
    assert!(array.get(scope, 0).is_undefined());
    assert!(array.get(scope, 1).is_string());
    assert!(array.get(scope, 2).is_number());

    context.exit();
  });
  drop(locker);
  isolate.exit();
  drop(g);
}

#[test]
fn ui() {
  // This environment variable tells build.rs that we're running trybuild tests,
  // so it won't rebuild V8.
  std::env::set_var("DENO_TRYBUILD", "1");

  let t = trybuild::TestCases::new();
  t.compile_fail("tests/compile_fail/*.rs");
}

#[test]
fn equality() {
  let g = setup();
  let mut params = v8::Isolate::create_params();
  params.set_array_buffer_allocator(v8::Allocator::new_default_allocator());
  let mut isolate = v8::Isolate::new(params);
  isolate.enter();
  let mut locker = v8::Locker::new(&isolate);
  v8::HandleScope::enter(&mut locker, |scope| {
    let mut context = v8::Context::new(scope);
    context.enter();

    assert!(v8_str(scope, "a").strict_equals(v8_str(scope, "a").into()));
    assert!(!v8_str(scope, "a").strict_equals(v8_str(scope, "b").into()));

    assert!(v8_str(scope, "a").same_value(v8_str(scope, "a").into()));
    assert!(!v8_str(scope, "a").same_value(v8_str(scope, "b").into()));

    context.exit();
  });
  drop(locker);
  isolate.exit();
  drop(g);
}
