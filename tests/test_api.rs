// Copyright 2018-2019 the Deno authors. All rights reserved. MIT license.

#[macro_use]
extern crate lazy_static;

use rusty_v8 as v8;
use rusty_v8::{new_null, FunctionCallbackInfo, Local};
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
  params.set_array_buffer_allocator(
    v8::array_buffer::Allocator::new_default_allocator(),
  );
  let isolate = v8::Isolate::new(params);
  let locker = v8::Locker::new(&isolate);
  v8::HandleScope::enter(&isolate, |_scope1| {
    v8::HandleScope::enter(&isolate, |_scope2| {});
  });
  drop(locker);
  drop(g);
}

#[test]
#[allow(clippy::float_cmp)]
fn handle_scope_numbers() {
  let g = setup();
  let mut params = v8::Isolate::create_params();
  params.set_array_buffer_allocator(
    v8::array_buffer::Allocator::new_default_allocator(),
  );
  let isolate = v8::Isolate::new(params);
  let locker = v8::Locker::new(&isolate);
  v8::HandleScope::enter(&isolate, |_scope| {
    let l1 = v8::Integer::new(&isolate, -123);
    let l2 = v8::Integer::new_from_unsigned(&isolate, 456);
    v8::HandleScope::enter(&isolate, |_scope2| {
      let l3 = v8::Number::new(&isolate, 78.9);
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
fn test_string() {
  setup();
  let mut params = v8::Isolate::create_params();
  params.set_array_buffer_allocator(
    v8::array_buffer::Allocator::new_default_allocator(),
  );
  let isolate = v8::Isolate::new(params);
  let locker = v8::Locker::new(&isolate);
  v8::HandleScope::enter(&isolate, |_scope| {
    let reference = "Hello 🦕 world!";
    let local = v8_str(&isolate, reference);
    assert_eq!(15, local.length());
    assert_eq!(17, local.utf8_length(&isolate));
    assert_eq!(reference, local.to_rust_string_lossy(&isolate));
  });
  drop(locker);
}

fn v8_str(isolate: &v8::Isolate, s: &str) -> v8::Local<v8::String> {
  v8::String::new(&isolate, s, v8::NewStringType::Normal).unwrap()
}

#[test]
fn isolate_add_message_listener() {
  let g = setup();
  let mut params = v8::Isolate::create_params();
  params.set_array_buffer_allocator(
    v8::array_buffer::Allocator::new_default_allocator(),
  );
  let mut isolate = v8::Isolate::new(params);
  isolate.set_capture_stack_trace_for_uncaught_exceptions(true, 32);

  use std::sync::atomic::{AtomicUsize, Ordering};
  static CALL_COUNT: AtomicUsize = AtomicUsize::new(0);

  extern "C" fn check_message_0(
    message: Local<v8::Message>,
    _exception: Local<v8::Value>,
  ) {
    CALL_COUNT.fetch_add(1, Ordering::SeqCst);
    let isolate = message.get_isolate();
    let message_str = message.get();
    assert_eq!(message_str.to_rust_string_lossy(&isolate), "Uncaught foo");
  }
  isolate.add_message_listener(check_message_0);

  let locker = v8::Locker::new(&isolate);
  v8::HandleScope::enter(&isolate, |_s| {
    let mut context = v8::Context::new(&isolate);
    context.enter();
    let source = v8_str(&isolate, "throw 'foo'");
    let mut script = v8::Script::compile(context, source, None).unwrap();
    assert!(script.run(context).is_none());
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
  params.set_array_buffer_allocator(
    v8::array_buffer::Allocator::new_default_allocator(),
  );
  let isolate = v8::Isolate::new(params);
  let locker = v8::Locker::new(&isolate);

  v8::HandleScope::enter(&isolate, |_s| {
    let mut context = v8::Context::new(&isolate);
    context.enter();
    let source = v8_str(&isolate, "'Hello ' + 13 + 'th planet'");
    let mut script = v8::Script::compile(context, source, None).unwrap();
    source.to_rust_string_lossy(&isolate);
    let result = script.run(context).unwrap();
    // TODO: safer casts.
    let result: v8::Local<v8::String> =
      unsafe { std::mem::transmute_copy(&result) };
    assert_eq!(result.to_rust_string_lossy(&isolate), "Hello 13th planet");
    context.exit();
  });
  drop(locker);
}

#[test]
fn script_origin() {
  setup();
  let mut params = v8::Isolate::create_params();
  params.set_array_buffer_allocator(
    v8::array_buffer::Allocator::new_default_allocator(),
  );
  let isolate = v8::Isolate::new(params);
  let locker = v8::Locker::new(&isolate);

  v8::HandleScope::enter(&isolate, |_s| {
    let mut context = v8::Context::new(&isolate);
    context.enter();

    let resource_name = v8_str(&isolate, "foo.js");
    let resource_line_offset = v8::Integer::new(&isolate, 4);
    let resource_column_offset = v8::Integer::new(&isolate, 5);
    let resource_is_shared_cross_origin = v8::new_true(&isolate);
    let script_id = v8::Integer::new(&isolate, 123);
    let source_map_url = v8_str(&isolate, "source_map_url");
    let resource_is_opaque = v8::new_true(&isolate);
    let is_wasm = v8::new_false(&isolate);
    let is_module = v8::new_false(&isolate);

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

    let source = v8_str(&isolate, "1+2");
    let mut script =
      v8::Script::compile(context, source, Some(&script_origin)).unwrap();
    source.to_rust_string_lossy(&isolate);
    let _result = script.run(context).unwrap();
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
  params.set_array_buffer_allocator(
    v8::array_buffer::Allocator::new_default_allocator(),
  );
  let isolate = v8::Isolate::new(params);
  let locker = v8::Locker::new(&isolate);
  v8::HandleScope::enter(&isolate, |_scope| {
    let null = v8::new_null(&isolate);
    assert!(!null.is_undefined());
    assert!(null.is_null());
    assert!(null.is_null_or_undefined());

    let undefined = v8::new_undefined(&isolate);
    assert!(undefined.is_undefined());
    assert!(!undefined.is_null());
    assert!(undefined.is_null_or_undefined());

    let true_ = v8::new_true(&isolate);
    assert!(!true_.is_undefined());
    assert!(!true_.is_null());
    assert!(!true_.is_null_or_undefined());

    let false_ = v8::new_false(&isolate);
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
  params.set_array_buffer_allocator(
    v8::array_buffer::Allocator::new_default_allocator(),
  );
  let mut isolate = v8::Isolate::new(params);
  let locker = v8::Locker::new(&isolate);
  isolate.enter();
  v8::HandleScope::enter(&isolate, |_scope| {
    let mut context = v8::Context::new(&isolate);
    context.enter();
    let reference = "This is a test error";
    let local = v8_str(&isolate, reference);
    v8::range_error(local);
    v8::reference_error(local);
    v8::syntax_error(local);
    v8::type_error(local);
    let exception = v8::error(local);
    let msg = v8::create_message(&isolate, exception);
    let msg_string = msg.get();
    let rust_msg_string = msg_string.to_rust_string_lossy(&isolate);
    assert_eq!(
      "Uncaught Error: This is a test error".to_string(),
      rust_msg_string
    );
    assert!(v8::get_stack_trace(exception).is_none());
    context.exit();
  });
  drop(locker);
  isolate.exit();
}

#[test]
fn json() {
  setup();
  let mut params = v8::Isolate::create_params();
  params.set_array_buffer_allocator(
    v8::array_buffer::Allocator::new_default_allocator(),
  );
  let isolate = v8::Isolate::new(params);
  let locker = v8::Locker::new(&isolate);
  v8::HandleScope::enter(&isolate, |_s| {
    let mut context = v8::Context::new(&isolate);
    context.enter();
    let json_string = v8_str(&isolate, "{\"a\": 1, \"b\": 2}");
    let maybe_value = v8::JSON::Parse(context, json_string);
    assert!(maybe_value.is_some());
    let value = maybe_value.unwrap();
    let maybe_stringified = v8::JSON::Stringify(context, value);
    assert!(maybe_stringified.is_some());
    let stringified = maybe_stringified.unwrap();
    let rust_str = stringified.to_rust_string_lossy(&isolate);
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
  params.set_array_buffer_allocator(
    v8::array_buffer::Allocator::new_default_allocator(),
  );
  let isolate = v8::Isolate::new(params);
  let locker = v8::Locker::new(&isolate);
  v8::HandleScope::enter(&isolate, |_scope| {
    let mut context = v8::Context::new(&isolate);
    context.enter();
    let null: v8::Local<v8::Value> = new_null(&isolate).into();
    let s1 = v8_str(&isolate, "a");
    let s2 = v8_str(&isolate, "b");
    let name1: Local<v8::Name> = cast(s1);
    let name2: Local<v8::Name> = cast(s2);
    let names = vec![name1, name2];
    let v1: v8::Local<v8::Value> = v8::Number::new(&isolate, 1.0).into();
    let v2: v8::Local<v8::Value> = v8::Number::new(&isolate, 2.0).into();
    let values = vec![v1, v2];
    let object = v8::Object::new(&isolate, null, names, values, 2);
    assert!(!object.is_null_or_undefined());
    context.exit();
  });
  drop(locker);
}

#[test]
fn promise_resolved() {
  setup();
  let mut params = v8::Isolate::create_params();
  params.set_array_buffer_allocator(
    v8::array_buffer::Allocator::new_default_allocator(),
  );
  let isolate = v8::Isolate::new(params);
  let locker = v8::Locker::new(&isolate);
  v8::HandleScope::enter(&isolate, |_scope| {
    let mut context = v8::Context::new(&isolate);
    context.enter();
    let maybe_resolver = v8::PromiseResolver::new(context);
    assert!(maybe_resolver.is_some());
    let mut resolver = maybe_resolver.unwrap();
    let mut promise = resolver.get_promise();
    assert!(!promise.has_handler());
    assert_eq!(promise.state(), v8::PromiseState::Pending);
    let str = v8_str(&isolate, "test");
    let value: Local<v8::Value> = cast(str);
    resolver.resolve(context, value);
    assert_eq!(promise.state(), v8::PromiseState::Fulfilled);
    let result = promise.result();
    let result_str: v8::Local<v8::String> = cast(result);
    assert_eq!(
      result_str.to_rust_string_lossy(&isolate),
      "test".to_string()
    );
    // Resolve again with different value, since promise is already in `Fulfilled` state
    // it should be ignored.
    let str = v8_str(&isolate, "test2");
    let value: Local<v8::Value> = cast(str);
    resolver.resolve(context, value);
    let result = promise.result();
    let result_str: v8::Local<v8::String> = cast(result);
    assert_eq!(
      result_str.to_rust_string_lossy(&isolate),
      "test".to_string()
    );
    context.exit();
  });
  drop(locker);
}

#[test]
fn promise_rejected() {
  setup();
  let mut params = v8::Isolate::create_params();
  params.set_array_buffer_allocator(
    v8::array_buffer::Allocator::new_default_allocator(),
  );
  let isolate = v8::Isolate::new(params);
  let locker = v8::Locker::new(&isolate);
  v8::HandleScope::enter(&isolate, |_scope| {
    let mut context = v8::Context::new(&isolate);
    context.enter();
    let maybe_resolver = v8::PromiseResolver::new(context);
    assert!(maybe_resolver.is_some());
    let mut resolver = maybe_resolver.unwrap();
    let mut promise = resolver.get_promise();
    assert!(!promise.has_handler());
    assert_eq!(promise.state(), v8::PromiseState::Pending);
    let str = v8_str(&isolate, "test");
    let value: Local<v8::Value> = cast(str);
    let rejected = resolver.reject(context, value);
    assert!(rejected.unwrap());
    assert_eq!(promise.state(), v8::PromiseState::Rejected);
    let result = promise.result();
    let result_str: v8::Local<v8::String> = cast(result);
    assert_eq!(
      result_str.to_rust_string_lossy(&isolate),
      "test".to_string()
    );
    // Reject again with different value, since promise is already in `Rejected` state
    // it should be ignored.
    let str = v8_str(&isolate, "test2");
    let value: Local<v8::Value> = cast(str);
    resolver.reject(context, value);
    let result = promise.result();
    let result_str: v8::Local<v8::String> = cast(result);
    assert_eq!(
      result_str.to_rust_string_lossy(&isolate),
      "test".to_string()
    );
    context.exit();
  });
  drop(locker);
}

extern "C" fn fn_callback(info: &FunctionCallbackInfo) {
  assert_eq!(info.length(), 0);
  let isolate = info.get_isolate();
  v8::HandleScope::enter(&isolate, |_scope| {
    let mut context = v8::Context::new(&isolate);
    context.enter();
    let s = v8_str(&isolate, "Hello callback!");
    let value: Local<v8::Value> = s.into();
    let rv = info.get_return_value();
    let rv_value = rv.get();
    assert!(rv_value.is_undefined());
    rv.set(value);
    context.exit();
  });
}

#[test]
fn function() {
  setup();
  let mut params = v8::Isolate::create_params();
  params.set_array_buffer_allocator(
    v8::array_buffer::Allocator::new_default_allocator(),
  );
  let isolate = v8::Isolate::new(params);
  let locker = v8::Locker::new(&isolate);
  v8::HandleScope::enter(&isolate, |_scope| {
    let mut context = v8::Context::new(&isolate);
    context.enter();
    let global = context.global();
    let recv: Local<v8::Value> = global.into();
    // create function using template
    let mut fn_template = v8::FunctionTemplate::new(&isolate, fn_callback);
    let mut function = fn_template
      .get_function(context)
      .expect("Unable to create function");
    let _value = v8::Function::call(&mut *function, context, recv, 0, vec![]);
    // create function without a template
    let mut function = v8::Function::new(context, fn_callback)
      .expect("Unable to create function");
    let maybe_value =
      v8::Function::call(&mut *function, context, recv, 0, vec![]);
    let value = maybe_value.unwrap();
    let value_str: v8::Local<v8::String> = cast(value);
    let rust_str = value_str.to_rust_string_lossy(&isolate);
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
  let locker = v8::Locker::new(isolate);
  v8::HandleScope::enter(&isolate, |_scope| {
    let value_str: v8::Local<v8::String> = cast(value);
    let rust_str = value_str.to_rust_string_lossy(&isolate);
    assert_eq!(rust_str, "promise rejected".to_string());
  });
  drop(locker);
}

#[test]
fn set_promise_reject_callback() {
  setup();
  let mut params = v8::Isolate::create_params();
  params.set_array_buffer_allocator(
    v8::array_buffer::Allocator::new_default_allocator(),
  );
  let mut isolate = v8::Isolate::new(params);
  isolate.set_promise_reject_callback(promise_reject_callback);
  isolate.enter();
  let locker = v8::Locker::new(&isolate);
  v8::HandleScope::enter(&isolate, |_scope| {
    let mut context = v8::Context::new(&isolate);
    context.enter();
    let mut resolver = v8::PromiseResolver::new(context).unwrap();
    let str_ = v8_str(&isolate, "promise rejected");
    let value: Local<v8::Value> = cast(str_);
    resolver.reject(context, value);
    context.exit();
  });
  drop(locker);
  isolate.exit();
}
