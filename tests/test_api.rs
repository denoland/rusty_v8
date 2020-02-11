// Copyright 2019-2020 the Deno authors. All rights reserved. MIT license.

#[macro_use]
extern crate lazy_static;

//use std::convert::{Into, TryFrom, TryInto};
use std::sync::atomic::{AtomicUsize, Ordering};
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
      Some(v8::Number::new(escapable_scope, 78.9))
    });
    assert_eq!(number.unwrap().value(), 78.9);

    let string = v8::EscapableHandleScope::new(scope1, |escapable_scope| {
      let string = v8::String::new(escapable_scope, "Hello 🦕 world!").unwrap();
      Some(string)
    });
    assert_eq!(
      "Hello 🦕 world!",
      string.unwrap().to_rust_string_lossy(scope1)
    );

    let string = v8::EscapableHandleScope::new(scope1, |escapable_scope| {
      let nested_str_val = v8::EscapableHandleScope::new(
        escapable_scope,
        |nested_escapable_scope| {
          let string =
            v8::String::new(nested_escapable_scope, "Hello 🦕 world!").unwrap();
          Some(string)
        },
      )
      .unwrap();
      Some(nested_str_val)
    })
    .unwrap();
    assert_eq!("Hello 🦕 world!", string.to_rust_string_lossy(scope1));
  });
}

#[test]
fn microtasks() {
  let _setup_guard = setup();
  let mut params = v8::Isolate::create_params();
  params.set_array_buffer_allocator(v8::new_default_allocator());
  let mut isolate = v8::Isolate::new(params);

  isolate.run_microtasks();

  v8::HandleScope::new(&mut isolate, |scope| {
    let mut context = v8::Context::new(scope);
    context.enter();

    static CALL_COUNT: AtomicUsize = AtomicUsize::new(0);
    let function = v8::Function::new(
      scope,
      context,
      |_: &mut v8::Scope,
       _: v8::FunctionCallbackArguments,
       _: v8::ReturnValue| {
        CALL_COUNT.fetch_add(1, Ordering::SeqCst);
      },
    )
    .unwrap();
    scope.isolate().enqueue_microtask(function);

    assert_eq!(CALL_COUNT.load(Ordering::SeqCst), 0);
    scope.isolate().run_microtasks();
    assert_eq!(CALL_COUNT.load(Ordering::SeqCst), 1);

    context.exit();
  });
}

#[test]
fn array_buffer() {
  let _setup_guard = setup();
  let mut params = v8::Isolate::create_params();
  params.set_array_buffer_allocator(v8::new_default_allocator());
  let mut isolate = v8::Isolate::new(params);
  v8::HandleScope::new(&mut isolate, |scope| {
    let mut context = v8::Context::new(scope);
    context.enter();

    let ab = v8::ArrayBuffer::new(scope, 42);
    assert_eq!(42, ab.byte_length());

    let bs = v8::ArrayBuffer::new_backing_store(scope, 84);
    assert_eq!(84, bs.byte_length());
    assert_eq!(false, bs.is_shared());

    let data: Box<[u8]> = vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9].into_boxed_slice();
    let unique_bs = v8::ArrayBuffer::new_backing_store_from_boxed_slice(data);
    assert_eq!(10, unique_bs.byte_length());
    assert_eq!(false, unique_bs.is_shared());
    assert_eq!(unique_bs[0], 0);
    assert_eq!(unique_bs[9], 9);

    let mut shared_bs_1 = unique_bs.make_shared();
    {
      let bs = unsafe { &mut *shared_bs_1.get() };
      assert_eq!(10, bs.byte_length());
      assert_eq!(false, bs.is_shared());
      assert_eq!(bs[0], 0);
      assert_eq!(bs[9], 9);
    }

    let ab = v8::ArrayBuffer::with_backing_store(scope, &mut shared_bs_1);
    let shared_bs_2 = ab.get_backing_store();
    {
      let bs = unsafe { &mut *shared_bs_2.get() };
      assert_eq!(10, ab.byte_length());
      assert_eq!(bs[0], 0);
      assert_eq!(bs[9], 9);
    }

    context.exit();
  });
}

#[test]
fn array_buffer_with_shared_backing_store() {
  let _setup_guard = setup();
  let mut params = v8::Isolate::create_params();
  params.set_array_buffer_allocator(v8::new_default_allocator());
  let mut isolate = v8::Isolate::new(params);
  v8::HandleScope::new(&mut isolate, |scope| {
    let mut context = v8::Context::new(scope);
    context.enter();

    let ab1 = v8::ArrayBuffer::new(scope, 42);
    assert_eq!(42, ab1.byte_length());

    let bs1 = ab1.get_backing_store();
    assert_eq!(ab1.byte_length(), unsafe { (*bs1.get()).byte_length() });
    assert_eq!(2, v8::SharedRef::use_count(&bs1));

    let bs2 = ab1.get_backing_store();
    assert_eq!(ab1.byte_length(), unsafe { (*bs2.get()).byte_length() });
    assert_eq!(3, v8::SharedRef::use_count(&bs1));
    assert_eq!(3, v8::SharedRef::use_count(&bs2));

    let mut bs3 = ab1.get_backing_store();
    assert_eq!(ab1.byte_length(), unsafe { (*bs3.get()).byte_length() });
    assert_eq!(4, v8::SharedRef::use_count(&bs1));
    assert_eq!(4, v8::SharedRef::use_count(&bs2));
    assert_eq!(4, v8::SharedRef::use_count(&bs3));

    drop(bs2);
    assert_eq!(3, v8::SharedRef::use_count(&bs1));
    assert_eq!(3, v8::SharedRef::use_count(&bs3));

    drop(bs1);
    assert_eq!(2, v8::SharedRef::use_count(&bs3));

    let ab2 = v8::ArrayBuffer::with_backing_store(scope, &mut bs3);
    assert_eq!(ab1.byte_length(), ab2.byte_length());
    assert_eq!(3, v8::SharedRef::use_count(&bs3));

    let bs4 = ab2.get_backing_store();
    assert_eq!(ab1.byte_length(), unsafe { (*bs4.get()).byte_length() });
    assert_eq!(4, v8::SharedRef::use_count(&bs3));
    assert_eq!(4, v8::SharedRef::use_count(&bs4));

    let bs5 = bs4.clone();
    assert_eq!(5, v8::SharedRef::use_count(&bs3));
    assert_eq!(5, v8::SharedRef::use_count(&bs4));
    assert_eq!(5, v8::SharedRef::use_count(&bs5));

    drop(bs3);
    assert_eq!(4, v8::SharedRef::use_count(&bs4));
    assert_eq!(4, v8::SharedRef::use_count(&bs4));

    drop(bs4);
    assert_eq!(3, v8::SharedRef::use_count(&bs5));

    context.exit();
  });
}

fn v8_str(scope: &mut v8::Scope, s: &str) -> v8::Local<v8::String> {
  v8::String::new(scope, s).unwrap()
}

fn eval(
  scope: &mut v8::Scope,
  context: v8::Local<v8::Context>,
  code: &'static str,
) -> Option<v8::Local<v8::Value>> {
  v8::EscapableHandleScope::new(scope, |scope| {
    let source = v8_str(scope, code);
    let mut script = v8::Script::compile(scope, context, source, None).unwrap();
    script.run(scope, context)
  })
}

#[test]
fn try_catch() {
  let _setup_guard = setup();
  let mut params = v8::Isolate::create_params();
  params.set_array_buffer_allocator(v8::new_default_allocator());
  let mut isolate = v8::Isolate::new(params);
  v8::HandleScope::new(&mut isolate, |scope| {
    let mut context = v8::Context::new(scope);
    context.enter();

    v8::TryCatch::new(scope, |scope, tc| {
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
    });
    v8::TryCatch::new(scope, |scope, mut tc| {
      let result = eval(scope, context, "1 + 1");
      assert!(result.is_some());
      assert!(!tc.has_caught());
      assert!(tc.exception().is_none());
      assert!(tc.stack_trace(scope, context).is_none());
      assert!(tc.message().is_none());
      assert!(tc.rethrow().is_none());
    });
    // Rethrow and reset.
    v8::TryCatch::new(scope, |scope, tc1| {
      v8::TryCatch::new(scope, |scope, mut tc2| {
        eval(scope, context, "throw 'bar'");
        assert!(tc2.has_caught());
        assert!(tc2.rethrow().is_some());
        tc2.reset();
        assert!(!tc2.has_caught());
      });
      assert!(tc1.has_caught());
    });

    context.exit();
  });
}
