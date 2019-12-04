// Copyright 2018-2019 the Deno authors. All rights reserved. MIT license.
#[macro_use]
extern crate lazy_static;

use rusty_v8 as v8;
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
