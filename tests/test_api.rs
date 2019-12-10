// Copyright 2018-2019 the Deno authors. All rights reserved. MIT license.
#[macro_use]
extern crate lazy_static;

use rusty_v8 as v8;
use rusty_v8::{new_null, Local};
use std::default::Default;
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
  let mut isolate = v8::Isolate::new(params);
  let mut locker = v8::Locker::new(&mut isolate);
  v8::HandleScope::enter(&mut locker, |scope| {
    v8::HandleScope::enter(scope, |_scope| {});
  });
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
  let mut isolate = v8::Isolate::new(params);
  let mut locker = v8::Locker::new(&mut isolate);
  v8::HandleScope::enter(&mut locker, |scope| {
    let l1 = v8::Integer::new(scope, -123);
    let l2 = v8::Integer::new_from_unsigned(scope, 456);
    v8::HandleScope::enter(scope, |scope2| {
      let l3 = v8::Number::new(scope2, 78.9);
      assert_eq!(l1.value(), -123);
      assert_eq!(l2.value(), 456);
      assert_eq!(l3.value(), 78.9);
      assert_eq!(v8::Number::value(&l1), -123f64);
      assert_eq!(v8::Number::value(&l2), 456f64);
    });
  });
  drop(g);
}

#[test]
fn test_string() {
  setup();
  let mut params = v8::Isolate::create_params();
  params.set_array_buffer_allocator(
    v8::array_buffer::Allocator::new_default_allocator(),
  );
  let mut isolate = v8::Isolate::new(params);
  let mut locker = v8::Locker::new(&mut isolate);
  v8::HandleScope::enter(&mut locker, |scope| {
    let reference = "Hello 🦕 world!";
    let local =
      v8::String::new(scope, reference, v8::NewStringType::Normal).unwrap();
    assert_eq!(15, local.length());
    assert_eq!(17, local.utf8_length(scope));
    assert_eq!(reference, local.to_rust_string_lossy(scope));
  });
}

#[test]
fn isolate_new() {
  let g = setup();
  let mut params = v8::Isolate::create_params();
  params.set_array_buffer_allocator(
    v8::array_buffer::Allocator::new_default_allocator(),
  );
  v8::Isolate::new(params);
  drop(g);
}

#[test]
fn script_compile_and_run() {
  setup();
  let mut params = v8::Isolate::create_params();
  params.set_array_buffer_allocator(
    v8::array_buffer::Allocator::new_default_allocator(),
  );
  let mut isolate = v8::Isolate::new(params);
  let mut locker = v8::Locker::new(&mut isolate);

  v8::HandleScope::enter(&mut locker, |s| {
    let mut context = v8::Context::new(s);
    context.enter();
    let source =
      v8::String::new(s, "'Hello ' + 13 + 'th planet'", Default::default())
        .unwrap();
    let mut script = v8::Script::compile(s, context, source, None).unwrap();
    source.to_rust_string_lossy(s);
    let result = script.run(s, context).unwrap();
    // TODO: safer casts.
    let result: v8::Local<v8::String> =
      unsafe { std::mem::transmute_copy(&result) };
    assert_eq!(result.to_rust_string_lossy(s), "Hello 13th planet");
    context.exit();
  });
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
  let mut isolate = v8::Isolate::new(params);
  let mut locker = v8::Locker::new(&mut isolate);
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
}

#[test]
fn exception() {
  setup();
  let mut params = v8::Isolate::create_params();
  params.set_array_buffer_allocator(
    v8::array_buffer::Allocator::new_default_allocator(),
  );
  let mut isolate = v8::Isolate::new(params);
  let mut locker = v8::Locker::new(&mut isolate);
  isolate.enter();
  v8::HandleScope::enter(&mut locker, |scope| {
    let mut context = v8::Context::new(scope);
    context.enter();
    let reference = "This is a test error";
    let local =
      v8::String::new(scope, reference, v8::NewStringType::Normal).unwrap();
    v8::Exception::RangeError(local);
    v8::Exception::ReferenceError(local);
    v8::Exception::SyntaxError(local);
    v8::Exception::TypeError(local);
    let exception = v8::Exception::Error(local);
    let mut msg = v8::Exception::CreateMessage(scope, exception);
    let msg_string = msg.get();
    let rust_msg_string = msg_string.to_rust_string_lossy(scope);
    assert_eq!(
      "Uncaught Error: This is a test error".to_string(),
      rust_msg_string
    );
    assert!(v8::Exception::GetStackTrace(exception).is_none());
    context.exit();
  });
  isolate.exit();
}

#[test]
fn json() {
  setup();
  let mut params = v8::Isolate::create_params();
  params.set_array_buffer_allocator(
    v8::array_buffer::Allocator::new_default_allocator(),
  );
  let mut isolate = v8::Isolate::new(params);
  let mut locker = v8::Locker::new(&mut isolate);
  v8::HandleScope::enter(&mut locker, |s| {
    let mut context = v8::Context::new(s);
    context.enter();
    let json_string =
      v8::String::new(s, "{\"a\": 1, \"b\": 2}", Default::default()).unwrap();
    let maybe_value = v8::JSON::Parse(context, json_string);
    assert!(maybe_value.is_some());
    let value = maybe_value.unwrap();
    let maybe_stringified = v8::JSON::Stringify(context, value);
    assert!(maybe_stringified.is_some());
    let stringified = maybe_stringified.unwrap();
    let rust_str = stringified.to_rust_string_lossy(s);
    assert_eq!("{\"a\":1,\"b\":2}".to_string(), rust_str);
    context.exit();
  });
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
  let mut isolate = v8::Isolate::new(params);
  let mut locker = v8::Locker::new(&mut isolate);
  v8::HandleScope::enter(&mut locker, |scope| {
    let mut context = v8::Context::new(scope);
    context.enter();
    let null: v8::Local<v8::Value> = cast(new_null(scope));
    let s1 = v8::String::new(scope, "a", v8::NewStringType::Normal).unwrap();
    let s2 = v8::String::new(scope, "b", v8::NewStringType::Normal).unwrap();
    let name1: Local<v8::Name> = cast(s1);
    let name2: Local<v8::Name> = cast(s2);
    let names = vec![name1, name2];
    let v1: v8::Local<v8::Value> = cast(v8::Number::new(scope, 1.0));
    let v2: v8::Local<v8::Value> = cast(v8::Number::new(scope, 2.0));
    let values = vec![v1, v2];
    let object = v8::Object::new(scope, null, names, values, 2);
    assert!(!object.is_null_or_undefined());
    context.exit();
  });
}

#[test]
fn promise() {
  setup();
  let mut params = v8::Isolate::create_params();
  params.set_array_buffer_allocator(
    v8::array_buffer::Allocator::new_default_allocator(),
  );
  let mut isolate = v8::Isolate::new(params);
  let mut locker = v8::Locker::new(&mut isolate);
  v8::HandleScope::enter(&mut locker, |scope| {
    let mut context = v8::Context::new(scope);
    context.enter();
    let maybe_resolver = v8::PromiseResolver::new(context);
    assert!(maybe_resolver.is_some());
    context.exit();
  });
}
