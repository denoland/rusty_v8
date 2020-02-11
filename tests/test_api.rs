// Copyright 2019-2020 the Deno authors. All rights reserved. MIT license.

#[macro_use]
extern crate lazy_static;

//use std::convert::{Into, TryFrom, TryInto};
//use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

use rusty_v8 as v8;
// TODO(piscisaureus): Ideally there would be no need to import this trait.
//use v8::MapFnTo;

lazy_static! {
  static ref INIT_LOCK: Mutex<u32> = Mutex::new(0);
}

#[must_use]
struct SetupGuard {}

impl Drop for SetupGuard {
  fn drop(&mut self) {
    // TODO shutdown process cleanly.
  }
}

fn setup() -> SetupGuard {
  let mut g = INIT_LOCK.lock().unwrap();
  *g += 1;
  if *g == 1 {
    v8::V8::initialize_platform(v8::new_default_platform());
    v8::V8::initialize();
  }
  SetupGuard {}
}

#[test]
fn handle_scope_nested() {
  let _setup_guard = setup();
  let mut params = v8::Isolate::create_params();
  params.set_array_buffer_allocator(v8::new_default_allocator());
  let mut isolate = v8::Isolate::new(params);

  v8::HandleScope::new(&mut isolate, |scope1| {
    v8::HandleScope::new(scope1, |scope2| {
      let n = v8::Integer::new(scope2, 42);
      assert_eq!(n.value(), 42);
    });
  });
}

#[test]
#[allow(clippy::float_cmp)]
fn handle_scope_numbers() {
  let _setup_guard = setup();
  let mut params = v8::Isolate::create_params();
  params.set_array_buffer_allocator(v8::new_default_allocator());
  let mut isolate = v8::Isolate::new(params);
  v8::HandleScope::new(&mut isolate, |scope1| {
    let l1 = v8::Integer::new(scope1, -123);
    let l2 = v8::Integer::new_from_unsigned(scope1, 456);
    v8::HandleScope::new(scope1, move |scope2| {
      let l3 = v8::Number::new(scope2, 78.9);
      assert_eq!(l1.value(), -123);
      assert_eq!(l2.value(), 456);
      assert_eq!(l3.value(), 78.9);
      assert_eq!(v8::Number::value(&l1), -123f64);
      assert_eq!(v8::Number::value(&l2), 456f64);
    });
  });
}

#[test]
fn global_handles() {
  let _setup_guard = setup();
  let mut params = v8::Isolate::create_params();
  params.set_array_buffer_allocator(v8::new_default_allocator());
  let mut isolate = v8::Isolate::new(params);
  let mut g1 = v8::Global::<v8::String>::new();
  let mut g2 = v8::Global::<v8::Integer>::new();
  let mut g3 = v8::Global::<v8::Integer>::new();
  let mut _g4 = v8::Global::<v8::Integer>::new();
  let mut g5 = v8::Global::<v8::Script>::new();
  let mut g6 = v8::Global::<v8::Integer>::new();

  v8::HandleScope::new(&mut isolate, |scope| {
    let l1 = v8::String::new(scope, "bla").unwrap();
    let l2 = v8::Integer::new(scope, 123);
    g1.set(scope, l1);
    g2.set(scope, l2);
    g3.set(scope, &g2);
    _g4 = v8::Global::new_from(scope, l2);
    let l6 = v8::Integer::new(scope, 100);
    g6.set(scope, l6);
  });
  v8::HandleScope::new(&mut isolate, |scope| {
    assert!(!g1.is_empty());
    assert_eq!(g1.get(scope).unwrap().to_rust_string_lossy(scope), "bla");
    assert!(!g2.is_empty());
    assert_eq!(g2.get(scope).unwrap().value(), 123);
    assert!(!g3.is_empty());
    assert_eq!(g3.get(scope).unwrap().value(), 123);
    assert!(!_g4.is_empty());
    assert_eq!(_g4.get(scope).unwrap().value(), 123);
    assert!(g5.is_empty());
    let num = g6.get(scope).unwrap();
    g6.reset(scope);
    assert_eq!(num.value(), 100);
  });
  g1.reset(&mut isolate);
  assert!(g1.is_empty());
  g2.reset(&mut isolate);
  assert!(g2.is_empty());
  g3.reset(&mut isolate);
  assert!(g3.is_empty());
  _g4.reset(&mut isolate);
  assert!(_g4.is_empty());
  // TODO(ry) Globals should probably clean up automatically and not cause
  // segfaults.
  g5.reset(&mut isolate);
  assert!(g5.is_empty());
  g6.reset(&mut isolate);
  assert!(g6.is_empty());
}

#[test]
fn test_string() {
  let _setup_guard = setup();
  let mut params = v8::Isolate::create_params();
  params.set_array_buffer_allocator(v8::new_default_allocator());
  let mut isolate = v8::Isolate::new(params);
  v8::HandleScope::new(&mut isolate, |scope| {
    let reference = "Hello 🦕 world!";
    let local = v8::String::new(scope, reference).unwrap();
    assert_eq!(15, local.length());
    assert_eq!(17, local.utf8_length(scope));
    assert_eq!(reference, local.to_rust_string_lossy(scope));
  });
  v8::HandleScope::new(&mut isolate, |scope| {
    let local = v8::String::empty(scope);
    assert_eq!(0, local.length());
    assert_eq!(0, local.utf8_length(scope));
    assert_eq!("", local.to_rust_string_lossy(scope));
  });
  v8::HandleScope::new(&mut isolate, |scope| {
    let local =
      v8::String::new_from_utf8(scope, b"", v8::NewStringType::Normal).unwrap();
    assert_eq!(0, local.length());
    assert_eq!(0, local.utf8_length(scope));
    assert_eq!("", local.to_rust_string_lossy(scope));
  });
}

#[test]
#[allow(clippy::float_cmp)]
fn escapable_handle_scope() {
  let _setup_guard = setup();
  let mut params = v8::Isolate::create_params();
  params.set_array_buffer_allocator(v8::new_default_allocator());
  let mut isolate = v8::Isolate::new(params);
  v8::HandleScope::new(&mut isolate, |scope1| {
    // After dropping EscapableHandleScope, we should be able to
    // read escaped values.
    let number = v8::EscapableHandleScope::new(scope1, |escapable_scope| {
      let number = v8::Number::new(escapable_scope, 78.9);
      number
    });
    assert_eq!(number.value(), 78.9);

    let string = v8::EscapableHandleScope::new(scope1, |escapable_scope| {
      let string = v8::String::new(escapable_scope, "Hello 🦕 world!").unwrap();
      string
    });
    assert_eq!("Hello 🦕 world!", string.to_rust_string_lossy(scope1));

    let string = v8::EscapableHandleScope::new(scope1, |escapable_scope| {
      let nested_str_val = v8::EscapableHandleScope::new(
        escapable_scope,
        |nested_escapable_scope| {
          let string =
            v8::String::new(nested_escapable_scope, "Hello 🦕 world!").unwrap();
          string
        },
      );
      nested_str_val
    });
    assert_eq!("Hello 🦕 world!", string.to_rust_string_lossy(scope1));
  });
}
