// Copyright 2019-2021 the Deno authors. All rights reserved. MIT license.
use lazy_static::lazy_static;
use std::any::type_name;
use std::cell::RefCell;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::convert::{Into, TryFrom, TryInto};
use std::ffi::c_void;
use std::ffi::CStr;
use std::hash::Hash;
use std::os::raw::c_char;
use std::ptr::NonNull;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::sync::Mutex;

// TODO(piscisaureus): Ideally there would be no need to import this trait.
use v8::MapFnTo;

#[must_use]
struct SetupGuard {}

impl Drop for SetupGuard {
  fn drop(&mut self) {
    // TODO shutdown process cleanly.
  }
}

fn setup() -> SetupGuard {
  static START: std::sync::Once = std::sync::Once::new();
  START.call_once(|| {
    assert!(v8::icu::set_common_data_69(align_data::include_aligned!(
      align_data::Align16,
      "../third_party/icu/common/icudtl.dat"
    ))
    .is_ok());
    v8::V8::set_flags_from_string("--expose_gc --harmony-import-assertions");
    v8::V8::initialize_platform(
      v8::new_default_platform(0, false).make_shared(),
    );
    v8::V8::initialize();
  });
  SetupGuard {}
}

#[test]
fn handle_scope_nested() {
  let _setup_guard = setup();
  let isolate = &mut v8::Isolate::new(Default::default());
  {
    let scope1 = &mut v8::HandleScope::new(isolate);
    {
      let _scope2 = &mut v8::HandleScope::new(scope1);
    }
  }
}

#[test]
#[allow(clippy::float_cmp)]
fn handle_scope_numbers() {
  let _setup_guard = setup();
  let isolate = &mut v8::Isolate::new(Default::default());
  {
    let scope1 = &mut v8::HandleScope::new(isolate);
    let l1 = v8::Integer::new(scope1, -123);
    let l2 = v8::Integer::new_from_unsigned(scope1, 456);
    {
      let scope2 = &mut v8::HandleScope::new(scope1);
      let l3 = v8::Number::new(scope2, 78.9);
      assert_eq!(l1.value(), -123);
      assert_eq!(l2.value(), 456);
      assert_eq!(l3.value(), 78.9);
      assert_eq!(v8::Number::value(&l1), -123f64);
      assert_eq!(v8::Number::value(&l2), 456f64);
    }
  }
}

#[test]
fn handle_scope_non_lexical_lifetime() {
  let _setup_guard = setup();
  let isolate = &mut v8::Isolate::new(Default::default());
  let scope1 = &mut v8::HandleScope::new(isolate);

  // Despite `local` living slightly longer than `scope2`, this test should
  // not crash.
  let local = {
    let scope2 = &mut v8::HandleScope::new(scope1);
    v8::Integer::new(scope2, 123)
  };
  assert_eq!(local.value(), 123);
}

#[test]
fn global_handles() {
  let _setup_guard = setup();
  let isolate = &mut v8::Isolate::new(Default::default());
  let g1: v8::Global<v8::String>;
  let mut g2: Option<v8::Global<v8::Integer>> = None;
  let g3: v8::Global<v8::Integer>;
  let g4: v8::Global<v8::Integer>;
  let mut g5: Option<v8::Global<v8::Integer>> = None;
  let g6;
  {
    let scope = &mut v8::HandleScope::new(isolate);
    let l1 = v8::String::new(scope, "bla").unwrap();
    let l2 = v8::Integer::new(scope, 123);
    g1 = v8::Global::new(scope, l1);
    g2.replace(v8::Global::new(scope, l2));
    g3 = v8::Global::new(scope, g2.as_ref().unwrap());
    g4 = v8::Global::new(scope, l2);
    let l5 = v8::Integer::new(scope, 100);
    g5.replace(v8::Global::new(scope, l5));
    g6 = g1.clone();
  }
  {
    let scope = &mut v8::HandleScope::new(isolate);
    assert_eq!(g1.open(scope).to_rust_string_lossy(scope), "bla");
    assert_eq!(g2.as_ref().unwrap().open(scope).value(), 123);
    assert_eq!(g3.open(scope).value(), 123);
    assert_eq!(g4.open(scope).value(), 123);
    {
      let num = g5.as_ref().unwrap().open(scope);
      assert_eq!(num.value(), 100);
    }
    g5.take();
    assert!(g6 == g1);
    assert_eq!(g6.open(scope).to_rust_string_lossy(scope), "bla");
  }
}

#[test]
fn local_handle_deref() {
  let _setup_guard = setup();
  let isolate = &mut v8::Isolate::new(Default::default());
  let scope = &mut v8::HandleScope::new(isolate);
  let context = v8::Context::new(scope);
  let scope = &mut v8::ContextScope::new(scope, context);
  let key = v8::String::new(scope, "key").unwrap();
  let obj: v8::Local<v8::Object> = v8::Object::new(scope);
  obj.get(scope, key.into());
  {
    use v8::Handle;
    obj.get(scope, key.into());
    obj.open(scope).get(scope, key.into());
  }
}

#[test]
fn global_handle_drop() {
  let _setup_guard = setup();

  // Global 'g1' will be dropped _after_ the Isolate has been disposed.
  let _g1: v8::Global<v8::String>;

  let isolate = &mut v8::Isolate::new(Default::default());
  let scope = &mut v8::HandleScope::new(isolate);

  let l1 = v8::String::new(scope, "foo").unwrap();
  _g1 = v8::Global::new(scope, l1);

  // Global 'g2' will be dropped _before_ the Isolate has been disposed.
  let l2 = v8::String::new(scope, "bar").unwrap();
  let _g2 = v8::Global::new(scope, l2);
}

#[test]
fn test_string() {
  let _setup_guard = setup();
  let isolate = &mut v8::Isolate::new(Default::default());
  {
    let scope = &mut v8::HandleScope::new(isolate);
    let reference = "Hello 🦕 world!";
    let local = v8::String::new(scope, reference).unwrap();
    assert_eq!(15, local.length());
    assert_eq!(17, local.utf8_length(scope));
    assert_eq!(reference, local.to_rust_string_lossy(scope));
    let mut vec = Vec::new();
    vec.resize(17, 0);
    let options = v8::WriteOptions::NO_NULL_TERMINATION;
    let mut nchars = 0;
    assert_eq!(
      17,
      local.write_utf8(scope, &mut vec, Some(&mut nchars), options)
    );
    assert_eq!(15, nchars);
    let mut u16_buffer = [0u16; 16];
    assert_eq!(15, local.write(scope, &mut u16_buffer, 0, options));
    assert_eq!(
      String::from(reference),
      String::from_utf16(&u16_buffer[..15]).unwrap()
    );
  }
  {
    let scope = &mut v8::HandleScope::new(isolate);
    let local = v8::String::empty(scope);
    assert_eq!(0, local.length());
    assert_eq!(0, local.utf8_length(scope));
    assert_eq!("", local.to_rust_string_lossy(scope));
  }
  {
    let scope = &mut v8::HandleScope::new(isolate);
    let local =
      v8::String::new_from_utf8(scope, b"", v8::NewStringType::Normal).unwrap();
    assert_eq!(0, local.length());
    assert_eq!(0, local.utf8_length(scope));
    assert_eq!("", local.to_rust_string_lossy(scope));
  }
  {
    let scope = &mut v8::HandleScope::new(isolate);
    let local =
      v8::String::new_from_one_byte(scope, b"foo", v8::NewStringType::Normal)
        .unwrap();
    assert_eq!(3, local.length());
    assert_eq!(3, local.utf8_length(scope));
    let options = v8::WriteOptions::NO_NULL_TERMINATION;
    let mut buffer = [0u8; 3];
    assert_eq!(3, local.write_one_byte(scope, &mut buffer, 0, options));
    assert_eq!(b"foo", &buffer);
    assert_eq!("foo", local.to_rust_string_lossy(scope));
  }
  {
    let scope = &mut v8::HandleScope::new(isolate);
    let local = v8::String::new_from_two_byte(
      scope,
      &[0xD83E, 0xDD95],
      v8::NewStringType::Normal,
    )
    .unwrap();
    assert_eq!(2, local.length());
    assert_eq!(4, local.utf8_length(scope));
    assert_eq!("🦕", local.to_rust_string_lossy(scope));
  }
}

#[test]
#[allow(clippy::float_cmp)]
fn escapable_handle_scope() {
  let _setup_guard = setup();
  let isolate = &mut v8::Isolate::new(Default::default());
  {
    let handle_scope = &mut v8::HandleScope::new(isolate);

    // After dropping EscapableHandleScope, we should be able to
    // read escaped values.
    let number = {
      let escapable_scope = &mut v8::EscapableHandleScope::new(handle_scope);
      let number = v8::Number::new(escapable_scope, 78.9);
      escapable_scope.escape(number)
    };
    assert_eq!(number.value(), 78.9);

    let string = {
      let escapable_scope = &mut v8::EscapableHandleScope::new(handle_scope);
      let string = v8::String::new(escapable_scope, "Hello 🦕 world!").unwrap();
      escapable_scope.escape(string)
    };
    assert_eq!("Hello 🦕 world!", string.to_rust_string_lossy(handle_scope));

    let string = {
      let escapable_scope = &mut v8::EscapableHandleScope::new(handle_scope);
      let nested_str_val = {
        let nested_escapable_scope =
          &mut v8::EscapableHandleScope::new(escapable_scope);
        let string =
          v8::String::new(nested_escapable_scope, "Hello 🦕 world!").unwrap();
        nested_escapable_scope.escape(string)
      };
      escapable_scope.escape(nested_str_val)
    };
    assert_eq!("Hello 🦕 world!", string.to_rust_string_lossy(handle_scope));
  }
}

#[test]
#[should_panic(expected = "EscapableHandleScope::escape() called twice")]
fn escapable_handle_scope_can_escape_only_once() {
  let _setup_guard = setup();
  let isolate = &mut v8::Isolate::new(Default::default());

  let scope1 = &mut v8::HandleScope::new(isolate);
  let scope2 = &mut v8::EscapableHandleScope::new(scope1);

  let local1 = v8::Integer::new(scope2, -123);
  let escaped1 = scope2.escape(local1);
  assert!(escaped1 == local1);

  let local2 = v8::Integer::new(scope2, 456);
  let escaped2 = scope2.escape(local2);
  assert!(escaped2 == local2);
}

#[test]
fn context_scope() {
  let _setup_guard = setup();
  let isolate = &mut v8::Isolate::new(Default::default());

  let scope = &mut v8::HandleScope::new(isolate);
  let context1 = v8::Context::new(scope);
  let scope = &mut v8::ContextScope::new(scope, context1);

  assert!(scope.get_current_context() == context1);
  assert!(scope.get_entered_or_microtask_context() == context1);

  {
    let context2 = v8::Context::new(scope);
    let scope = &mut v8::ContextScope::new(scope, context2);

    assert!(scope.get_current_context() == context2);
    assert!(scope.get_entered_or_microtask_context() == context2);
  }

  assert!(scope.get_current_context() == context1);
  assert!(scope.get_entered_or_microtask_context() == context1);
}

#[test]
#[should_panic(
  expected = "HandleScope<()> and Context do not belong to the same Isolate"
)]
fn context_scope_param_and_context_must_share_isolate() {
  let _setup_guard = setup();
  let isolate1 = &mut v8::Isolate::new(Default::default());
  let isolate2 = &mut v8::Isolate::new(Default::default());
  let scope1 = &mut v8::HandleScope::new(isolate1);
  let scope2 = &mut v8::HandleScope::new(isolate2);
  let context1 = v8::Context::new(scope1);
  let context2 = v8::Context::new(scope2);
  let _context_scope_12 = &mut v8::ContextScope::new(scope1, context2);
  let _context_scope_21 = &mut v8::ContextScope::new(scope2, context1);
}

#[test]
#[should_panic(
  expected = "attempt to use Handle in an Isolate that is not its host"
)]
fn handle_scope_param_and_context_must_share_isolate() {
  let _setup_guard = setup();
  let isolate1 = &mut v8::Isolate::new(Default::default());
  let isolate2 = &mut v8::Isolate::new(Default::default());
  let global_context1;
  let global_context2;
  {
    let scope1 = &mut v8::HandleScope::new(isolate1);
    let scope2 = &mut v8::HandleScope::new(isolate2);
    let local_context_1 = v8::Context::new(scope1);
    let local_context_2 = v8::Context::new(scope2);
    global_context1 = v8::Global::new(scope1, local_context_1);
    global_context2 = v8::Global::new(scope2, local_context_2);
  }
  let _handle_scope_12 =
    &mut v8::HandleScope::with_context(isolate1, global_context2);
  let _handle_scope_21 =
    &mut v8::HandleScope::with_context(isolate2, global_context1);
}

#[test]
fn microtasks() {
  let _setup_guard = setup();
  let isolate = &mut v8::Isolate::new(Default::default());

  assert_eq!(isolate.get_microtasks_policy(), v8::MicrotasksPolicy::Auto);
  isolate.set_microtasks_policy(v8::MicrotasksPolicy::Explicit);
  assert_eq!(
    isolate.get_microtasks_policy(),
    v8::MicrotasksPolicy::Explicit
  );

  isolate.perform_microtask_checkpoint();

  {
    let scope = &mut v8::HandleScope::new(isolate);
    let context = v8::Context::new(scope);
    let scope = &mut v8::ContextScope::new(scope, context);

    static CALL_COUNT: AtomicUsize = AtomicUsize::new(0);
    let function = v8::Function::new(
      scope,
      |_: &mut v8::HandleScope,
       _: v8::FunctionCallbackArguments,
       _: v8::ReturnValue| {
        CALL_COUNT.fetch_add(1, Ordering::SeqCst);
      },
    )
    .unwrap();
    scope.enqueue_microtask(function);

    // Flushes the microtasks queue unless the policy is set to explicit.
    let _ = eval(scope, "").unwrap();

    assert_eq!(CALL_COUNT.load(Ordering::SeqCst), 0);
    scope.perform_microtask_checkpoint();
    assert_eq!(CALL_COUNT.load(Ordering::SeqCst), 1);

    scope.set_microtasks_policy(v8::MicrotasksPolicy::Auto);
    assert_eq!(scope.get_microtasks_policy(), v8::MicrotasksPolicy::Auto);
    scope.enqueue_microtask(function);

    let _ = eval(scope, "").unwrap();

    assert_eq!(CALL_COUNT.load(Ordering::SeqCst), 2);
  }
}

#[test]
fn get_isolate_from_handle() {
  extern "C" {
    fn v8__internal__GetIsolateFromHeapObject(
      location: *const v8::Data,
    ) -> *mut v8::Isolate;
  }

  fn check_handle_helper(
    isolate: &mut v8::Isolate,
    expect_some: Option<bool>,
    local: v8::Local<v8::Data>,
  ) {
    let isolate_ptr = NonNull::from(isolate);
    let maybe_ptr = unsafe { v8__internal__GetIsolateFromHeapObject(&*local) };
    let maybe_ptr = NonNull::new(maybe_ptr);
    if let Some(ptr) = maybe_ptr {
      assert_eq!(ptr, isolate_ptr);
    }
    if let Some(expected_some) = expect_some {
      assert_eq!(maybe_ptr.is_some(), expected_some);
    }
  }

  fn check_handle<'s, F, D>(
    scope: &mut v8::HandleScope<'s>,
    expect_some: Option<bool>,
    f: F,
  ) where
    F: Fn(&mut v8::HandleScope<'s>) -> D,
    D: Into<v8::Local<'s, v8::Data>>,
  {
    let local = f(scope).into();

    // Check that we can get the isolate from a Local.
    check_handle_helper(scope, expect_some, local);

    // Check that we can still get it after converting it to a Global.
    let global = v8::Global::new(scope, local);
    let local2 = v8::Local::new(scope, &global);
    check_handle_helper(scope, expect_some, local2);
  }

  fn check_eval<'s>(
    scope: &mut v8::HandleScope<'s>,
    expect_some: Option<bool>,
    code: &str,
  ) {
    check_handle(scope, expect_some, |scope| eval(scope, code).unwrap());
  }

  let _setup_guard = setup();
  let isolate = &mut v8::Isolate::new(Default::default());

  let scope = &mut v8::HandleScope::new(isolate);
  let context = v8::Context::new(scope);
  let scope = &mut v8::ContextScope::new(scope, context);

  check_handle(scope, None, |s| v8::null(s));
  check_handle(scope, None, |s| v8::undefined(s));
  check_handle(scope, None, |s| v8::Boolean::new(s, true));
  check_handle(scope, None, |s| v8::Boolean::new(s, false));
  check_handle(scope, None, |s| v8::String::new(s, "").unwrap());
  check_eval(scope, None, "''");
  check_handle(scope, Some(true), |s| v8::String::new(s, "Words").unwrap());
  check_eval(scope, Some(true), "'Hello'");
  check_eval(scope, Some(true), "Symbol()");
  check_handle(scope, Some(true), |s| v8::Object::new(s));
  check_eval(scope, Some(true), "this");
  check_handle(scope, Some(true), |s| s.get_current_context());
  check_eval(scope, Some(true), "({ foo: 'bar' })");
  check_eval(scope, Some(true), "() => {}");
  check_handle(scope, Some(true), |s| v8::Number::new(s, 4.2f64));
  check_handle(scope, Some(true), |s| v8::Number::new(s, -0f64));
  check_handle(scope, Some(false), |s| v8::Integer::new(s, 0));
  check_eval(scope, Some(true), "3.3");
  check_eval(scope, Some(false), "3.3 / 3.3");
}

#[test]
fn array_buffer() {
  let _setup_guard = setup();
  let isolate = &mut v8::Isolate::new(Default::default());
  {
    let scope = &mut v8::HandleScope::new(isolate);
    let context = v8::Context::new(scope);
    let scope = &mut v8::ContextScope::new(scope, context);

    let ab = v8::ArrayBuffer::new(scope, 42);
    assert_eq!(42, ab.byte_length());

    assert!(ab.is_detachable());
    ab.detach();
    assert_eq!(0, ab.byte_length());
    ab.detach(); // Calling it twice should be a no-op.

    let bs = v8::ArrayBuffer::new_backing_store(scope, 84);
    assert_eq!(84, bs.byte_length());
    assert!(!bs.is_shared());

    let data: Box<[u8]> = vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9].into_boxed_slice();
    let unique_bs = v8::ArrayBuffer::new_backing_store_from_boxed_slice(data);
    assert_eq!(10, unique_bs.byte_length());
    assert!(!unique_bs.is_shared());
    assert_eq!(unique_bs[0].get(), 0);
    assert_eq!(unique_bs[9].get(), 9);

    let shared_bs_1 = unique_bs.make_shared();
    assert_eq!(10, shared_bs_1.byte_length());
    assert!(!shared_bs_1.is_shared());
    assert_eq!(shared_bs_1[0].get(), 0);
    assert_eq!(shared_bs_1[9].get(), 9);

    let ab = v8::ArrayBuffer::with_backing_store(scope, &shared_bs_1);
    let shared_bs_2 = ab.get_backing_store();
    assert_eq!(10, shared_bs_2.byte_length());
    assert_eq!(shared_bs_2[0].get(), 0);
    assert_eq!(shared_bs_2[9].get(), 9);
  }
}

#[test]
fn backing_store_segfault() {
  let _setup_guard = setup();
  let array_buffer_allocator = v8::new_default_allocator().make_shared();
  let shared_bs = {
    array_buffer_allocator.assert_use_count_eq(1);
    let params = v8::Isolate::create_params()
      .array_buffer_allocator(array_buffer_allocator.clone());
    array_buffer_allocator.assert_use_count_eq(2);
    let isolate = &mut v8::Isolate::new(params);
    array_buffer_allocator.assert_use_count_eq(2);
    let scope = &mut v8::HandleScope::new(isolate);
    let context = v8::Context::new(scope);
    let scope = &mut v8::ContextScope::new(scope, context);
    let ab = v8::ArrayBuffer::new(scope, 10);
    let shared_bs = ab.get_backing_store();
    array_buffer_allocator.assert_use_count_eq(3);
    shared_bs
  };
  shared_bs.assert_use_count_eq(1);
  array_buffer_allocator.assert_use_count_eq(2);
  drop(array_buffer_allocator);
  drop(shared_bs); // Error occurred here.
}

#[test]
fn shared_array_buffer_allocator() {
  let alloc1 = v8::new_default_allocator().make_shared();
  alloc1.assert_use_count_eq(1);

  let alloc2 = alloc1.clone();
  alloc1.assert_use_count_eq(2);
  alloc2.assert_use_count_eq(2);

  let mut alloc2 = v8::SharedPtr::from(alloc2);
  alloc1.assert_use_count_eq(2);
  alloc2.assert_use_count_eq(2);

  drop(alloc1);
  alloc2.assert_use_count_eq(1);

  alloc2.take();
  alloc2.assert_use_count_eq(0);
}

#[test]
fn array_buffer_with_shared_backing_store() {
  let _setup_guard = setup();
  let isolate = &mut v8::Isolate::new(Default::default());
  {
    let scope = &mut v8::HandleScope::new(isolate);

    let context = v8::Context::new(scope);
    let scope = &mut v8::ContextScope::new(scope, context);

    let ab1 = v8::ArrayBuffer::new(scope, 42);
    assert_eq!(42, ab1.byte_length());

    let bs1 = ab1.get_backing_store();
    assert_eq!(ab1.byte_length(), bs1.byte_length());
    bs1.assert_use_count_eq(2);

    let bs2 = ab1.get_backing_store();
    assert_eq!(ab1.byte_length(), bs2.byte_length());
    bs1.assert_use_count_eq(3);
    bs2.assert_use_count_eq(3);

    let bs3 = ab1.get_backing_store();
    assert_eq!(ab1.byte_length(), bs3.byte_length());
    bs1.assert_use_count_eq(4);
    bs2.assert_use_count_eq(4);
    bs3.assert_use_count_eq(4);

    drop(bs2);
    bs1.assert_use_count_eq(3);
    bs3.assert_use_count_eq(3);

    drop(bs1);
    bs3.assert_use_count_eq(2);

    let ab2 = v8::ArrayBuffer::with_backing_store(scope, &bs3);
    assert_eq!(ab1.byte_length(), ab2.byte_length());
    bs3.assert_use_count_eq(3);

    let bs4 = ab2.get_backing_store();
    assert_eq!(ab1.byte_length(), bs4.byte_length());
    bs3.assert_use_count_eq(4);
    bs4.assert_use_count_eq(4);

    let bs5 = bs4.clone();
    bs3.assert_use_count_eq(5);
    bs4.assert_use_count_eq(5);
    bs5.assert_use_count_eq(5);

    drop(bs3);
    bs4.assert_use_count_eq(4);
    bs5.assert_use_count_eq(4);

    drop(bs4);
    bs5.assert_use_count_eq(3);
  }
}

#[test]
fn deref_empty_backing_store() {
  // Test that the slice that results from derefing a backing store is not
  // backed by a null pointer, since that would be UB.

  let _setup_guard = setup();
  let isolate = &mut v8::Isolate::new(Default::default());

  let backing_store = v8::ArrayBuffer::new_backing_store(isolate, 0);
  let slice: &[std::cell::Cell<u8>] = &backing_store;
  assert!(!slice.as_ptr().is_null());
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

#[test]
fn external() {
  let _setup_guard = setup();
  let isolate = &mut v8::Isolate::new(Default::default());
  let scope = &mut v8::HandleScope::new(isolate);

  let ex1_value = 1usize as *mut std::ffi::c_void;
  let ex1_handle_a = v8::External::new(scope, ex1_value);
  assert_eq!(ex1_handle_a.value(), ex1_value);

  let context = v8::Context::new(scope);
  let scope = &mut v8::ContextScope::new(scope, context);
  let global = context.global(scope);

  let ex2_value = 2334567usize as *mut std::ffi::c_void;
  let ex3_value = -2isize as *mut std::ffi::c_void;

  let ex2_handle_a = v8::External::new(scope, ex2_value);
  let ex3_handle_a = v8::External::new(scope, ex3_value);

  assert!(ex1_handle_a != ex2_handle_a);
  assert!(ex2_handle_a != ex3_handle_a);
  assert!(ex3_handle_a != ex1_handle_a);

  assert_ne!(ex2_value, ex3_value);
  assert_eq!(ex2_handle_a.value(), ex2_value);
  assert_eq!(ex3_handle_a.value(), ex3_value);

  let ex1_key = v8::String::new(scope, "ex1").unwrap().into();
  let ex2_key = v8::String::new(scope, "ex2").unwrap().into();
  let ex3_key = v8::String::new(scope, "ex3").unwrap().into();

  global.set(scope, ex1_key, ex1_handle_a.into());
  global.set(scope, ex2_key, ex2_handle_a.into());
  global.set(scope, ex3_key, ex3_handle_a.into());

  let ex1_handle_b: v8::Local<v8::External> =
    eval(scope, "ex1").unwrap().try_into().unwrap();
  let ex2_handle_b: v8::Local<v8::External> =
    eval(scope, "ex2").unwrap().try_into().unwrap();
  let ex3_handle_b: v8::Local<v8::External> =
    eval(scope, "ex3").unwrap().try_into().unwrap();

  assert!(ex1_handle_b != ex2_handle_b);
  assert!(ex2_handle_b != ex3_handle_b);
  assert!(ex3_handle_b != ex1_handle_b);

  assert!(ex1_handle_a == ex1_handle_b);
  assert!(ex2_handle_a == ex2_handle_b);
  assert!(ex3_handle_a == ex3_handle_b);

  assert_ne!(ex1_handle_a.value(), ex2_value);
  assert_ne!(ex2_handle_a.value(), ex3_value);
  assert_ne!(ex3_handle_a.value(), ex1_value);

  assert_eq!(ex1_handle_a.value(), ex1_value);
  assert_eq!(ex2_handle_a.value(), ex2_value);
  assert_eq!(ex3_handle_a.value(), ex3_value);
}

#[test]
fn try_catch() {
  let _setup_guard = setup();
  let isolate = &mut v8::Isolate::new(Default::default());
  {
    let scope = &mut v8::HandleScope::new(isolate);
    let context = v8::Context::new(scope);
    let scope = &mut v8::ContextScope::new(scope, context);
    {
      // Error thrown - should be caught.
      let tc = &mut v8::TryCatch::new(scope);
      let result = eval(tc, "throw new Error('foo')");
      assert!(result.is_none());
      assert!(tc.has_caught());
      assert!(tc.exception().is_some());
      assert!(tc.stack_trace().is_some());
      assert!(tc.message().is_some());
      assert_eq!(
        tc.message().unwrap().get(tc).to_rust_string_lossy(tc),
        "Uncaught Error: foo"
      );
    };
    {
      // No error thrown.
      let tc = &mut v8::TryCatch::new(scope);
      let result = eval(tc, "1 + 1");
      assert!(result.is_some());
      assert!(!tc.has_caught());
      assert!(tc.exception().is_none());
      assert!(tc.stack_trace().is_none());
      assert!(tc.message().is_none());
      assert!(tc.rethrow().is_none());
    };
    {
      // Rethrow and reset.
      let tc1 = &mut v8::TryCatch::new(scope);
      {
        let tc2 = &mut v8::TryCatch::new(tc1);
        eval(tc2, "throw 'bar'");
        assert!(tc2.has_caught());
        assert!(tc2.rethrow().is_some());
        tc2.reset();
        assert!(!tc2.has_caught());
      }
      assert!(tc1.has_caught());
    };
  }
}

#[test]
fn try_catch_caught_lifetime() {
  let _setup_guard = setup();
  let isolate = &mut v8::Isolate::new(Default::default());
  let scope = &mut v8::HandleScope::new(isolate);
  let context = v8::Context::new(scope);
  let scope = &mut v8::ContextScope::new(scope, context);
  let (caught_exc, caught_msg) = {
    let tc = &mut v8::TryCatch::new(scope);
    // Throw exception.
    let msg = v8::String::new(tc, "DANG!").unwrap();
    let exc = v8::Exception::type_error(tc, msg);
    tc.throw_exception(exc);
    // Catch exception.
    let caught_exc = tc.exception().unwrap();
    let caught_msg = tc.message().unwrap();
    // Move `caught_exc` and `caught_msg` out of the extent of the TryCatch,
    // but still within the extent of the enclosing HandleScope.
    (caught_exc, caught_msg)
  };
  // This should not crash.
  assert!(caught_exc.to_rust_string_lossy(scope).contains("DANG"));
  assert!(caught_msg
    .get(scope)
    .to_rust_string_lossy(scope)
    .contains("DANG"));
}

#[test]
fn throw_exception() {
  let _setup_guard = setup();
  let isolate = &mut v8::Isolate::new(Default::default());
  {
    let scope = &mut v8::HandleScope::new(isolate);
    let context = v8::Context::new(scope);
    let scope = &mut v8::ContextScope::new(scope, context);
    {
      let tc = &mut v8::TryCatch::new(scope);
      let exception = v8::String::new(tc, "boom").unwrap();
      tc.throw_exception(exception.into());
      assert!(tc.has_caught());
      assert!(tc
        .exception()
        .unwrap()
        .strict_equals(v8::String::new(tc, "boom").unwrap().into()));
    };
  }
}

#[test]
fn isolate_termination_methods() {
  let _setup_guard = setup();
  let isolate = v8::Isolate::new(Default::default());
  let handle = isolate.thread_safe_handle();
  drop(isolate);
  assert!(!handle.terminate_execution());
  assert!(!handle.cancel_terminate_execution());
  assert!(!handle.is_execution_terminating());
  static CALL_COUNT: AtomicUsize = AtomicUsize::new(0);
  extern "C" fn callback(
    _isolate: &mut v8::Isolate,
    data: *mut std::ffi::c_void,
  ) {
    assert_eq!(data, std::ptr::null_mut());
    CALL_COUNT.fetch_add(1, Ordering::SeqCst);
  }
  assert!(!handle.request_interrupt(callback, std::ptr::null_mut()));
  assert_eq!(CALL_COUNT.load(Ordering::SeqCst), 0);
}

#[test]
fn thread_safe_handle_drop_after_isolate() {
  let _setup_guard = setup();
  let isolate = v8::Isolate::new(Default::default());
  let handle = isolate.thread_safe_handle();
  // We can call it twice.
  let handle_ = isolate.thread_safe_handle();
  // Check that handle is Send and Sync.
  fn f<S: Send + Sync>(_: S) {}
  f(handle_);
  // All methods on IsolateHandle should return false after the isolate is
  // dropped.
  drop(isolate);
  assert!(!handle.terminate_execution());
  assert!(!handle.cancel_terminate_execution());
  assert!(!handle.is_execution_terminating());
  static CALL_COUNT: AtomicUsize = AtomicUsize::new(0);
  extern "C" fn callback(
    _isolate: &mut v8::Isolate,
    data: *mut std::ffi::c_void,
  ) {
    assert_eq!(data, std::ptr::null_mut());
    CALL_COUNT.fetch_add(1, Ordering::SeqCst);
  }
  assert!(!handle.request_interrupt(callback, std::ptr::null_mut()));
  assert_eq!(CALL_COUNT.load(Ordering::SeqCst), 0);
}

#[test]
fn terminate_execution() {
  let _setup_guard = setup();
  let isolate = &mut v8::Isolate::new(Default::default());
  let (tx, rx) = std::sync::mpsc::channel::<bool>();
  let handle = isolate.thread_safe_handle();

  let scope = &mut v8::HandleScope::new(isolate);
  let context = v8::Context::new(scope);
  let scope = &mut v8::ContextScope::new(scope, context);

  let t = std::thread::spawn(move || {
    // allow deno to boot and run
    std::thread::sleep(std::time::Duration::from_millis(300));
    handle.terminate_execution();
    // allow shutdown
    std::thread::sleep(std::time::Duration::from_millis(200));
    // unless reported otherwise the test should fail after this point
    tx.send(false).ok();
  });

  // Run an infinite loop, which should be terminated.
  let source = v8::String::new(scope, "for(;;) {}").unwrap();
  let r = v8::Script::compile(scope, source, None);
  let script = r.unwrap();
  let result = script.run(scope);
  assert!(result.is_none());
  // TODO assert_eq!(e.to_string(), "Uncaught Error: execution terminated")
  let msg = rx.recv().expect("execution should be terminated");
  assert!(!msg);
  // Make sure the isolate unusable again.
  eval(scope, "1+1").expect("execution should be possible again");
  t.join().expect("join t");
}

// TODO(ry) This test should use threads
#[test]
fn request_interrupt_small_scripts() {
  let _setup_guard = setup();
  let isolate = &mut v8::Isolate::new(Default::default());
  let handle = isolate.thread_safe_handle();
  {
    let scope = &mut v8::HandleScope::new(isolate);
    let context = v8::Context::new(scope);
    let scope = &mut v8::ContextScope::new(scope, context);

    static CALL_COUNT: AtomicUsize = AtomicUsize::new(0);
    extern "C" fn callback(
      _isolate: &mut v8::Isolate,
      data: *mut std::ffi::c_void,
    ) {
      assert_eq!(data, std::ptr::null_mut());
      CALL_COUNT.fetch_add(1, Ordering::SeqCst);
    }
    handle.request_interrupt(callback, std::ptr::null_mut());
    eval(scope, "(function(x){return x;})(1);");
    assert_eq!(CALL_COUNT.load(Ordering::SeqCst), 1);
  }
}

#[test]
fn add_message_listener() {
  let _setup_guard = setup();
  let isolate = &mut v8::Isolate::new(Default::default());
  isolate.set_capture_stack_trace_for_uncaught_exceptions(true, 32);

  static CALL_COUNT: AtomicUsize = AtomicUsize::new(0);

  extern "C" fn check_message_0(
    message: v8::Local<v8::Message>,
    _exception: v8::Local<v8::Value>,
  ) {
    let scope = &mut unsafe { v8::CallbackScope::new(message) };
    let scope = &mut v8::HandleScope::new(scope);
    let message_str = message.get(scope);
    assert_eq!(message_str.to_rust_string_lossy(scope), "Uncaught foo");
    assert_eq!(Some(1), message.get_line_number(scope));
    assert!(message.get_script_resource_name(scope).is_some());
    assert!(message.get_source_line(scope).is_some());
    assert_eq!(message.get_start_position(), 0);
    assert_eq!(message.get_end_position(), 1);
    assert_eq!(message.get_wasm_function_index(), -1);
    assert!(message.error_level() >= 0);
    assert_eq!(message.get_start_column(), 0);
    assert_eq!(message.get_end_column(), 1);
    assert!(!message.is_shared_cross_origin());
    assert!(!message.is_opaque());
    let stack_trace = message.get_stack_trace(scope).unwrap();
    assert_eq!(1, stack_trace.get_frame_count());
    let frame = stack_trace.get_frame(scope, 0).unwrap();
    assert_eq!(1, frame.get_line_number());
    assert_eq!(1, frame.get_column());
    // Note: V8 flags like --expose_externalize_string and --expose_gc install
    // scripts of their own and therefore affect the script id that we get.
    assert_eq!(4, frame.get_script_id());
    assert!(frame.get_script_name(scope).is_none());
    assert!(frame.get_script_name_or_source_url(scope).is_none());
    assert!(frame.get_function_name(scope).is_none());
    assert!(!frame.is_eval());
    assert!(!frame.is_constructor());
    assert!(!frame.is_wasm());
    assert!(frame.is_user_javascript());
    CALL_COUNT.fetch_add(1, Ordering::SeqCst);
  }
  isolate.add_message_listener(check_message_0);

  {
    let scope = &mut v8::HandleScope::new(isolate);
    let context = v8::Context::new(scope);
    let scope = &mut v8::ContextScope::new(scope, context);
    let source = v8::String::new(scope, "throw 'foo'").unwrap();
    let script = v8::Script::compile(scope, source, None).unwrap();
    assert!(script.run(scope).is_none());
    assert_eq!(CALL_COUNT.load(Ordering::SeqCst), 1);
  }
}

fn unexpected_module_resolve_callback<'a>(
  _context: v8::Local<'a, v8::Context>,
  _specifier: v8::Local<'a, v8::String>,
  _import_assertions: v8::Local<'a, v8::FixedArray>,
  _referrer: v8::Local<'a, v8::Module>,
) -> Option<v8::Local<'a, v8::Module>> {
  unreachable!()
}

#[test]
fn set_host_initialize_import_meta_object_callback() {
  let _setup_guard = setup();
  let isolate = &mut v8::Isolate::new(Default::default());

  static CALL_COUNT: AtomicUsize = AtomicUsize::new(0);

  extern "C" fn callback(
    context: v8::Local<v8::Context>,
    _module: v8::Local<v8::Module>,
    meta: v8::Local<v8::Object>,
  ) {
    CALL_COUNT.fetch_add(1, Ordering::SeqCst);
    let scope = &mut unsafe { v8::CallbackScope::new(context) };
    let scope = &mut v8::HandleScope::new(scope);
    let key = v8::String::new(scope, "foo").unwrap();
    let value = v8::String::new(scope, "bar").unwrap();
    meta.create_data_property(scope, key.into(), value.into());
  }
  isolate.set_host_initialize_import_meta_object_callback(callback);

  {
    let scope = &mut v8::HandleScope::new(isolate);
    let context = v8::Context::new(scope);
    let scope = &mut v8::ContextScope::new(scope, context);
    let source = mock_source(
      scope,
      "google.com",
      "if (import.meta.foo != 'bar') throw 'bad'",
    );
    let module = v8::script_compiler::compile_module(scope, source).unwrap();
    let result =
      module.instantiate_module(scope, unexpected_module_resolve_callback);
    assert!(result.is_some());
    module.evaluate(scope).unwrap();
    assert_eq!(v8::ModuleStatus::Evaluated, module.get_status());
    assert_eq!(CALL_COUNT.load(Ordering::SeqCst), 1);
  }
}

#[test]
fn script_compile_and_run() {
  let _setup_guard = setup();
  let isolate = &mut v8::Isolate::new(Default::default());
  {
    let scope = &mut v8::HandleScope::new(isolate);
    let context = v8::Context::new(scope);
    let scope = &mut v8::ContextScope::new(scope, context);
    let source = v8::String::new(scope, "'Hello ' + 13 + 'th planet'").unwrap();
    let script = v8::Script::compile(scope, source, None).unwrap();
    source.to_rust_string_lossy(scope);
    let result = script.run(scope).unwrap();
    assert_eq!(result.to_rust_string_lossy(scope), "Hello 13th planet");
  }
}

#[test]
fn script_origin() {
  let _setup_guard = setup();
  let isolate = &mut v8::Isolate::new(Default::default());

  {
    let scope = &mut v8::HandleScope::new(isolate);
    let context = v8::Context::new(scope);
    let scope = &mut v8::ContextScope::new(scope, context);

    let resource_name = v8::String::new(scope, "foo.js").unwrap();
    let resource_line_offset = 4;
    let resource_column_offset = 5;
    let resource_is_shared_cross_origin = true;
    let script_id = 123;
    let source_map_url = v8::String::new(scope, "source_map_url").unwrap();
    let resource_is_opaque = true;
    let is_wasm = false;
    let is_module = false;

    let script_origin = v8::ScriptOrigin::new(
      scope,
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

    let source = v8::String::new(scope, "1+2").unwrap();
    let script =
      v8::Script::compile(scope, source, Some(&script_origin)).unwrap();
    source.to_rust_string_lossy(scope);
    let _result = script.run(scope).unwrap();
  }
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
  for (c1, c2) in chars.iter().copied().map(u16::from).zip(view) {
    assert_eq!(c1, c2);
  }
}

#[test]
fn inspector_string_buffer() {
  let chars = b"Hello Venus!";
  let mut buf = {
    let src_view = v8::inspector::StringView::from(&chars[..]);
    v8::inspector::StringBuffer::create(src_view)
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
  let _setup_guard = setup();
  let isolate = &mut v8::Isolate::new(Default::default());
  {
    let scope = &mut v8::HandleScope::new(isolate);
    let null = v8::null(scope);
    assert!(!null.is_undefined());
    assert!(null.is_null());
    assert!(null.is_null_or_undefined());

    let undefined = v8::undefined(scope);
    assert!(undefined.is_undefined());
    assert!(!undefined.is_null());
    assert!(undefined.is_null_or_undefined());

    let true_ = v8::Boolean::new(scope, true);
    assert!(true_.is_true());
    assert!(!true_.is_undefined());
    assert!(!true_.is_null());
    assert!(!true_.is_null_or_undefined());

    let false_ = v8::Boolean::new(scope, false);
    assert!(false_.is_false());
    assert!(!false_.is_undefined());
    assert!(!false_.is_null());
    assert!(!false_.is_null_or_undefined());
  }
}

#[test]
fn exception() {
  let _setup_guard = setup();
  let isolate = &mut v8::Isolate::new(Default::default());
  let scope = &mut v8::HandleScope::new(isolate);
  let context = v8::Context::new(scope);
  let scope = &mut v8::ContextScope::new(scope, context);

  let msg_in = v8::String::new(scope, "This is a test error").unwrap();
  let _exception = v8::Exception::error(scope, msg_in);
  let _exception = v8::Exception::range_error(scope, msg_in);
  let _exception = v8::Exception::reference_error(scope, msg_in);
  let _exception = v8::Exception::syntax_error(scope, msg_in);
  let exception = v8::Exception::type_error(scope, msg_in);

  let actual_msg_out =
    v8::Exception::create_message(scope, exception).get(scope);
  let expected_msg_out =
    v8::String::new(scope, "Uncaught TypeError: This is a test error").unwrap();
  assert!(actual_msg_out.strict_equals(expected_msg_out.into()));
  assert!(v8::Exception::get_stack_trace(scope, exception).is_none());
}

#[test]
fn create_message_argument_lifetimes() {
  let _setup_guard = setup();
  let isolate = &mut v8::Isolate::new(Default::default());
  let scope = &mut v8::HandleScope::new(isolate);
  let context = v8::Context::new(scope);
  let scope = &mut v8::ContextScope::new(scope, context);

  {
    let create_message = v8::Function::new(
      scope,
      |scope: &mut v8::HandleScope,
       args: v8::FunctionCallbackArguments,
       mut rv: v8::ReturnValue| {
        let message = v8::Exception::create_message(scope, args.get(0));
        let message_str = message.get(scope);
        rv.set(message_str.into())
      },
    )
    .unwrap();
    let receiver = context.global(scope);
    let message_str = v8::String::new(scope, "mishap").unwrap();
    let exception = v8::Exception::type_error(scope, message_str);
    let actual = create_message
      .call(scope, receiver.into(), &[exception])
      .unwrap();
    let expected =
      v8::String::new(scope, "Uncaught TypeError: mishap").unwrap();
    assert!(actual.strict_equals(expected.into()));
  }
}

#[test]
fn json() {
  let _setup_guard = setup();
  let isolate = &mut v8::Isolate::new(Default::default());
  {
    let scope = &mut v8::HandleScope::new(isolate);
    let context = v8::Context::new(scope);
    let scope = &mut v8::ContextScope::new(scope, context);
    let json_string = v8::String::new(scope, "{\"a\": 1, \"b\": 2}").unwrap();
    let maybe_value = v8::json::parse(scope, json_string);
    assert!(maybe_value.is_some());
    let value = maybe_value.unwrap();
    let maybe_stringified = v8::json::stringify(scope, value);
    assert!(maybe_stringified.is_some());
    let stringified = maybe_stringified.unwrap();
    let rust_str = stringified.to_rust_string_lossy(scope);
    assert_eq!("{\"a\":1,\"b\":2}".to_string(), rust_str);
  }
}

#[test]
fn no_internal_field() {
  let _setup_guard = setup();
  let isolate = &mut v8::Isolate::new(Default::default());
  {
    let scope = &mut v8::HandleScope::new(isolate);
    let context = v8::Context::new(scope);
    let scope = &mut v8::ContextScope::new(scope, context);
    let object = v8::Object::new(scope);
    let value = v8::Integer::new(scope, 42).into();
    assert_eq!(0, object.internal_field_count());
    for index in &[0, 1, 1337] {
      assert!(object.get_internal_field(scope, *index).is_none());
      assert!(!object.set_internal_field(*index, value));
      assert!(object.get_internal_field(scope, *index).is_none());
    }
  }
}

#[test]
fn object_template() {
  let _setup_guard = setup();
  let isolate = &mut v8::Isolate::new(Default::default());
  {
    let scope = &mut v8::HandleScope::new(isolate);
    let object_templ = v8::ObjectTemplate::new(scope);
    let function_templ = v8::FunctionTemplate::new(scope, fortytwo_callback);
    let name = v8::String::new(scope, "f").unwrap();
    let attr = v8::READ_ONLY + v8::DONT_ENUM + v8::DONT_DELETE;
    object_templ.set_internal_field_count(1);
    object_templ.set_with_attr(name.into(), function_templ.into(), attr);
    let context = v8::Context::new(scope);
    let scope = &mut v8::ContextScope::new(scope, context);

    let object = object_templ.new_instance(scope).unwrap();
    assert!(!object.is_null_or_undefined());
    assert_eq!(1, object.internal_field_count());

    let value = object.get_internal_field(scope, 0).unwrap();
    assert!(value.is_undefined());

    let fortytwo = v8::Integer::new(scope, 42).into();
    assert!(object.set_internal_field(0, fortytwo));
    let value = object.get_internal_field(scope, 0).unwrap();
    assert!(value.same_value(fortytwo));

    let name = v8::String::new(scope, "g").unwrap();
    context.global(scope).define_own_property(
      scope,
      name.into(),
      object.into(),
      v8::DONT_ENUM,
    );
    let source = r#"
      {
        const d = Object.getOwnPropertyDescriptor(globalThis, "g");
        [d.configurable, d.enumerable, d.writable].toString()
      }
    "#;
    let actual = eval(scope, source).unwrap();
    let expected = v8::String::new(scope, "true,false,true").unwrap();
    assert!(expected.strict_equals(actual));
    let actual = eval(scope, "g.f()").unwrap();
    let expected = v8::Integer::new(scope, 42);
    assert!(expected.strict_equals(actual));
    let source = r#"
      {
        const d = Object.getOwnPropertyDescriptor(g, "f");
        [d.configurable, d.enumerable, d.writable].toString()
      }
    "#;
    let actual = eval(scope, source).unwrap();
    let expected = v8::String::new(scope, "false,false,false").unwrap();
    assert!(expected.strict_equals(actual));
  }
}

#[test]
fn object_template_from_function_template() {
  let _setup_guard = setup();
  let isolate = &mut v8::Isolate::new(Default::default());
  {
    let scope = &mut v8::HandleScope::new(isolate);
    let function_templ = v8::FunctionTemplate::new(scope, fortytwo_callback);
    let expected_class_name = v8::String::new(scope, "fortytwo").unwrap();
    function_templ.set_class_name(expected_class_name);
    let object_templ =
      v8::ObjectTemplate::new_from_template(scope, function_templ);
    assert_eq!(0, object_templ.internal_field_count());
    let context = v8::Context::new(scope);
    let scope = &mut v8::ContextScope::new(scope, context);
    let object = object_templ.new_instance(scope).unwrap();
    assert!(!object.is_null_or_undefined());
    let name = v8::String::new(scope, "g").unwrap();
    context.global(scope).set(scope, name.into(), object.into());
    let actual_class_name = eval(scope, "g.constructor.name").unwrap();
    assert!(expected_class_name.strict_equals(actual_class_name));
  }
}

#[test]
fn function_template_signature() {
  let _setup_guard = setup();
  let isolate = &mut v8::Isolate::new(Default::default());
  {
    let scope = &mut v8::HandleScope::new(isolate);

    let templ0 = v8::FunctionTemplate::new(scope, fortytwo_callback);
    let signature = v8::Signature::new(scope, templ0);
    let templ1 = v8::FunctionTemplate::builder(fortytwo_callback)
      .signature(signature)
      .build(scope);

    let context = v8::Context::new(scope);
    let scope = &mut v8::ContextScope::new(scope, context);
    let scope = &mut v8::TryCatch::new(scope);
    let global = context.global(scope);

    let name = v8::String::new(scope, "C").unwrap();
    let value = templ0.get_function(scope).unwrap();
    global.set(scope, name.into(), value.into()).unwrap();

    let name = v8::String::new(scope, "f").unwrap();
    let value = templ1.get_function(scope).unwrap();
    global.set(scope, name.into(), value.into()).unwrap();

    assert!(eval(scope, "f.call(new C)").is_some());
    assert!(eval(scope, "f.call(new Object)").is_none());
    assert!(scope.has_caught());
    assert!(scope
      .exception()
      .unwrap()
      .to_rust_string_lossy(scope)
      .contains("Illegal invocation"));
  }
}

#[test]
fn function_template_prototype() {
  let _setup_guard = setup();
  let isolate = &mut v8::Isolate::new(Default::default());
  {
    let scope = &mut v8::HandleScope::new(isolate);

    let context = v8::Context::new(scope);
    let scope = &mut v8::ContextScope::new(scope, context);
    let scope = &mut v8::TryCatch::new(scope);
    let function_templ = v8::FunctionTemplate::new(scope, fortytwo_callback);
    let prototype_templ = function_templ.prototype_template(scope);

    let amount_name = v8::String::new(scope, "amount").unwrap();
    let value = v8::Number::new(scope, 1.0);
    let second_value = v8::Number::new(scope, 2.0);
    let third_value = v8::Number::new(scope, 3.0);
    prototype_templ.set(amount_name.into(), value.into());

    let function = function_templ.get_function(scope).unwrap();
    function.new_instance(scope, &[]);

    let object1 = function.new_instance(scope, &[]).unwrap();
    assert!(!object1.is_null_or_undefined());
    let name = v8::String::new(scope, "ob1").unwrap();
    context
      .global(scope)
      .set(scope, name.into(), object1.into());

    let actual_amount =
      eval(scope, "ob1.amount").unwrap().to_number(scope).unwrap();
    dbg!("{}", actual_amount.number_value(scope).unwrap());
    assert!(value.eq(&actual_amount));

    let object2 = function.new_instance(scope, &[]).unwrap();
    assert!(!object2.is_null_or_undefined());
    let name = v8::String::new(scope, "ob2").unwrap();
    context
      .global(scope)
      .set(scope, name.into(), object2.into());

    let actual_amount =
      eval(scope, "ob2.amount").unwrap().to_number(scope).unwrap();
    dbg!("{}", actual_amount.number_value(scope).unwrap());
    assert!(value.eq(&actual_amount));

    eval(scope, "ob1.amount = 2").unwrap();

    let actual_amount =
      eval(scope, "ob1.amount").unwrap().to_number(scope).unwrap();
    dbg!("{}", actual_amount.number_value(scope).unwrap());
    assert!(second_value.eq(&actual_amount));

    // We need to get the prototype of the object to change it, it is not the same object as the prototype template!
    object2
      .get_prototype(scope)
      .unwrap()
      .to_object(scope)
      .unwrap()
      .set(scope, amount_name.into(), third_value.into());

    let actual_amount =
      eval(scope, "ob1.amount").unwrap().to_number(scope).unwrap();
    dbg!("{}", actual_amount.number_value(scope).unwrap());
    assert!(second_value.eq(&actual_amount));

    let actual_amount =
      eval(scope, "ob2.amount").unwrap().to_number(scope).unwrap();
    dbg!("{}", actual_amount.number_value(scope).unwrap());
    assert!(third_value.eq(&actual_amount));
  }
}

#[test]
fn object_template_set_accessor() {
  let _setup_guard = setup();
  let isolate = &mut v8::Isolate::new(Default::default());
  let scope = &mut v8::HandleScope::new(isolate);
  let context = v8::Context::new(scope);
  let scope = &mut v8::ContextScope::new(scope, context);

  {
    let getter = |scope: &mut v8::HandleScope,
                  key: v8::Local<v8::Name>,
                  args: v8::PropertyCallbackArguments,
                  mut rv: v8::ReturnValue| {
      let this = args.this();

      let expected_key = v8::String::new(scope, "key").unwrap();
      assert!(key.strict_equals(expected_key.into()));

      rv.set(this.get_internal_field(scope, 0).unwrap());
    };

    let setter = |scope: &mut v8::HandleScope,
                  key: v8::Local<v8::Name>,
                  value: v8::Local<v8::Value>,
                  args: v8::PropertyCallbackArguments| {
      let this = args.this();

      let expected_key = v8::String::new(scope, "key").unwrap();
      assert!(key.strict_equals(expected_key.into()));

      assert!(value.is_int32());
      assert!(this.set_internal_field(0, value));
    };

    let key = v8::String::new(scope, "key").unwrap();
    let name = v8::String::new(scope, "obj").unwrap();

    // Lone getter
    let templ = v8::ObjectTemplate::new(scope);
    templ.set_internal_field_count(1);
    templ.set_accessor(key.into(), getter);

    let obj = templ.new_instance(scope).unwrap();
    let int = v8::Integer::new(scope, 42);
    obj.set_internal_field(0, int.into());
    scope.get_current_context().global(scope).set(
      scope,
      name.into(),
      obj.into(),
    );
    assert!(eval(scope, "obj.key").unwrap().strict_equals(int.into()));

    // Getter + setter
    let templ = v8::ObjectTemplate::new(scope);
    templ.set_internal_field_count(1);
    templ.set_accessor_with_setter(key.into(), getter, setter);

    let obj = templ.new_instance(scope).unwrap();
    obj.set_internal_field(0, int.into());
    scope.get_current_context().global(scope).set(
      scope,
      name.into(),
      obj.into(),
    );
    let new_int = v8::Integer::new(scope, 9);
    eval(scope, "obj.key = 9");
    assert!(obj
      .get_internal_field(scope, 0)
      .unwrap()
      .strict_equals(new_int.into()));
    // Falls back on standard setter
    assert!(eval(scope, "obj.key2 = null; obj.key2").unwrap().is_null());
  }
}

#[test]
fn object() {
  let _setup_guard = setup();
  let isolate = &mut v8::Isolate::new(Default::default());
  {
    let scope = &mut v8::HandleScope::new(isolate);
    let context = v8::Context::new(scope);
    let scope = &mut v8::ContextScope::new(scope, context);
    let null: v8::Local<v8::Value> = v8::null(scope).into();
    let n1: v8::Local<v8::Name> = v8::String::new(scope, "a").unwrap().into();
    let n2: v8::Local<v8::Name> = v8::String::new(scope, "b").unwrap().into();
    let v1: v8::Local<v8::Value> = v8::Number::new(scope, 1.0).into();
    let v2: v8::Local<v8::Value> = v8::Number::new(scope, 2.0).into();
    let object = v8::Object::with_prototype_and_properties(
      scope,
      null,
      &[n1, n2],
      &[v1, v2],
    );
    assert!(!object.is_null_or_undefined());
    let lhs = object.creation_context(scope).global(scope);
    let rhs = context.global(scope);
    assert!(lhs.strict_equals(rhs.into()));

    let object_ = v8::Object::new(scope);
    assert!(!object_.is_null_or_undefined());
    let id = object_.get_identity_hash();
    assert_ne!(id, 0);

    assert!(object.has(scope, n1.into()).unwrap());
    let n_unused = v8::String::new(scope, "unused").unwrap().into();
    assert!(!object.has(scope, n_unused).unwrap());
    assert!(object.delete(scope, n1.into()).unwrap());
    assert!(!object.has(scope, n1.into()).unwrap());
  }
}

#[test]
fn array() {
  let _setup_guard = setup();
  let isolate = &mut v8::Isolate::new(Default::default());
  {
    let scope = &mut v8::HandleScope::new(isolate);
    let context = v8::Context::new(scope);
    let scope = &mut v8::ContextScope::new(scope, context);
    let s1 = v8::String::new(scope, "a").unwrap();
    let s2 = v8::String::new(scope, "b").unwrap();
    let array = v8::Array::new(scope, 2);
    assert_eq!(array.length(), 2);
    let lhs = array.creation_context(scope).global(scope);
    let rhs = context.global(scope);
    assert!(lhs.strict_equals(rhs.into()));
    array.set_index(scope, 0, s1.into());
    array.set_index(scope, 1, s2.into());

    let maybe_v1 = array.get_index(scope, 0);
    assert!(maybe_v1.is_some());
    assert!(maybe_v1.unwrap().same_value(s1.into()));
    let maybe_v2 = array.get_index(scope, 1);
    assert!(maybe_v2.is_some());
    assert!(maybe_v2.unwrap().same_value(s2.into()));

    let array = v8::Array::new_with_elements(scope, &[]);
    assert_eq!(array.length(), 0);

    let array = v8::Array::new_with_elements(scope, &[s1.into(), s2.into()]);
    assert_eq!(array.length(), 2);

    let maybe_v1 = array.get_index(scope, 0);
    assert!(maybe_v1.is_some());
    assert!(maybe_v1.unwrap().same_value(s1.into()));
    let maybe_v2 = array.get_index(scope, 1);
    assert!(maybe_v2.is_some());
    assert!(maybe_v2.unwrap().same_value(s2.into()));

    assert!(array.has_index(scope, 1).unwrap());
    assert!(array.delete_index(scope, 1).unwrap());
    assert!(!array.has_index(scope, 1).unwrap());
  }
}

#[test]
fn create_data_property() {
  let _setup_guard = setup();
  let isolate = &mut v8::Isolate::new(Default::default());
  {
    let scope = &mut v8::HandleScope::new(isolate);
    let context = v8::Context::new(scope);
    let scope = &mut v8::ContextScope::new(scope, context);

    eval(scope, "var a = {};");

    let key = v8::String::new(scope, "a").unwrap();
    let obj = context.global(scope).get(scope, key.into()).unwrap();
    assert!(obj.is_object());
    let obj = obj.to_object(scope).unwrap();
    let key = v8::String::new(scope, "foo").unwrap();
    let value = v8::String::new(scope, "bar").unwrap();
    assert!(obj
      .create_data_property(scope, key.into(), value.into())
      .unwrap());
    let actual = obj.get(scope, key.into()).unwrap();
    assert!(value.strict_equals(actual));

    let key2 = v8::String::new(scope, "foo2").unwrap();
    assert!(obj.set(scope, key2.into(), value.into()).unwrap());
    let actual = obj.get(scope, key2.into()).unwrap();
    assert!(value.strict_equals(actual));
  }
}

#[test]
fn object_set_accessor() {
  let _setup_guard = setup();
  let isolate = &mut v8::Isolate::new(Default::default());
  let scope = &mut v8::HandleScope::new(isolate);
  let context = v8::Context::new(scope);
  let scope = &mut v8::ContextScope::new(scope, context);

  {
    static CALL_COUNT: AtomicUsize = AtomicUsize::new(0);

    let getter = |scope: &mut v8::HandleScope,
                  key: v8::Local<v8::Name>,
                  args: v8::PropertyCallbackArguments,
                  mut rv: v8::ReturnValue| {
      let this = args.this();

      let expected_key = v8::String::new(scope, "getter_key").unwrap();
      assert!(key.strict_equals(expected_key.into()));

      let int_key = v8::String::new(scope, "int_key").unwrap();
      let int_value = this.get(scope, int_key.into()).unwrap();
      let int_value = v8::Local::<v8::Integer>::try_from(int_value).unwrap();
      assert_eq!(int_value.value(), 42);

      let s = v8::String::new(scope, "hello").unwrap();
      assert!(rv.get(scope).is_undefined());
      rv.set(s.into());

      CALL_COUNT.fetch_add(1, Ordering::SeqCst);
    };

    let obj = v8::Object::new(scope);

    let getter_key = v8::String::new(scope, "getter_key").unwrap();
    obj.set_accessor(scope, getter_key.into(), getter);

    let int_key = v8::String::new(scope, "int_key").unwrap();
    let int_value = v8::Integer::new(scope, 42);
    obj.set(scope, int_key.into(), int_value.into());

    let obj_name = v8::String::new(scope, "obj").unwrap();
    context
      .global(scope)
      .set(scope, obj_name.into(), obj.into());

    let actual = eval(scope, "obj.getter_key").unwrap();
    let expected = v8::String::new(scope, "hello").unwrap();
    assert!(actual.strict_equals(expected.into()));

    assert_eq!(CALL_COUNT.load(Ordering::SeqCst), 1);
  }
}

#[test]
fn object_set_accessor_with_setter() {
  let _setup_guard = setup();
  let isolate = &mut v8::Isolate::new(Default::default());
  let scope = &mut v8::HandleScope::new(isolate);
  let context = v8::Context::new(scope);
  let scope = &mut v8::ContextScope::new(scope, context);

  {
    static CALL_COUNT: AtomicUsize = AtomicUsize::new(0);

    let getter = |scope: &mut v8::HandleScope,
                  key: v8::Local<v8::Name>,
                  args: v8::PropertyCallbackArguments,
                  mut rv: v8::ReturnValue| {
      let this = args.this();

      let expected_key = v8::String::new(scope, "getter_setter_key").unwrap();
      assert!(key.strict_equals(expected_key.into()));

      let int_key = v8::String::new(scope, "int_key").unwrap();
      let int_value = this.get(scope, int_key.into()).unwrap();
      let int_value = v8::Local::<v8::Integer>::try_from(int_value).unwrap();
      assert_eq!(int_value.value(), 42);

      let s = v8::String::new(scope, "hello").unwrap();
      assert!(rv.get(scope).is_undefined());
      rv.set(s.into());

      CALL_COUNT.fetch_add(1, Ordering::SeqCst);
    };

    let setter = |scope: &mut v8::HandleScope,
                  key: v8::Local<v8::Name>,
                  value: v8::Local<v8::Value>,
                  args: v8::PropertyCallbackArguments| {
      println!("setter called");
      let this = args.this();

      let expected_key = v8::String::new(scope, "getter_setter_key").unwrap();
      assert!(key.strict_equals(expected_key.into()));

      let int_key = v8::String::new(scope, "int_key").unwrap();
      let int_value = this.get(scope, int_key.into()).unwrap();
      let int_value = v8::Local::<v8::Integer>::try_from(int_value).unwrap();
      assert_eq!(int_value.value(), 42);

      let new_value = v8::Local::<v8::Integer>::try_from(value).unwrap();
      this.set(scope, int_key.into(), new_value.into());

      CALL_COUNT.fetch_add(1, Ordering::SeqCst);
    };

    let obj = v8::Object::new(scope);

    let getter_setter_key =
      v8::String::new(scope, "getter_setter_key").unwrap();
    obj.set_accessor_with_setter(
      scope,
      getter_setter_key.into(),
      getter,
      setter,
    );

    let int_key = v8::String::new(scope, "int_key").unwrap();
    let int_value = v8::Integer::new(scope, 42);
    obj.set(scope, int_key.into(), int_value.into());

    let obj_name = v8::String::new(scope, "obj").unwrap();
    context
      .global(scope)
      .set(scope, obj_name.into(), obj.into());

    let actual = eval(scope, "obj.getter_setter_key").unwrap();
    let expected = v8::String::new(scope, "hello").unwrap();
    assert!(actual.strict_equals(expected.into()));

    eval(scope, "obj.getter_setter_key = 123").unwrap();
    assert_eq!(
      obj
        .get(scope, int_key.into())
        .unwrap()
        .to_integer(scope)
        .unwrap()
        .value(),
      123
    );

    assert_eq!(CALL_COUNT.load(Ordering::SeqCst), 2);
  }
}

#[test]
fn promise_resolved() {
  let _setup_guard = setup();
  let isolate = &mut v8::Isolate::new(Default::default());
  {
    let scope = &mut v8::HandleScope::new(isolate);
    let context = v8::Context::new(scope);
    let scope = &mut v8::ContextScope::new(scope, context);
    let maybe_resolver = v8::PromiseResolver::new(scope);
    assert!(maybe_resolver.is_some());
    let resolver = maybe_resolver.unwrap();
    let promise = resolver.get_promise(scope);
    assert!(!promise.has_handler());
    assert_eq!(promise.state(), v8::PromiseState::Pending);
    let value = v8::String::new(scope, "test").unwrap();
    resolver.resolve(scope, value.into());
    assert_eq!(promise.state(), v8::PromiseState::Fulfilled);
    let result = promise.result(scope);
    assert_eq!(result.to_rust_string_lossy(scope), "test".to_string());
    // Resolve again with different value, since promise is already in
    // `Fulfilled` state it should be ignored.
    let value = v8::String::new(scope, "test2").unwrap();
    resolver.resolve(scope, value.into());
    let result = promise.result(scope);
    assert_eq!(result.to_rust_string_lossy(scope), "test".to_string());
  }
}

#[test]
fn promise_rejected() {
  let _setup_guard = setup();
  let isolate = &mut v8::Isolate::new(Default::default());
  {
    let scope = &mut v8::HandleScope::new(isolate);
    let context = v8::Context::new(scope);
    let scope = &mut v8::ContextScope::new(scope, context);
    let maybe_resolver = v8::PromiseResolver::new(scope);
    assert!(maybe_resolver.is_some());
    let resolver = maybe_resolver.unwrap();
    let promise = resolver.get_promise(scope);
    assert!(!promise.has_handler());
    assert_eq!(promise.state(), v8::PromiseState::Pending);
    let value = v8::String::new(scope, "test").unwrap();
    let rejected = resolver.reject(scope, value.into());
    assert!(rejected.unwrap());
    assert_eq!(promise.state(), v8::PromiseState::Rejected);
    let result = promise.result(scope);
    assert_eq!(result.to_rust_string_lossy(scope), "test".to_string());
    // Reject again with different value, since promise is already in `Rejected`
    // state it should be ignored.
    let value = v8::String::new(scope, "test2").unwrap();
    resolver.reject(scope, value.into());
    let result = promise.result(scope);
    assert_eq!(result.to_rust_string_lossy(scope), "test".to_string());
  }
}
#[test]
fn proxy() {
  let _setup_guard = setup();
  let isolate = &mut v8::Isolate::new(Default::default());
  {
    let scope = &mut v8::HandleScope::new(isolate);
    let context = v8::Context::new(scope);
    let scope = &mut v8::ContextScope::new(scope, context);
    let target = v8::Object::new(scope);
    let handler = v8::Object::new(scope);
    let maybe_proxy = v8::Proxy::new(scope, target, handler);
    assert!(maybe_proxy.is_some());
    let proxy = maybe_proxy.unwrap();
    assert!(target == proxy.get_target(scope));
    assert!(handler == proxy.get_handler(scope));
    assert!(!proxy.is_revoked());
    proxy.revoke();
    assert!(proxy.is_revoked());
  }
}

fn fn_callback(
  scope: &mut v8::HandleScope,
  args: v8::FunctionCallbackArguments,
  mut rv: v8::ReturnValue,
) {
  assert_eq!(args.length(), 0);
  let s = v8::String::new(scope, "Hello callback!").unwrap();
  assert!(rv.get(scope).is_undefined());
  rv.set(s.into());
}

fn fn_callback2(
  scope: &mut v8::HandleScope,
  args: v8::FunctionCallbackArguments,
  mut rv: v8::ReturnValue,
) {
  assert_eq!(args.length(), 2);
  let arg1_val = v8::String::new(scope, "arg1").unwrap();
  let arg1 = args.get(0);
  assert!(arg1.is_string());
  assert!(arg1.strict_equals(arg1_val.into()));

  let arg2_val = v8::Integer::new(scope, 2);
  let arg2 = args.get(1);
  assert!(arg2.is_number());
  assert!(arg2.strict_equals(arg2_val.into()));

  let s = v8::String::new(scope, "Hello callback!").unwrap();
  assert!(rv.get(scope).is_undefined());
  rv.set(s.into());
}

fn fortytwo_callback(
  scope: &mut v8::HandleScope,
  _: v8::FunctionCallbackArguments,
  mut rv: v8::ReturnValue,
) {
  rv.set(v8::Integer::new(scope, 42).into());
}

fn data_is_true_callback(
  _scope: &mut v8::HandleScope,
  args: v8::FunctionCallbackArguments,
  _rv: v8::ReturnValue,
) {
  let data = args.data();
  assert!(data.is_some());
  let data = data.unwrap();
  assert!(data.is_true());
}

#[test]
fn function() {
  let _setup_guard = setup();
  let isolate = &mut v8::Isolate::new(Default::default());

  {
    let scope = &mut v8::HandleScope::new(isolate);
    let context = v8::Context::new(scope);
    let scope = &mut v8::ContextScope::new(scope, context);
    let global = context.global(scope);
    let recv: v8::Local<v8::Value> = global.into();

    // create function using template
    let fn_template = v8::FunctionTemplate::new(scope, fn_callback);
    let function = fn_template
      .get_function(scope)
      .expect("Unable to create function");
    let lhs = function.creation_context(scope).global(scope);
    let rhs = context.global(scope);
    assert!(lhs.strict_equals(rhs.into()));
    let value = function
      .call(scope, recv, &[])
      .expect("Function call failed");
    let value_str = value.to_string(scope).unwrap();
    let rust_str = value_str.to_rust_string_lossy(scope);
    assert_eq!(rust_str, "Hello callback!".to_string());

    // create function without a template
    let function = v8::Function::new(scope, fn_callback2)
      .expect("Unable to create function");
    let arg1 = v8::String::new(scope, "arg1").unwrap();
    let arg2 = v8::Integer::new(scope, 2);
    let value = function
      .call(scope, recv, &[arg1.into(), arg2.into()])
      .unwrap();
    let value_str = value.to_string(scope).unwrap();
    let rust_str = value_str.to_rust_string_lossy(scope);
    assert_eq!(rust_str, "Hello callback!".to_string());

    // create a function with associated data
    let true_data = v8::Boolean::new(scope, true);
    let function = v8::Function::builder(data_is_true_callback)
      .data(true_data.into())
      .build(scope)
      .expect("Unable to create function with data");
    let value = function
      .call(scope, recv, &[])
      .expect("Function call failed");
    assert!(value.is_undefined());

    // create a prototype-less function that throws on new
    let function = v8::Function::builder(fn_callback)
      .length(42)
      .constructor_behavior(v8::ConstructorBehavior::Throw)
      .build(scope)
      .unwrap();
    let name = v8::String::new(scope, "f").unwrap();
    global.set(scope, name.into(), function.into()).unwrap();
    let result = eval(scope, "f.length").unwrap();
    assert_eq!(42, result.integer_value(scope).unwrap());
    let result = eval(scope, "f.prototype").unwrap();
    assert!(result.is_undefined());
    assert!(eval(scope, "new f()").is_none()); // throws
  }
}

#[test]
fn constructor() {
  let _setup_guard = setup();
  let isolate = &mut v8::Isolate::new(Default::default());

  {
    let scope = &mut v8::HandleScope::new(isolate);
    let context = v8::Context::new(scope);
    let scope = &mut v8::ContextScope::new(scope, context);
    let global = context.global(scope);
    let array_name = v8::String::new(scope, "Array").unwrap();
    let array_constructor = global.get(scope, array_name.into()).unwrap();
    let array_constructor =
      v8::Local::<v8::Function>::try_from(array_constructor).unwrap();
    let array = array_constructor.new_instance(scope, &[]).unwrap();
    v8::Local::<v8::Array>::try_from(array).unwrap();
  }
}

extern "C" fn promise_reject_callback(msg: v8::PromiseRejectMessage) {
  let scope = &mut unsafe { v8::CallbackScope::new(&msg) };
  let event = msg.get_event();
  assert_eq!(event, v8::PromiseRejectEvent::PromiseRejectWithNoHandler);
  let promise = msg.get_promise();
  assert_eq!(promise.state(), v8::PromiseState::Rejected);
  let value = msg.get_value().unwrap();
  {
    let scope = &mut v8::HandleScope::new(scope);
    let value_str = value.to_rust_string_lossy(scope);
    assert_eq!(value_str, "promise rejected".to_string());
  }
}

#[test]
fn set_promise_reject_callback() {
  let _setup_guard = setup();
  let isolate = &mut v8::Isolate::new(Default::default());
  isolate.set_promise_reject_callback(promise_reject_callback);
  {
    let scope = &mut v8::HandleScope::new(isolate);
    let context = v8::Context::new(scope);
    let scope = &mut v8::ContextScope::new(scope, context);
    let resolver = v8::PromiseResolver::new(scope).unwrap();
    let value = v8::String::new(scope, "promise rejected").unwrap();
    resolver.reject(scope, value.into());
  }
}

#[test]
fn promise_reject_callback_no_value() {
  extern "C" fn promise_reject_callback(m: v8::PromiseRejectMessage) {
    use v8::PromiseRejectEvent::*;
    let value = m.get_value();
    match m.get_event() {
      PromiseHandlerAddedAfterReject => assert!(value.is_none()),
      PromiseRejectWithNoHandler => assert!(value.is_some()),
      _ => unreachable!(),
    };
  }
  let _setup_guard = setup();
  let isolate = &mut v8::Isolate::new(Default::default());
  isolate.set_promise_reject_callback(promise_reject_callback);
  {
    let scope = &mut v8::HandleScope::new(isolate);
    let context = v8::Context::new(scope);
    let scope = &mut v8::ContextScope::new(scope, context);
    let source = r#"
      function kaboom(resolve, reject) {
        throw new Error("kaboom");
      }
      new Promise(kaboom).then(_ => {});
    "#;
    eval(scope, source).unwrap();
  }
}

#[test]
fn promise_hook() {
  extern "C" fn hook(
    type_: v8::PromiseHookType,
    promise: v8::Local<v8::Promise>,
    _parent: v8::Local<v8::Value>,
  ) {
    // Check that PromiseHookType implements Clone and PartialEq.
    #[allow(clippy::clone_on_copy)]
    if type_.clone() == v8::PromiseHookType::Init {}
    let scope = &mut unsafe { v8::CallbackScope::new(promise) };
    let context = promise.creation_context(scope);
    let scope = &mut v8::ContextScope::new(scope, context);
    let global = context.global(scope);
    let name = v8::String::new(scope, "hook").unwrap();
    let func = global.get(scope, name.into()).unwrap();
    let func = v8::Local::<v8::Function>::try_from(func).unwrap();
    let args = &[v8::Integer::new(scope, type_ as i32).into(), promise.into()];
    func.call(scope, global.into(), args).unwrap();
  }
  let _setup_guard = setup();
  let isolate = &mut v8::Isolate::new(Default::default());
  isolate.set_promise_hook(hook);
  {
    let scope = &mut v8::HandleScope::new(isolate);
    let context = v8::Context::new(scope);
    let scope = &mut v8::ContextScope::new(scope, context);
    let source = r#"
      var promises = new Set();
      function hook(type, promise) {
        if (type === /* Init    */ 0) promises.add(promise);
        if (type === /* Resolve */ 1) promises.delete(promise);
      }
      function expect(expected, actual = promises.size) {
        if (actual !== expected) throw `expected ${expected}, actual ${actual}`;
      }
      expect(0);
      new Promise(resolve => {
        expect(1);
        resolve();
        expect(0);
      });
      expect(0);
      new Promise(() => {});
      expect(1);
      promises.values().next().value
    "#;
    let promise = eval(scope, source).unwrap();
    let promise = v8::Local::<v8::Promise>::try_from(promise).unwrap();
    assert!(!promise.has_handler());
    assert_eq!(promise.state(), v8::PromiseState::Pending);
  }
}

#[test]
fn allow_atomics_wait() {
  let _setup_guard = setup();
  let isolate = &mut v8::Isolate::new(Default::default());
  for allow in &[false, true, false] {
    let allow = *allow;
    isolate.set_allow_atomics_wait(allow);
    {
      let scope = &mut v8::HandleScope::new(isolate);
      let context = v8::Context::new(scope);
      let scope = &mut v8::ContextScope::new(scope, context);
      let source = r#"
        const b = new SharedArrayBuffer(4);
        const a = new Int32Array(b);
        "timed-out" === Atomics.wait(a, 0, 0, 1);
      "#;
      let try_catch = &mut v8::TryCatch::new(scope);
      let result = eval(try_catch, source);
      if allow {
        assert!(!try_catch.has_caught());
        assert!(result.unwrap().is_true());
      } else {
        assert!(try_catch.has_caught());
        let exc = try_catch.exception().unwrap();
        let exc = exc.to_string(try_catch).unwrap();
        let exc = exc.to_rust_string_lossy(try_catch);
        assert!(exc.contains("Atomics.wait cannot be called in this context"));
      }
    }
  }
}

fn mock_script_origin<'s>(
  scope: &mut v8::HandleScope<'s>,
  resource_name_: &str,
) -> v8::ScriptOrigin<'s> {
  let resource_name = v8::String::new(scope, resource_name_).unwrap();
  let resource_line_offset = 0;
  let resource_column_offset = 0;
  let resource_is_shared_cross_origin = true;
  let script_id = 123;
  let source_map_url = v8::String::new(scope, "source_map_url").unwrap();
  let resource_is_opaque = true;
  let is_wasm = false;
  let is_module = true;
  v8::ScriptOrigin::new(
    scope,
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

fn mock_source<'s>(
  scope: &mut v8::HandleScope<'s>,
  resource_name: &str,
  source: &str,
) -> v8::script_compiler::Source {
  let source_str = v8::String::new(scope, source).unwrap();
  let script_origin = mock_script_origin(scope, resource_name);
  v8::script_compiler::Source::new(source_str, Some(&script_origin))
}

#[test]
fn script_compiler_source() {
  let _setup_guard = setup();
  let isolate = &mut v8::Isolate::new(Default::default());
  isolate.set_promise_reject_callback(promise_reject_callback);
  {
    let scope = &mut v8::HandleScope::new(isolate);
    let context = v8::Context::new(scope);
    let scope = &mut v8::ContextScope::new(scope, context);

    let source = "1+2";
    let script_origin = mock_script_origin(scope, "foo.js");
    let source = v8::script_compiler::Source::new(
      v8::String::new(scope, source).unwrap(),
      Some(&script_origin),
    );

    let result = v8::script_compiler::compile_module(scope, source);
    assert!(result.is_some());
  }
}

#[test]
fn module_instantiation_failures1() {
  let _setup_guard = setup();
  let isolate = &mut v8::Isolate::new(Default::default());
  {
    let scope = &mut v8::HandleScope::new(isolate);
    let context = v8::Context::new(scope);
    let scope = &mut v8::ContextScope::new(scope, context);

    let source_text = v8::String::new(
      scope,
      "import './foo.js';\n\
       export {} from './bar.js';",
    )
    .unwrap();
    let origin = mock_script_origin(scope, "foo.js");
    let source = v8::script_compiler::Source::new(source_text, Some(&origin));

    let module = v8::script_compiler::compile_module(scope, source).unwrap();
    assert_eq!(v8::ModuleStatus::Uninstantiated, module.get_status());
    let module_requests = module.get_module_requests();
    assert_eq!(2, module_requests.length());
    assert!(module.script_id().is_some());

    let mr1 = v8::Local::<v8::ModuleRequest>::try_from(
      module_requests.get(scope, 0).unwrap(),
    )
    .unwrap();
    assert_eq!("./foo.js", mr1.get_specifier().to_rust_string_lossy(scope));
    let loc = module.source_offset_to_location(mr1.get_source_offset());
    assert_eq!(0, loc.get_line_number());
    assert_eq!(7, loc.get_column_number());
    assert_eq!(0, mr1.get_import_assertions().length());

    let mr2 = v8::Local::<v8::ModuleRequest>::try_from(
      module_requests.get(scope, 1).unwrap(),
    )
    .unwrap();
    assert_eq!("./bar.js", mr2.get_specifier().to_rust_string_lossy(scope));
    let loc = module.source_offset_to_location(mr2.get_source_offset());
    assert_eq!(1, loc.get_line_number());
    assert_eq!(15, loc.get_column_number());
    assert_eq!(0, mr2.get_import_assertions().length());

    // Instantiation should fail.
    {
      let tc = &mut v8::TryCatch::new(scope);
      fn resolve_callback<'a>(
        context: v8::Local<'a, v8::Context>,
        _specifier: v8::Local<'a, v8::String>,
        _import_assertions: v8::Local<'a, v8::FixedArray>,
        _referrer: v8::Local<'a, v8::Module>,
      ) -> Option<v8::Local<'a, v8::Module>> {
        let scope = &mut unsafe { v8::CallbackScope::new(context) };
        let scope = &mut v8::HandleScope::new(scope);
        let e = v8::String::new(scope, "boom").unwrap();
        scope.throw_exception(e.into());
        None
      }
      let result = module.instantiate_module(tc, resolve_callback);
      assert!(result.is_none());
      assert!(tc.has_caught());
      assert!(tc
        .exception()
        .unwrap()
        .strict_equals(v8::String::new(tc, "boom").unwrap().into()));
      assert_eq!(v8::ModuleStatus::Uninstantiated, module.get_status());
    }
  }
}

// Clippy thinks the return value doesn't need to be an Option, it's unaware
// of the mapping that MapFnFrom<F> does for ResolveModuleCallback.
#[allow(clippy::unnecessary_wraps)]
fn compile_specifier_as_module_resolve_callback<'a>(
  context: v8::Local<'a, v8::Context>,
  specifier: v8::Local<'a, v8::String>,
  _import_assertions: v8::Local<'a, v8::FixedArray>,
  _referrer: v8::Local<'a, v8::Module>,
) -> Option<v8::Local<'a, v8::Module>> {
  let scope = &mut unsafe { v8::CallbackScope::new(context) };
  let origin = mock_script_origin(scope, "module.js");
  let source = v8::script_compiler::Source::new(specifier, Some(&origin));
  let module = v8::script_compiler::compile_module(scope, source).unwrap();
  Some(module)
}

#[test]
fn module_evaluation() {
  let _setup_guard = setup();
  let isolate = &mut v8::Isolate::new(Default::default());
  {
    let scope = &mut v8::HandleScope::new(isolate);
    let context = v8::Context::new(scope);
    let scope = &mut v8::ContextScope::new(scope, context);

    let source_text = v8::String::new(
      scope,
      "import 'Object.expando = 5';\n\
       import 'Object.expando *= 2';",
    )
    .unwrap();
    let origin = mock_script_origin(scope, "foo.js");
    let source = v8::script_compiler::Source::new(source_text, Some(&origin));

    let module = v8::script_compiler::compile_module(scope, source).unwrap();
    assert!(module.script_id().is_some());
    assert!(module.is_source_text_module());
    assert!(!module.is_synthetic_module());
    assert_eq!(v8::ModuleStatus::Uninstantiated, module.get_status());
    module.hash(&mut DefaultHasher::new()); // Should not crash.

    let result = module
      .instantiate_module(scope, compile_specifier_as_module_resolve_callback);
    assert!(result.unwrap());
    assert_eq!(v8::ModuleStatus::Instantiated, module.get_status());

    let result = module.evaluate(scope);
    assert!(result.is_some());
    assert_eq!(v8::ModuleStatus::Evaluated, module.get_status());

    let result = eval(scope, "Object.expando").unwrap();
    assert!(result.is_number());
    let expected = v8::Number::new(scope, 10.);
    assert!(result.strict_equals(expected.into()));
  }
}

#[test]
fn import_assertions() {
  let _setup_guard = setup();
  let isolate = &mut v8::Isolate::new(Default::default());

  // Clippy thinks the return value doesn't need to be an Option, it's unaware
  // of the mapping that MapFnFrom<F> does for ResolveModuleCallback.
  #[allow(clippy::unnecessary_wraps)]
  fn module_resolve_callback<'a>(
    context: v8::Local<'a, v8::Context>,
    _specifier: v8::Local<'a, v8::String>,
    import_assertions: v8::Local<'a, v8::FixedArray>,
    _referrer: v8::Local<'a, v8::Module>,
  ) -> Option<v8::Local<'a, v8::Module>> {
    let scope = &mut unsafe { v8::CallbackScope::new(context) };

    // "type" keyword, value and source offset of assertion
    assert_eq!(import_assertions.length(), 3);
    let assert1 = import_assertions.get(scope, 0).unwrap();
    let assert1_val = v8::Local::<v8::Value>::try_from(assert1).unwrap();
    assert_eq!(assert1_val.to_rust_string_lossy(scope), "type");
    let assert2 = import_assertions.get(scope, 1).unwrap();
    let assert2_val = v8::Local::<v8::Value>::try_from(assert2).unwrap();
    assert_eq!(assert2_val.to_rust_string_lossy(scope), "json");
    let assert3 = import_assertions.get(scope, 2).unwrap();
    let assert3_val = v8::Local::<v8::Value>::try_from(assert3).unwrap();
    assert_eq!(assert3_val.to_rust_string_lossy(scope), "27");

    let origin = mock_script_origin(scope, "module.js");
    let src = v8::String::new(scope, "export const a = 'a';").unwrap();
    let source = v8::script_compiler::Source::new(src, Some(&origin));
    let module = v8::script_compiler::compile_module(scope, source).unwrap();
    Some(module)
  }

  extern "C" fn dynamic_import_cb(
    context: v8::Local<v8::Context>,
    _referrer: v8::Local<v8::ScriptOrModule>,
    _specifier: v8::Local<v8::String>,
    import_assertions: v8::Local<v8::FixedArray>,
  ) -> *mut v8::Promise {
    let scope = &mut unsafe { v8::CallbackScope::new(context) };
    let scope = &mut v8::HandleScope::new(scope);
    // "type" keyword, value
    assert_eq!(import_assertions.length(), 2);
    let assert1 = import_assertions.get(scope, 0).unwrap();
    let assert1_val = v8::Local::<v8::Value>::try_from(assert1).unwrap();
    assert_eq!(assert1_val.to_rust_string_lossy(scope), "type");
    let assert2 = import_assertions.get(scope, 1).unwrap();
    let assert2_val = v8::Local::<v8::Value>::try_from(assert2).unwrap();
    assert_eq!(assert2_val.to_rust_string_lossy(scope), "json");
    std::ptr::null_mut()
  }
  isolate.set_host_import_module_dynamically_callback(dynamic_import_cb);

  {
    let scope = &mut v8::HandleScope::new(isolate);
    let context = v8::Context::new(scope);
    let scope = &mut v8::ContextScope::new(scope, context);

    let source_text = v8::String::new(
      scope,
      "import 'foo.json' assert { type: \"json\" };\n\
        import('foo.json', { assert: { type: 'json' } });",
    )
    .unwrap();
    let origin = mock_script_origin(scope, "foo.js");
    let source = v8::script_compiler::Source::new(source_text, Some(&origin));

    let module = v8::script_compiler::compile_module(scope, source).unwrap();
    assert!(module.script_id().is_some());
    assert!(module.is_source_text_module());
    assert!(!module.is_synthetic_module());
    assert_eq!(v8::ModuleStatus::Uninstantiated, module.get_status());
    module.hash(&mut DefaultHasher::new()); // Should not crash.

    let result = module.instantiate_module(scope, module_resolve_callback);
    assert!(result.unwrap());
    assert_eq!(v8::ModuleStatus::Instantiated, module.get_status());
  }
}

#[test]
fn primitive_array() {
  let _setup_guard = setup();
  let isolate = &mut v8::Isolate::new(Default::default());
  {
    let scope = &mut v8::HandleScope::new(isolate);
    let context = v8::Context::new(scope);
    let scope = &mut v8::ContextScope::new(scope, context);

    let length = 3;
    let array = v8::PrimitiveArray::new(scope, length);
    assert_eq!(length, array.length());

    for i in 0..length {
      let item = array.get(scope, i);
      assert!(item.is_undefined());
    }

    let string = v8::String::new(scope, "test").unwrap();
    array.set(scope, 1, string.into());
    assert!(array.get(scope, 0).is_undefined());
    assert!(array.get(scope, 1).is_string());

    let num = v8::Number::new(scope, 0.42);
    array.set(scope, 2, num.into());
    assert!(array.get(scope, 0).is_undefined());
    assert!(array.get(scope, 1).is_string());
    assert!(array.get(scope, 2).is_number());
  }
}

#[test]
fn equality() {
  let _setup_guard = setup();
  let isolate = &mut v8::Isolate::new(Default::default());
  {
    let scope = &mut v8::HandleScope::new(isolate);
    let context = v8::Context::new(scope);
    let scope = &mut v8::ContextScope::new(scope, context);

    assert!(v8::String::new(scope, "a")
      .unwrap()
      .strict_equals(v8::String::new(scope, "a").unwrap().into()));
    assert!(!v8::String::new(scope, "a")
      .unwrap()
      .strict_equals(v8::String::new(scope, "b").unwrap().into()));

    assert!(v8::String::new(scope, "a")
      .unwrap()
      .same_value(v8::String::new(scope, "a").unwrap().into()));
    assert!(!v8::String::new(scope, "a")
      .unwrap()
      .same_value(v8::String::new(scope, "b").unwrap().into()));
  }
}

#[test]
#[allow(clippy::eq_op)]
fn equality_edge_cases() {
  let _setup_guard = setup();
  let isolate = &mut v8::Isolate::new(Default::default());

  let scope = &mut v8::HandleScope::new(isolate);
  let context = v8::Context::new(scope);
  let scope = &mut v8::ContextScope::new(scope, context);

  let pos_zero = eval(scope, "0").unwrap();
  let neg_zero = eval(scope, "-0").unwrap();
  let nan = eval(scope, "NaN").unwrap();

  assert!(pos_zero == pos_zero);
  assert!(pos_zero.same_value(pos_zero));
  assert!(pos_zero.same_value_zero(pos_zero));
  assert!(pos_zero.strict_equals(pos_zero));
  assert_eq!(pos_zero.get_hash(), pos_zero.get_hash());

  assert!(neg_zero == neg_zero);
  assert!(neg_zero.same_value(neg_zero));
  assert!(neg_zero.same_value_zero(neg_zero));
  assert!(neg_zero.strict_equals(neg_zero));
  assert_eq!(neg_zero.get_hash(), neg_zero.get_hash());

  assert!(pos_zero == neg_zero);
  assert!(!pos_zero.same_value(neg_zero));
  assert!(pos_zero.same_value_zero(neg_zero));
  assert!(pos_zero.strict_equals(neg_zero));
  assert_eq!(pos_zero.get_hash(), neg_zero.get_hash());

  assert!(neg_zero == pos_zero);
  assert!(!neg_zero.same_value(pos_zero));
  assert!(neg_zero.same_value_zero(pos_zero));
  assert!(neg_zero.strict_equals(pos_zero));
  assert_eq!(neg_zero.get_hash(), pos_zero.get_hash());

  assert!(nan == nan);
  assert!(nan.same_value(nan));
  assert!(nan.same_value_zero(nan));
  assert!(!nan.strict_equals(nan));
  assert_eq!(nan.get_hash(), nan.get_hash());

  assert!(nan != pos_zero);
  assert!(!nan.same_value(pos_zero));
  assert!(!nan.same_value_zero(pos_zero));
  assert!(!nan.strict_equals(pos_zero));

  assert!(neg_zero != nan);
  assert!(!neg_zero.same_value(nan));
  assert!(!neg_zero.same_value_zero(nan));
  assert!(!neg_zero.strict_equals(nan));
}

#[test]
fn get_hash() {
  use std::collections::HashMap;
  use std::collections::HashSet;
  use std::iter::once;

  let _setup_guard = setup();
  let isolate = &mut v8::Isolate::new(Default::default());

  let scope = &mut v8::HandleScope::new(isolate);
  let context = v8::Context::new(scope);
  let scope = &mut v8::ContextScope::new(scope, context);

  // Note: the set with hashes and the collition counter is used below in both
  // the 'primitives' and the 'objects' section.
  let mut hashes = HashSet::new();
  let mut collision_count = 0;

  let mut get_primitives = || -> v8::Local<v8::Array> {
    eval(
      scope,
      r#"[
        undefined,
        null,
        false,
        true,
        0,
        123,
        12345e67,
        123456789012345678901234567890123456789012345678901234567890n,
        NaN,
        -Infinity,
        "",
        "hello metaverse!",
        Symbol.isConcatSpreadable
      ]"#,
    )
    .unwrap()
    .try_into()
    .unwrap()
  };

  let primitives1 = get_primitives();
  let primitives2 = get_primitives();

  let len = primitives1.length();
  assert!(len > 10);
  assert_eq!(len, primitives2.length());

  let mut name_count = 0;

  for i in 0..len {
    let pri1 = primitives1.get_index(scope, i).unwrap();
    let pri2 = primitives2.get_index(scope, i).unwrap();
    let hash = pri1.get_hash();
    assert_ne!(hash, 0);
    assert_eq!(hash, pri2.get_hash());
    if let Ok(name) = v8::Local::<v8::Name>::try_from(pri1) {
      assert_eq!(hash, name.get_identity_hash());
      name_count += 1;
    }
    if !hashes.insert(hash) {
      collision_count += 1;
    }
    let map =
      once((v8::Global::new(scope, pri1), i)).collect::<HashMap<_, _>>();
    assert_eq!(map[&*pri2], i);
  }

  assert_eq!(name_count, 3);
  assert!(collision_count <= 2);

  for _ in 0..1 {
    let objects: v8::Local::<v8::Array> = eval(
      scope,
      r#"[
        [1, 2, 3],
        (function() { return arguments; })(1, 2, 3),
        { a: 1, b: 2, c: 3 },
        Object.create(null),
        new Map([[null, 1], ["2", 3n]]),
        new Set(),
        function f() {},
        function* f() {},
        async function f() {},
        async function* f() {},
        foo => foo,
        async bar => bar,
        class Custom extends Object { method(p) { return -p; } },
        new class MyString extends String { constructor() { super("yeaeaeah"); } },
        (() => { try { not_defined } catch(e) { return e; } })()
      ]"#)
    .unwrap()
    .try_into()
    .unwrap();

    let len = objects.length();
    assert!(len > 10);

    for i in 0..len {
      let val = objects.get_index(scope, i).unwrap();
      let hash = val.get_hash();
      assert_ne!(hash, 0);
      let obj = v8::Local::<v8::Object>::try_from(val).unwrap();
      assert_eq!(hash, obj.get_identity_hash());
      if !hashes.insert(hash) {
        collision_count += 1;
      }
      let map =
        once((v8::Global::new(scope, obj), i)).collect::<HashMap<_, _>>();
      assert_eq!(map[&*obj], i);
    }

    assert!(collision_count <= 2);
  }

  // TODO: add tests for `External` and for types that are not derived from
  // `v8::Value`, like `Module`, `Function/ObjectTemplate` etc.
}

#[test]
fn array_buffer_view() {
  let _setup_guard = setup();
  let isolate = &mut v8::Isolate::new(Default::default());
  {
    let scope = &mut v8::HandleScope::new(isolate);
    let context = v8::Context::new(scope);
    let scope = &mut v8::ContextScope::new(scope, context);
    let source =
      v8::String::new(scope, "new Uint8Array([23,23,23,23])").unwrap();
    let script = v8::Script::compile(scope, source, None).unwrap();
    source.to_rust_string_lossy(scope);
    let result: v8::Local<v8::ArrayBufferView> =
      script.run(scope).unwrap().try_into().unwrap();
    assert_eq!(result.byte_length(), 4);
    assert_eq!(result.byte_offset(), 0);
    let mut dest = [0; 4];
    let copy_bytes = result.copy_contents(&mut dest);
    assert_eq!(copy_bytes, 4);
    assert_eq!(dest, [23, 23, 23, 23]);
    let maybe_ab = result.buffer(scope);
    assert!(maybe_ab.is_some());
    let ab = maybe_ab.unwrap();
    assert_eq!(ab.byte_length(), 4);
  }
}

#[test]
fn snapshot_creator() {
  let _setup_guard = setup();
  // First we create the snapshot, there is a single global variable 'a' set to
  // the value 3.
  let isolate_data_index;
  let context_data_index;
  let context_data_index_2;
  let startup_data = {
    let mut snapshot_creator = v8::SnapshotCreator::new(None);
    // TODO(ry) this shouldn't be necessary. workaround unfinished business in
    // the scope type system.
    let mut isolate = unsafe { snapshot_creator.get_owned_isolate() };
    {
      // Check that the SnapshotCreator isolate has been set up correctly.
      let _ = isolate.thread_safe_handle();

      let scope = &mut v8::HandleScope::new(&mut isolate);
      let context = v8::Context::new(scope);
      let scope = &mut v8::ContextScope::new(scope, context);

      let source = v8::String::new(scope, "a = 1 + 2").unwrap();
      let script = v8::Script::compile(scope, source, None).unwrap();
      script.run(scope).unwrap();

      snapshot_creator.set_default_context(context);

      isolate_data_index =
        snapshot_creator.add_isolate_data(v8::Number::new(scope, 1.0));
      context_data_index =
        snapshot_creator.add_context_data(context, v8::Number::new(scope, 2.0));
      context_data_index_2 =
        snapshot_creator.add_context_data(context, v8::Number::new(scope, 3.0));
    }
    std::mem::forget(isolate); // TODO(ry) this shouldn't be necessary.
    snapshot_creator
      .create_blob(v8::FunctionCodeHandling::Clear)
      .unwrap()
  };
  assert!(startup_data.len() > 0);
  // Now we try to load up the snapshot and check that 'a' has the correct
  // value.
  {
    let params = v8::Isolate::create_params().snapshot_blob(startup_data);
    let isolate = &mut v8::Isolate::new(params);
    {
      let scope = &mut v8::HandleScope::new(isolate);
      let context = v8::Context::new(scope);
      let scope = &mut v8::ContextScope::new(scope, context);
      let source = v8::String::new(scope, "a === 3").unwrap();
      let script = v8::Script::compile(scope, source, None).unwrap();
      let result = script.run(scope).unwrap();
      let true_val = v8::Boolean::new(scope, true).into();
      assert!(result.same_value(true_val));

      let isolate_data = scope
        .get_isolate_data_from_snapshot_once::<v8::Value>(isolate_data_index);
      assert!(isolate_data.unwrap() == v8::Number::new(scope, 1.0));
      let no_data_err = scope
        .get_isolate_data_from_snapshot_once::<v8::Value>(isolate_data_index);
      assert!(matches!(no_data_err, Err(v8::DataError::NoData { .. })));

      let context_data = scope
        .get_context_data_from_snapshot_once::<v8::Value>(context_data_index);
      assert!(context_data.unwrap() == v8::Number::new(scope, 2.0));
      let no_data_err = scope
        .get_context_data_from_snapshot_once::<v8::Value>(context_data_index);
      assert!(matches!(no_data_err, Err(v8::DataError::NoData { .. })));

      let bad_type_err = scope
        .get_context_data_from_snapshot_once::<v8::Private>(
          context_data_index_2,
        );
      assert!(matches!(bad_type_err, Err(v8::DataError::BadType { .. })));
    }
  }
}

lazy_static! {
  static ref EXTERNAL_REFERENCES: v8::ExternalReferences =
    v8::ExternalReferences::new(&[v8::ExternalReference {
      function: fn_callback.map_fn_to()
    }]);
}

#[test]
fn external_references() {
  let _setup_guard = setup();
  // First we create the snapshot, there is a single global variable 'a' set to
  // the value 3.
  let startup_data = {
    let mut snapshot_creator =
      v8::SnapshotCreator::new(Some(&EXTERNAL_REFERENCES));
    // TODO(ry) this shouldn't be necessary. workaround unfinished business in
    // the scope type system.
    let mut isolate = unsafe { snapshot_creator.get_owned_isolate() };
    {
      let scope = &mut v8::HandleScope::new(&mut isolate);
      let context = v8::Context::new(scope);
      let scope = &mut v8::ContextScope::new(scope, context);

      // create function using template
      let fn_template = v8::FunctionTemplate::new(scope, fn_callback);
      let function = fn_template
        .get_function(scope)
        .expect("Unable to create function");

      let global = context.global(scope);
      let key = v8::String::new(scope, "F").unwrap();
      global.set(scope, key.into(), function.into());

      snapshot_creator.set_default_context(context);
    }
    std::mem::forget(isolate); // TODO(ry) this shouldn't be necessary.
    snapshot_creator
      .create_blob(v8::FunctionCodeHandling::Clear)
      .unwrap()
  };
  assert!(startup_data.len() > 0);
  // Now we try to load up the snapshot and check that 'a' has the correct
  // value.
  {
    let params = v8::Isolate::create_params()
      .snapshot_blob(startup_data)
      .external_references(&**EXTERNAL_REFERENCES);
    let isolate = &mut v8::Isolate::new(params);
    {
      let scope = &mut v8::HandleScope::new(isolate);
      let context = v8::Context::new(scope);
      let scope = &mut v8::ContextScope::new(scope, context);

      let result = eval(scope, "if(F() != 'wrong answer') throw 'boom1'");
      assert!(result.is_none());

      let result = eval(scope, "if(F() != 'Hello callback!') throw 'boom2'");
      assert!(result.is_some());
    }
  }
}

#[test]
fn create_params_snapshot_blob() {
  let static_data = b"abcd";
  let _ = v8::CreateParams::default().snapshot_blob(&static_data[..]);

  let vec_1 = Vec::from(&b"defg"[..]);
  let _ = v8::CreateParams::default().snapshot_blob(vec_1);

  let vec_2 = std::fs::read(file!()).unwrap();
  let _ = v8::CreateParams::default().snapshot_blob(vec_2);

  let arc_slice: std::sync::Arc<[u8]> = std::fs::read(file!()).unwrap().into();
  let _ = v8::CreateParams::default().snapshot_blob(arc_slice.clone());
  let _ = v8::CreateParams::default().snapshot_blob(arc_slice);
}

#[test]
fn uint8_array() {
  let _setup_guard = setup();
  let isolate = &mut v8::Isolate::new(Default::default());
  {
    let scope = &mut v8::HandleScope::new(isolate);
    let context = v8::Context::new(scope);
    let scope = &mut v8::ContextScope::new(scope, context);
    let source =
      v8::String::new(scope, "new Uint8Array([23,23,23,23])").unwrap();
    let script = v8::Script::compile(scope, source, None).unwrap();
    source.to_rust_string_lossy(scope);
    let result: v8::Local<v8::ArrayBufferView> =
      script.run(scope).unwrap().try_into().unwrap();
    assert_eq!(result.byte_length(), 4);
    assert_eq!(result.byte_offset(), 0);
    let mut dest = [0; 4];
    let copy_bytes = result.copy_contents(&mut dest);
    assert_eq!(copy_bytes, 4);
    assert_eq!(dest, [23, 23, 23, 23]);
    let maybe_ab = result.buffer(scope);
    assert!(maybe_ab.is_some());
    let ab = maybe_ab.unwrap();
    let uint8_array = v8::Uint8Array::new(scope, ab, 0, 0);
    assert!(uint8_array.is_some());
  }
}

#[test]
fn typed_array_constructors() {
  let _setup_guard = setup();
  let isolate = &mut v8::Isolate::new(Default::default());
  let scope = &mut v8::HandleScope::new(isolate);
  let context = v8::Context::new(scope);
  let scope = &mut v8::ContextScope::new(scope, context);

  let ab = v8::ArrayBuffer::new(scope, 8);

  let t = v8::Uint8Array::new(scope, ab, 0, 0).unwrap();
  assert!(t.is_uint8_array());

  let t = v8::Uint8ClampedArray::new(scope, ab, 0, 0).unwrap();
  assert!(t.is_uint8_clamped_array());

  let t = v8::Int8Array::new(scope, ab, 0, 0).unwrap();
  assert!(t.is_int8_array());

  let t = v8::Uint16Array::new(scope, ab, 0, 0).unwrap();
  assert!(t.is_uint16_array());

  let t = v8::Int16Array::new(scope, ab, 0, 0).unwrap();
  assert!(t.is_int16_array());

  let t = v8::Uint32Array::new(scope, ab, 0, 0).unwrap();
  assert!(t.is_uint32_array());

  let t = v8::Int32Array::new(scope, ab, 0, 0).unwrap();
  assert!(t.is_int32_array());

  let t = v8::Float32Array::new(scope, ab, 0, 0).unwrap();
  assert!(t.is_float32_array());

  let t = v8::Float64Array::new(scope, ab, 0, 0).unwrap();
  assert!(t.is_float64_array());

  let t = v8::BigUint64Array::new(scope, ab, 0, 0).unwrap();
  assert!(t.is_big_uint64_array());

  let t = v8::BigInt64Array::new(scope, ab, 0, 0).unwrap();
  assert!(t.is_big_int64_array());
}

#[test]
fn dynamic_import() {
  let _setup_guard = setup();
  let isolate = &mut v8::Isolate::new(Default::default());

  static CALL_COUNT: AtomicUsize = AtomicUsize::new(0);

  extern "C" fn dynamic_import_cb(
    context: v8::Local<v8::Context>,
    _referrer: v8::Local<v8::ScriptOrModule>,
    specifier: v8::Local<v8::String>,
    _import_assertions: v8::Local<v8::FixedArray>,
  ) -> *mut v8::Promise {
    let scope = &mut unsafe { v8::CallbackScope::new(context) };
    let scope = &mut v8::HandleScope::new(scope);
    assert!(
      specifier.strict_equals(v8::String::new(scope, "bar.js").unwrap().into())
    );
    let e = v8::String::new(scope, "boom").unwrap();
    scope.throw_exception(e.into());
    CALL_COUNT.fetch_add(1, Ordering::SeqCst);
    std::ptr::null_mut()
  }
  isolate.set_host_import_module_dynamically_callback(dynamic_import_cb);

  {
    let scope = &mut v8::HandleScope::new(isolate);
    let context = v8::Context::new(scope);
    let scope = &mut v8::ContextScope::new(scope, context);

    let result = eval(
      scope,
      "(async function () {\n\
         let x = await import('bar.js');\n\
       })();",
    );
    assert!(result.is_some());
    assert_eq!(CALL_COUNT.load(Ordering::SeqCst), 1);
  }
}

#[test]
fn shared_array_buffer() {
  let _setup_guard = setup();
  let isolate = &mut v8::Isolate::new(Default::default());
  {
    let scope = &mut v8::HandleScope::new(isolate);
    let context = v8::Context::new(scope);
    let scope = &mut v8::ContextScope::new(scope, context);

    let sab = v8::SharedArrayBuffer::new(scope, 16).unwrap();
    let shared_bs_1 = sab.get_backing_store();
    shared_bs_1[5].set(12);
    shared_bs_1[12].set(52);

    let global = context.global(scope);
    let key = v8::String::new(scope, "shared").unwrap();
    let r = global
      .create_data_property(scope, key.into(), sab.into())
      .unwrap();
    assert!(r);
    let source = v8::String::new(
      scope,
      r"sharedBytes = new Uint8Array(shared);
        sharedBytes[2] = 16;
        sharedBytes[14] = 62;
        sharedBytes[5] + sharedBytes[12]",
    )
    .unwrap();
    let script = v8::Script::compile(scope, source, None).unwrap();

    let result: v8::Local<v8::Integer> =
      script.run(scope).unwrap().try_into().unwrap();
    assert_eq!(result.value(), 64);
    assert_eq!(shared_bs_1[2].get(), 16);
    assert_eq!(shared_bs_1[14].get(), 62);

    let data: Box<[u8]> = vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9].into_boxed_slice();
    let bs = v8::SharedArrayBuffer::new_backing_store_from_boxed_slice(data);
    assert_eq!(bs.byte_length(), 10);
    assert!(bs.is_shared());

    let shared_bs_2 = bs.make_shared();
    assert_eq!(shared_bs_2.byte_length(), 10);
    assert!(shared_bs_2.is_shared());

    let ab = v8::SharedArrayBuffer::with_backing_store(scope, &shared_bs_2);
    let shared_bs_3 = ab.get_backing_store();
    assert_eq!(shared_bs_3.byte_length(), 10);
    assert_eq!(shared_bs_3[0].get(), 0);
    assert_eq!(shared_bs_3[9].get(), 9);
  }
}

#[test]
#[allow(clippy::cognitive_complexity)]
#[allow(clippy::eq_op)]
fn value_checker() {
  let _setup_guard = setup();
  let isolate = &mut v8::Isolate::new(Default::default());
  {
    let scope = &mut v8::HandleScope::new(isolate);
    let context = v8::Context::new(scope);
    let scope = &mut v8::ContextScope::new(scope, context);

    let value = eval(scope, "undefined").unwrap();
    assert!(value.is_undefined());
    assert!(value.is_null_or_undefined());
    assert!(value == value);
    assert!(value == v8::Local::<v8::Primitive>::try_from(value).unwrap());
    assert!(value == v8::undefined(scope));
    assert!(value != v8::null(scope));
    assert!(value != v8::Boolean::new(scope, false));
    assert!(value != v8::Integer::new(scope, 0));

    let value = eval(scope, "null").unwrap();
    assert!(value.is_null());
    assert!(value.is_null_or_undefined());
    assert!(value == value);
    assert!(value == v8::Local::<v8::Primitive>::try_from(value).unwrap());
    assert!(value == v8::null(scope));
    assert!(value == v8::Global::new(scope, value));
    assert!(v8::Global::new(scope, value) == v8::Global::new(scope, value));
    assert!(v8::Global::new(scope, value) == v8::null(scope));
    assert!(value != v8::undefined(scope));
    assert!(value != v8::Boolean::new(scope, false));
    assert!(value != v8::Integer::new(scope, 0));
    assert!(value.to_boolean(scope) == v8::Boolean::new(scope, false));
    assert!(!value.boolean_value(scope));

    let value = eval(scope, "true").unwrap();
    assert!(value.is_boolean());
    assert!(value.is_true());
    assert!(!value.is_false());
    assert!(value == value);
    assert!(value == v8::Local::<v8::Boolean>::try_from(value).unwrap());
    assert!(value == v8::Boolean::new(scope, true));
    assert!(value == v8::Global::new(scope, value));
    assert!(v8::Global::new(scope, value) == v8::Global::new(scope, value));
    assert!(v8::Global::new(scope, value) == eval(scope, "!false").unwrap());
    assert!(v8::Global::new(scope, value) != eval(scope, "1").unwrap());
    assert!(value != v8::Boolean::new(scope, false));
    assert!(value.boolean_value(scope));

    let value = eval(scope, "false").unwrap();
    assert!(value.is_boolean());
    assert!(!value.is_true());
    assert!(value.is_false());
    assert!(value == value);
    assert!(value == v8::Local::<v8::Boolean>::try_from(value).unwrap());
    assert!(value == v8::Boolean::new(scope, false));
    assert!(value == v8::Global::new(scope, value));
    assert!(v8::Global::new(scope, value) == v8::Global::new(scope, value));
    assert!(v8::Global::new(scope, value) == eval(scope, "!true").unwrap());
    assert!(v8::Global::new(scope, value) != eval(scope, "0").unwrap());
    assert!(value != v8::Boolean::new(scope, true));
    assert!(value != v8::null(scope));
    assert!(value != v8::undefined(scope));
    assert!(value != v8::Integer::new(scope, 0));
    assert!(!value.boolean_value(scope));

    let value = eval(scope, "'name'").unwrap();
    assert!(value.is_name());
    assert!(value.is_string());
    assert!(value == value);
    assert!(value == v8::Local::<v8::String>::try_from(value).unwrap());
    assert!(value == v8::String::new(scope, "name").unwrap());
    assert!(value != v8::String::new(scope, "name\0").unwrap());
    assert!(value != v8::Object::new(scope));
    assert!(value.to_boolean(scope) == v8::Boolean::new(scope, true));
    assert!(value.boolean_value(scope));

    let value = eval(scope, "Symbol()").unwrap();
    assert!(value.is_name());
    assert!(value.is_symbol());
    assert!(value == value);
    assert!(value == v8::Local::<v8::Symbol>::try_from(value).unwrap());
    assert!(value == v8::Global::new(scope, value));
    assert!(v8::Global::new(scope, value) == v8::Global::new(scope, value));
    assert!(value != eval(scope, "Symbol()").unwrap());
    assert!(v8::Global::new(scope, value) != eval(scope, "Symbol()").unwrap());

    let value = eval(scope, "() => 0").unwrap();
    assert!(value.is_function());
    assert!(value == value);
    assert!(value == v8::Local::<v8::Function>::try_from(value).unwrap());
    assert!(value == v8::Global::new(scope, value));
    assert!(v8::Global::new(scope, value) == v8::Global::new(scope, value));
    assert!(value != eval(scope, "() => 0").unwrap());
    assert!(v8::Global::new(scope, value) != eval(scope, "() => 0").unwrap());

    let value = eval(scope, "async () => 0").unwrap();
    assert!(value.is_async_function());
    assert!(value == value);
    assert!(value == v8::Local::<v8::Function>::try_from(value).unwrap());
    assert!(v8::Global::new(scope, value) == v8::Global::new(scope, value));
    assert!(value != v8::Object::new(scope));
    assert!(v8::Global::new(scope, value) != v8::Object::new(scope));

    let value = eval(scope, "[]").unwrap();
    assert!(value.is_array());
    assert!(value == value);
    assert!(value == v8::Local::<v8::Array>::try_from(value).unwrap());
    assert!(value != v8::Array::new(scope, 0));

    let value = eval(scope, "9007199254740995n").unwrap();
    assert!(value.is_big_int());
    assert!(value.to_big_int(scope).is_some());
    assert!(value == value);
    assert!(value == v8::Local::<v8::BigInt>::try_from(value).unwrap());
    assert!(value == eval(scope, "1801439850948199n * 5n").unwrap());
    assert!(value != eval(scope, "1801439850948199 * 5").unwrap());
    let detail_string = value.to_detail_string(scope).unwrap();
    let detail_string = detail_string.to_rust_string_lossy(scope);
    assert_eq!("9007199254740995", detail_string);

    let value = eval(scope, "123").unwrap();
    assert!(value.is_number());
    assert!(value.is_int32());
    assert!(value.is_uint32());
    assert!(value == value);
    assert!(value == v8::Local::<v8::Number>::try_from(value).unwrap());
    assert!(value == v8::Integer::new(scope, 123));
    assert!(value == v8::Number::new(scope, 123f64));
    assert!(value == value.to_int32(scope).unwrap());
    assert!(value != value.to_string(scope).unwrap());
    assert_eq!(123, value.to_uint32(scope).unwrap().value());
    assert_eq!(123, value.to_int32(scope).unwrap().value());
    assert_eq!(123, value.to_integer(scope).unwrap().value());
    assert_eq!(123, value.integer_value(scope).unwrap());
    assert_eq!(123, value.uint32_value(scope).unwrap());
    assert_eq!(123, value.int32_value(scope).unwrap());

    let value = eval(scope, "12.3").unwrap();
    assert!(value.is_number());
    assert!(!value.is_int32());
    assert!(!value.is_uint32());
    assert!(value == value);
    assert!(value == v8::Local::<v8::Number>::try_from(value).unwrap());
    assert!(value == v8::Number::new(scope, 12.3f64));
    assert!(value != value.to_integer(scope).unwrap());
    assert!(12.3 - value.number_value(scope).unwrap() < 0.00001);

    let value = eval(scope, "-123").unwrap();
    assert!(value.is_number());
    assert!(value.is_int32());
    assert!(!value.is_uint32());
    assert!(value == value);
    assert!(value == v8::Local::<v8::Int32>::try_from(value).unwrap());
    assert!(value == v8::Integer::new(scope, -123));
    assert!(value == v8::Number::new(scope, -123f64));
    assert!(value != v8::String::new(scope, "-123").unwrap());
    assert!(
      value
        == v8::Integer::new_from_unsigned(scope, -123i32 as u32)
          .to_int32(scope)
          .unwrap()
    );
    // The following test does not pass. This appears to be a V8 bug.
    // assert!(value != value.to_uint32(scope).unwrap());

    let value = eval(scope, "NaN").unwrap();
    assert!(value.is_number());
    assert!(!value.is_int32());
    assert!(!value.is_uint32());
    assert!(!value.strict_equals(value));
    assert!(
      value.to_string(scope).unwrap() == v8::String::new(scope, "NaN").unwrap()
    );

    let value = eval(scope, "({})").unwrap();
    assert!(value.is_object());
    assert!(value == value);
    assert!(value == v8::Local::<v8::Object>::try_from(value).unwrap());
    assert!(value == v8::Global::new(scope, value));
    assert!(v8::Global::new(scope, value) == v8::Global::new(scope, value));
    assert!(value != v8::Object::new(scope));
    assert!(v8::Global::new(scope, value) != v8::Object::new(scope));

    let value = eval(scope, "new Date()").unwrap();
    assert!(value.is_date());
    assert!(value == value);
    assert!(value == v8::Local::<v8::Date>::try_from(value).unwrap());
    assert!(value != eval(scope, "new Date()").unwrap());

    let value = eval(scope, "(function(){return arguments})()").unwrap();
    assert!(value.is_arguments_object());
    assert!(value == value);
    assert!(value == v8::Local::<v8::Object>::try_from(value).unwrap());
    assert!(value != v8::Object::new(scope));

    let value = eval(scope, "new Promise(function(){})").unwrap();
    assert!(value.is_promise());
    assert!(value == value);
    assert!(value == v8::Local::<v8::Promise>::try_from(value).unwrap());
    assert!(value != v8::Object::new(scope));

    let value = eval(scope, "new Map()").unwrap();
    assert!(value.is_map());
    assert!(value == value);
    assert!(value == v8::Local::<v8::Map>::try_from(value).unwrap());
    assert!(value != v8::Object::new(scope));

    let value = eval(scope, "new Set").unwrap();
    assert!(value.is_set());
    assert!(value == value);
    assert!(value == v8::Local::<v8::Set>::try_from(value).unwrap());
    assert!(value != v8::Object::new(scope));

    let value = eval(scope, "new Map().entries()").unwrap();
    assert!(value.is_map_iterator());
    assert!(value == value);
    assert!(value == v8::Local::<v8::Object>::try_from(value).unwrap());
    assert!(value != v8::Object::new(scope));

    let value = eval(scope, "new Set().entries()").unwrap();
    assert!(value.is_set_iterator());
    assert!(value == value);
    assert!(value == v8::Local::<v8::Object>::try_from(value).unwrap());
    assert!(value != v8::Object::new(scope));

    let value = eval(scope, "new WeakMap()").unwrap();
    assert!(value.is_weak_map());
    assert!(value == value);
    assert!(value == v8::Local::<v8::Object>::try_from(value).unwrap());
    assert!(value != v8::Object::new(scope));

    let value = eval(scope, "new WeakSet()").unwrap();
    assert!(value.is_weak_set());
    assert!(value == value);
    assert!(value == v8::Local::<v8::Object>::try_from(value).unwrap());
    assert!(value != v8::Object::new(scope));

    let value = eval(scope, "new ArrayBuffer(8)").unwrap();
    assert!(value.is_array_buffer());
    assert!(value == value);
    assert!(value == v8::Local::<v8::ArrayBuffer>::try_from(value).unwrap());
    assert!(value != v8::Object::new(scope));

    let value = eval(scope, "new Uint8Array([])").unwrap();
    assert!(value.is_uint8_array());
    assert!(value.is_array_buffer_view());
    assert!(value.is_typed_array());
    assert!(value == value);
    assert!(value == v8::Local::<v8::Uint8Array>::try_from(value).unwrap());
    assert!(value != v8::Object::new(scope));

    let value = eval(scope, "new Uint8ClampedArray([])").unwrap();
    assert!(value.is_uint8_clamped_array());
    assert!(value.is_array_buffer_view());
    assert!(value.is_typed_array());
    assert!(value == value);
    assert!(
      value == v8::Local::<v8::Uint8ClampedArray>::try_from(value).unwrap()
    );
    assert!(value != v8::Object::new(scope));

    let value = eval(scope, "new Int8Array([])").unwrap();
    assert!(value.is_int8_array());
    assert!(value.is_array_buffer_view());
    assert!(value.is_typed_array());
    assert!(value == value);
    assert!(value == v8::Local::<v8::Int8Array>::try_from(value).unwrap());
    assert!(value != v8::Object::new(scope));

    let value = eval(scope, "new Uint16Array([])").unwrap();
    assert!(value.is_uint16_array());
    assert!(value.is_array_buffer_view());
    assert!(value.is_typed_array());
    assert!(value == value);
    assert!(value == v8::Local::<v8::Uint16Array>::try_from(value).unwrap());
    assert!(value != v8::Object::new(scope));

    let value = eval(scope, "new Int16Array([])").unwrap();
    assert!(value.is_int16_array());
    assert!(value.is_array_buffer_view());
    assert!(value.is_typed_array());
    assert!(value == value);
    assert!(value == v8::Local::<v8::Int16Array>::try_from(value).unwrap());
    assert!(value != v8::Object::new(scope));

    let value = eval(scope, "new Uint32Array([])").unwrap();
    assert!(value.is_uint32_array());
    assert!(value.is_array_buffer_view());
    assert!(value.is_typed_array());
    assert!(value == value);
    assert!(value == v8::Local::<v8::Uint32Array>::try_from(value).unwrap());
    assert!(value != v8::Object::new(scope));

    let value = eval(scope, "new Int32Array([])").unwrap();
    assert!(value.is_int32_array());
    assert!(value.is_array_buffer_view());
    assert!(value.is_typed_array());
    assert!(value == value);
    assert!(value == v8::Local::<v8::Int32Array>::try_from(value).unwrap());
    assert!(value != v8::Object::new(scope));

    let value = eval(scope, "new Float32Array([])").unwrap();
    assert!(value.is_float32_array());
    assert!(value.is_array_buffer_view());
    assert!(value.is_typed_array());
    assert!(value == value);
    assert!(value == v8::Local::<v8::Float32Array>::try_from(value).unwrap());
    assert!(value != v8::Object::new(scope));

    let value = eval(scope, "new Float64Array([])").unwrap();
    assert!(value.is_float64_array());
    assert!(value.is_array_buffer_view());
    assert!(value.is_typed_array());
    assert!(value == value);
    assert!(value == v8::Local::<v8::Float64Array>::try_from(value).unwrap());
    assert!(value != v8::Object::new(scope));

    let value = eval(scope, "new BigInt64Array([])").unwrap();
    assert!(value.is_big_int64_array());
    assert!(value.is_array_buffer_view());
    assert!(value.is_typed_array());
    assert!(value == value);
    assert!(value == v8::Local::<v8::BigInt64Array>::try_from(value).unwrap());
    assert!(value != v8::Object::new(scope));

    let value = eval(scope, "new BigUint64Array([])").unwrap();
    assert!(value.is_big_uint64_array());
    assert!(value.is_array_buffer_view());
    assert!(value.is_typed_array());
    assert!(value == value);
    assert!(value == v8::Local::<v8::BigUint64Array>::try_from(value).unwrap());
    assert!(value != v8::Object::new(scope));

    let value = eval(scope, "new SharedArrayBuffer(64)").unwrap();
    assert!(value.is_shared_array_buffer());
    assert!(value == value);
    assert!(
      value == v8::Local::<v8::SharedArrayBuffer>::try_from(value).unwrap()
    );
    assert!(value != v8::Object::new(scope));

    let value = eval(scope, "new Proxy({},{})").unwrap();
    assert!(value.is_proxy());
    assert!(value == value);
    assert!(value == v8::Local::<v8::Proxy>::try_from(value).unwrap());
    assert!(value != v8::Object::new(scope));

    // Other checker, Just check if it can be called
    value.is_external();
    value.is_module_namespace_object();
    value.is_wasm_module_object();
  }
}

#[test]
fn try_from_data() {
  let _setup_guard = setup();

  let isolate = &mut v8::Isolate::new(Default::default());
  let scope = &mut v8::HandleScope::new(isolate);
  let context = v8::Context::new(scope);
  let scope = &mut v8::ContextScope::new(scope, context);

  let module_source = mock_source(scope, "answer.js", "fail()");
  let function_callback =
    |_: &mut v8::HandleScope,
     _: v8::FunctionCallbackArguments,
     _: v8::ReturnValue| { unreachable!() };

  let function_template = v8::FunctionTemplate::new(scope, function_callback);
  let d: v8::Local<v8::Data> = function_template.into();
  assert!(d.is_function_template());
  assert!(!d.is_module());
  assert!(!d.is_object_template());
  assert!(!d.is_private());
  assert!(!d.is_value());
  assert!(
    v8::Local::<v8::FunctionTemplate>::try_from(d).unwrap()
      == function_template
  );

  let module =
    v8::script_compiler::compile_module(scope, module_source).unwrap();
  let d: v8::Local<v8::Data> = module.into();
  assert!(!d.is_function_template());
  assert!(d.is_module());
  assert!(!d.is_object_template());
  assert!(!d.is_private());
  assert!(!d.is_value());
  assert!(v8::Local::<v8::Module>::try_from(d).unwrap() == module);

  let object_template = v8::ObjectTemplate::new(scope);
  let d: v8::Local<v8::Data> = object_template.into();
  assert!(!d.is_function_template());
  assert!(!d.is_module());
  assert!(d.is_object_template());
  assert!(!d.is_private());
  assert!(!d.is_value());
  assert!(
    v8::Local::<v8::ObjectTemplate>::try_from(d).unwrap() == object_template
  );

  let p: v8::Local<v8::Data> = v8::Private::new(scope, None).into();
  assert!(!p.is_function_template());
  assert!(!p.is_module());
  assert!(!p.is_object_template());
  assert!(p.is_private());
  assert!(!p.is_value());

  let values: &[v8::Local<v8::Value>] = &[
    v8::null(scope).into(),
    v8::undefined(scope).into(),
    v8::BigInt::new_from_u64(scope, 1337).into(),
    v8::Boolean::new(scope, true).into(),
    v8::Function::new(scope, function_callback).unwrap().into(),
    v8::Number::new(scope, 42.0).into(),
    v8::Object::new(scope).into(),
    v8::Symbol::new(scope, None).into(),
    v8::String::new(scope, "hello").unwrap().into(),
  ];
  for &v in values {
    let d: v8::Local<v8::Data> = v.into();
    assert!(!d.is_function_template());
    assert!(!d.is_module());
    assert!(!d.is_object_template());
    assert!(!d.is_private());
    assert!(d.is_value());
    assert!(v8::Local::<v8::Value>::try_from(d).unwrap() == v);
  }
}

#[test]
fn try_from_value() {
  let _setup_guard = setup();
  let isolate = &mut v8::Isolate::new(Default::default());
  {
    let scope = &mut v8::HandleScope::new(isolate);
    let context = v8::Context::new(scope);
    let scope = &mut v8::ContextScope::new(scope, context);

    {
      let value: v8::Local<v8::Value> = v8::undefined(scope).into();
      let _primitive = v8::Local::<v8::Primitive>::try_from(value).unwrap();
      assert!(matches!(
        v8::Local::<v8::Object>::try_from(value),
        Err(v8::DataError::BadType { expected, .. })
          if expected == type_name::<v8::Object>()
      ));
      assert!(matches!(
        v8::Local::<v8::Int32>::try_from(value),
        Err(v8::DataError::BadType { expected, .. })
          if expected == type_name::<v8::Int32>()
      ));
    }

    {
      let value: v8::Local<v8::Value> = v8::Boolean::new(scope, true).into();
      let primitive = v8::Local::<v8::Primitive>::try_from(value).unwrap();
      let _boolean = v8::Local::<v8::Boolean>::try_from(value).unwrap();
      let _boolean = v8::Local::<v8::Boolean>::try_from(primitive).unwrap();
      assert!(matches!(
        v8::Local::<v8::String>::try_from(value),
        Err(v8::DataError::BadType { expected, .. })
          if expected == type_name::<v8::String>()
      ));
      assert!(matches!(
        v8::Local::<v8::Number>::try_from(primitive),
        Err(v8::DataError::BadType { expected, .. })
          if expected == type_name::<v8::Number>()
      ));
    }

    {
      let value: v8::Local<v8::Value> = v8::Number::new(scope, -1234f64).into();
      let primitive = v8::Local::<v8::Primitive>::try_from(value).unwrap();
      let _number = v8::Local::<v8::Number>::try_from(value).unwrap();
      let number = v8::Local::<v8::Number>::try_from(primitive).unwrap();
      let _integer = v8::Local::<v8::Integer>::try_from(value).unwrap();
      let _integer = v8::Local::<v8::Integer>::try_from(primitive).unwrap();
      let integer = v8::Local::<v8::Integer>::try_from(number).unwrap();
      let _int32 = v8::Local::<v8::Int32>::try_from(value).unwrap();
      let _int32 = v8::Local::<v8::Int32>::try_from(primitive).unwrap();
      let _int32 = v8::Local::<v8::Int32>::try_from(integer).unwrap();
      let _int32 = v8::Local::<v8::Int32>::try_from(number).unwrap();
      assert!(matches!(
        v8::Local::<v8::String>::try_from(value),
        Err(v8::DataError::BadType { expected, .. })
          if expected == type_name::<v8::String>()
      ));
      assert!(matches!(
        v8::Local::<v8::Boolean>::try_from(primitive),
        Err(v8::DataError::BadType { expected, .. })
          if expected == type_name::<v8::Boolean>()
      ));
      assert!(matches!(
        v8::Local::<v8::Uint32>::try_from(integer),
        Err(v8::DataError::BadType { expected, .. })
          if expected == type_name::<v8::Uint32>()
      ));
    }

    {
      let value: v8::Local<v8::Value> = eval(scope, "(() => {})").unwrap();
      let object = v8::Local::<v8::Object>::try_from(value).unwrap();
      let _function = v8::Local::<v8::Function>::try_from(value).unwrap();
      let _function = v8::Local::<v8::Function>::try_from(object).unwrap();
      assert!(matches!(
        v8::Local::<v8::Primitive>::try_from(value),
        Err(v8::DataError::BadType { expected, .. })
          if expected == type_name::<v8::Primitive>()
      ));
      assert!(matches!(
        v8::Local::<v8::BigInt>::try_from(value),
        Err(v8::DataError::BadType { expected, .. })
          if expected == type_name::<v8::BigInt>()
      ));
      assert!(matches!(
        v8::Local::<v8::NumberObject>::try_from(value),
        Err(v8::DataError::BadType { expected, .. })
          if expected == type_name::<v8::NumberObject>()
      ));
      assert!(matches!(
        v8::Local::<v8::NumberObject>::try_from(object),
        Err(v8::DataError::BadType { expected, .. })
          if expected == type_name::<v8::NumberObject>()
      ));
      assert!(matches!(
        v8::Local::<v8::Set>::try_from(value),
        Err(v8::DataError::BadType { expected, .. })
          if expected == type_name::<v8::Set>()
      ));
      assert!(matches!(
        v8::Local::<v8::Set>::try_from(object),
        Err(v8::DataError::BadType { expected, .. })
          if expected == type_name::<v8::Set>()
      ));
    }
  }
}

struct ClientCounter {
  base: v8::inspector::V8InspectorClientBase,
  count_run_message_loop_on_pause: usize,
  count_quit_message_loop_on_pause: usize,
  count_run_if_waiting_for_debugger: usize,
  count_generate_unique_id: i64,
}

impl ClientCounter {
  fn new() -> Self {
    Self {
      base: v8::inspector::V8InspectorClientBase::new::<Self>(),
      count_run_message_loop_on_pause: 0,
      count_quit_message_loop_on_pause: 0,
      count_run_if_waiting_for_debugger: 0,
      count_generate_unique_id: 0,
    }
  }
}

impl v8::inspector::V8InspectorClientImpl for ClientCounter {
  fn base(&self) -> &v8::inspector::V8InspectorClientBase {
    &self.base
  }

  fn base_mut(&mut self) -> &mut v8::inspector::V8InspectorClientBase {
    &mut self.base
  }

  fn run_message_loop_on_pause(&mut self, context_group_id: i32) {
    assert_eq!(context_group_id, 1);
    self.count_run_message_loop_on_pause += 1;
  }

  fn quit_message_loop_on_pause(&mut self) {
    self.count_quit_message_loop_on_pause += 1;
  }

  fn run_if_waiting_for_debugger(&mut self, context_group_id: i32) {
    assert_eq!(context_group_id, 1);
    self.count_run_message_loop_on_pause += 1;
  }

  fn generate_unique_id(&mut self) -> i64 {
    self.count_generate_unique_id += 1;
    self.count_generate_unique_id
  }
}

struct ChannelCounter {
  base: v8::inspector::ChannelBase,
  count_send_response: usize,
  count_send_notification: usize,
  count_flush_protocol_notifications: usize,
}

impl ChannelCounter {
  pub fn new() -> Self {
    Self {
      base: v8::inspector::ChannelBase::new::<Self>(),
      count_send_response: 0,
      count_send_notification: 0,
      count_flush_protocol_notifications: 0,
    }
  }
}

impl v8::inspector::ChannelImpl for ChannelCounter {
  fn base(&self) -> &v8::inspector::ChannelBase {
    &self.base
  }
  fn base_mut(&mut self) -> &mut v8::inspector::ChannelBase {
    &mut self.base
  }
  fn send_response(
    &mut self,
    call_id: i32,
    message: v8::UniquePtr<v8::inspector::StringBuffer>,
  ) {
    println!(
      "send_response call_id {} message {}",
      call_id,
      message.unwrap().string()
    );
    self.count_send_response += 1;
  }
  fn send_notification(
    &mut self,
    message: v8::UniquePtr<v8::inspector::StringBuffer>,
  ) {
    println!("send_notification message {}", message.unwrap().string());
    self.count_send_notification += 1;
  }
  fn flush_protocol_notifications(&mut self) {
    self.count_flush_protocol_notifications += 1;
  }
}

#[test]
fn inspector_can_dispatch_method() {
  use v8::inspector::*;

  let message = String::from("Runtime.enable");
  let message = &message.into_bytes()[..];
  let string_view = StringView::from(message);
  assert!(V8InspectorSession::can_dispatch_method(string_view));

  let message = String::from("Debugger.enable");
  let message = &message.into_bytes()[..];
  let string_view = StringView::from(message);
  assert!(V8InspectorSession::can_dispatch_method(string_view));

  let message = String::from("Profiler.enable");
  let message = &message.into_bytes()[..];
  let string_view = StringView::from(message);
  assert!(V8InspectorSession::can_dispatch_method(string_view));

  let message = String::from("HeapProfiler.enable");
  let message = &message.into_bytes()[..];
  let string_view = StringView::from(message);
  assert!(V8InspectorSession::can_dispatch_method(string_view));

  let message = String::from("Console.enable");
  let message = &message.into_bytes()[..];
  let string_view = StringView::from(message);
  assert!(V8InspectorSession::can_dispatch_method(string_view));

  let message = String::from("Schema.getDomains");
  let message = &message.into_bytes()[..];
  let string_view = StringView::from(message);
  assert!(V8InspectorSession::can_dispatch_method(string_view));

  let message = String::from("Foo.enable");
  let message = &message.into_bytes()[..];
  let string_view = StringView::from(message);
  assert!(!V8InspectorSession::can_dispatch_method(string_view));

  let message = String::from("Bar.enable");
  let message = &message.into_bytes()[..];
  let string_view = StringView::from(message);
  assert!(!V8InspectorSession::can_dispatch_method(string_view));
}

#[test]
fn inspector_dispatch_protocol_message() {
  let _setup_guard = setup();
  let isolate = &mut v8::Isolate::new(Default::default());

  use v8::inspector::*;
  let mut default_client = ClientCounter::new();
  let mut inspector = V8Inspector::create(isolate, &mut default_client);

  let scope = &mut v8::HandleScope::new(isolate);
  let context = v8::Context::new(scope);
  let mut _scope = v8::ContextScope::new(scope, context);

  let name = b"";
  let name_view = StringView::from(&name[..]);
  inspector.context_created(context, 1, name_view);
  let mut channel = ChannelCounter::new();
  let state = b"{}";
  let state_view = StringView::from(&state[..]);
  let mut session = inspector.connect(1, &mut channel, state_view);
  let message = String::from(
    r#"{"id":1,"method":"Network.enable","params":{"maxPostDataSize":65536}}"#,
  );
  let message = &message.into_bytes()[..];
  let string_view = StringView::from(message);
  session.dispatch_protocol_message(string_view);
  assert_eq!(channel.count_send_response, 1);
  assert_eq!(channel.count_send_notification, 0);
  assert_eq!(channel.count_flush_protocol_notifications, 0);
}

#[test]
fn inspector_schedule_pause_on_next_statement() {
  let _setup_guard = setup();
  let isolate = &mut v8::Isolate::new(Default::default());

  use v8::inspector::*;
  let mut client = ClientCounter::new();
  let mut inspector = V8Inspector::create(isolate, &mut client);

  let scope = &mut v8::HandleScope::new(isolate);
  let context = v8::Context::new(scope);
  let scope = &mut v8::ContextScope::new(scope, context);

  let mut channel = ChannelCounter::new();
  let state = b"{}";
  let state_view = StringView::from(&state[..]);
  let mut session = inspector.connect(1, &mut channel, state_view);

  let name = b"";
  let name_view = StringView::from(&name[..]);
  inspector.context_created(context, 1, name_view);

  // In order for schedule_pause_on_next_statement to work, it seems you need
  // to first enable the debugger.
  let message = String::from(r#"{"id":1,"method":"Debugger.enable"}"#);
  let message = &message.into_bytes()[..];
  let message = StringView::from(message);
  session.dispatch_protocol_message(message);

  // The following commented out block seems to act similarly to
  // schedule_pause_on_next_statement. I'm not sure if they have the exact same
  // effect tho.
  //   let message = String::from(r#"{"id":2,"method":"Debugger.pause"}"#);
  //   let message = &message.into_bytes()[..];
  //   let message = StringView::from(message);
  //   session.dispatch_protocol_message(&message);
  let reason = b"";
  let reason = StringView::from(&reason[..]);
  let detail = b"";
  let detail = StringView::from(&detail[..]);
  session.schedule_pause_on_next_statement(reason, detail);

  assert_eq!(channel.count_send_response, 1);
  assert_eq!(channel.count_send_notification, 0);
  assert_eq!(channel.count_flush_protocol_notifications, 0);
  assert_eq!(client.count_run_message_loop_on_pause, 0);
  assert_eq!(client.count_quit_message_loop_on_pause, 0);
  assert_eq!(client.count_run_if_waiting_for_debugger, 0);

  let r = eval(scope, "1+2").unwrap();
  assert!(r.is_number());

  assert_eq!(channel.count_send_response, 1);
  assert_eq!(channel.count_send_notification, 3);
  assert_eq!(channel.count_flush_protocol_notifications, 1);
  assert_eq!(client.count_run_message_loop_on_pause, 1);
  assert_eq!(client.count_quit_message_loop_on_pause, 0);
  assert_eq!(client.count_run_if_waiting_for_debugger, 0);
  assert_ne!(client.count_generate_unique_id, 0);
}

#[test]
fn inspector_console_api_message() {
  let _setup_guard = setup();
  let isolate = &mut v8::Isolate::new(Default::default());

  use v8::inspector::*;

  struct Client {
    base: V8InspectorClientBase,
    messages: Vec<String>,
  }

  impl Client {
    fn new() -> Self {
      Self {
        base: V8InspectorClientBase::new::<Self>(),
        messages: Vec::new(),
      }
    }
  }

  impl V8InspectorClientImpl for Client {
    fn base(&self) -> &V8InspectorClientBase {
      &self.base
    }

    fn base_mut(&mut self) -> &mut V8InspectorClientBase {
      &mut self.base
    }

    fn console_api_message(
      &mut self,
      _context_group_id: i32,
      _level: i32,
      message: &StringView,
      _url: &StringView,
      _line_number: u32,
      _column_number: u32,
      _stack_trace: &mut V8StackTrace,
    ) {
      self.messages.push(message.to_string());
    }
  }

  let mut client = Client::new();
  let mut inspector = V8Inspector::create(isolate, &mut client);

  let scope = &mut v8::HandleScope::new(isolate);
  let context = v8::Context::new(scope);
  let scope = &mut v8::ContextScope::new(scope, context);

  let name = b"";
  let name_view = StringView::from(&name[..]);
  inspector.context_created(context, 1, name_view);

  let source = r#"
    console.log("one");
    console.error("two");
    console.trace("three");
  "#;
  let _ = eval(scope, source).unwrap();
  assert_eq!(client.messages, vec!["one", "two", "three"]);
}

#[test]
fn context_from_object_template() {
  let _setup_guard = setup();
  let isolate = &mut v8::Isolate::new(Default::default());
  {
    let scope = &mut v8::HandleScope::new(isolate);
    let object_templ = v8::ObjectTemplate::new(scope);
    let function_templ = v8::FunctionTemplate::new(scope, fortytwo_callback);
    let name = v8::String::new(scope, "f").unwrap();
    object_templ.set(name.into(), function_templ.into());
    let context = v8::Context::new_from_template(scope, object_templ);
    let scope = &mut v8::ContextScope::new(scope, context);
    let actual = eval(scope, "f()").unwrap();
    let expected = v8::Integer::new(scope, 42);
    assert!(expected.strict_equals(actual));
  }
}

#[test]
fn take_heap_snapshot() {
  let _setup_guard = setup();
  let isolate = &mut v8::Isolate::new(Default::default());
  {
    let scope = &mut v8::HandleScope::new(isolate);
    let context = v8::Context::new(scope);
    let scope = &mut v8::ContextScope::new(scope, context);
    let source = r#"
      {
        class Eyecatcher {}
        const eyecatchers = globalThis.eyecatchers = [];
        for (let i = 0; i < 1e4; i++) eyecatchers.push(new Eyecatcher);
      }
    "#;
    let _ = eval(scope, source).unwrap();
    let mut vec = Vec::<u8>::new();
    scope.take_heap_snapshot(|chunk| {
      vec.extend_from_slice(chunk);
      true
    });
    let s = std::str::from_utf8(&vec).unwrap();
    assert!(s.contains("Eyecatcher"));
  }
}

#[test]
fn test_prototype_api() {
  let _setup_guard = setup();
  let isolate = &mut v8::Isolate::new(Default::default());
  {
    let scope = &mut v8::HandleScope::new(isolate);
    let context = v8::Context::new(scope);
    let scope = &mut v8::ContextScope::new(scope, context);

    let obj = v8::Object::new(scope);
    let proto_obj = v8::Object::new(scope);
    let key_local: v8::Local<v8::Value> =
      v8::String::new(scope, "test_proto_key").unwrap().into();
    let value_local: v8::Local<v8::Value> =
      v8::String::new(scope, "test_proto_value").unwrap().into();
    proto_obj.set(scope, key_local, value_local);
    obj.set_prototype(scope, proto_obj.into());

    assert!(obj
      .get_prototype(scope)
      .unwrap()
      .same_value(proto_obj.into()));

    let sub_gotten = obj.get(scope, key_local).unwrap();
    assert!(sub_gotten.is_string());
    assert_eq!(sub_gotten.to_rust_string_lossy(scope), "test_proto_value");
  }
  {
    let scope = &mut v8::HandleScope::new(isolate);
    let context = v8::Context::new(scope);
    let scope = &mut v8::ContextScope::new(scope, context);

    let obj = v8::Object::new(scope);
    let null = v8::null(scope);
    obj.set_prototype(scope, null.into());

    assert!(obj.get_prototype(scope).unwrap().is_null());
  }
  {
    let scope = &mut v8::HandleScope::new(isolate);
    let context = v8::Context::new(scope);
    let scope = &mut v8::ContextScope::new(scope, context);

    let val = eval(scope, "({ __proto__: null })").unwrap();
    let obj = val.to_object(scope).unwrap();

    assert!(obj.get_prototype(scope).unwrap().is_null());
  }
}

#[test]
fn test_map_api() {
  let _setup_guard = setup();
  let isolate = &mut v8::Isolate::new(Default::default());
  {
    let scope = &mut v8::HandleScope::new(isolate);
    let context = v8::Context::new(scope);
    let scope = &mut v8::ContextScope::new(scope, context);

    let value = eval(scope, "new Map([['r','s'],['v',8]])").unwrap();
    assert!(value.is_map());
    assert!(value == v8::Local::<v8::Map>::try_from(value).unwrap());
    assert!(value != v8::Object::new(scope));
    assert_eq!(v8::Local::<v8::Map>::try_from(value).unwrap().size(), 2);
    let map = v8::Local::<v8::Map>::try_from(value).unwrap();
    assert_eq!(map.size(), 2);
    let map_array = map.as_array(scope);
    assert_eq!(map_array.length(), 4);
    assert!(
      map_array.get_index(scope, 0).unwrap()
        == v8::String::new(scope, "r").unwrap()
    );
    assert!(
      map_array.get_index(scope, 1).unwrap()
        == v8::String::new(scope, "s").unwrap()
    );
    assert!(
      map_array.get_index(scope, 2).unwrap()
        == v8::String::new(scope, "v").unwrap()
    );
    assert!(
      map_array.get_index(scope, 3).unwrap() == v8::Number::new(scope, 8f64)
    );
  }
}

#[test]
fn test_object_get_property_names() {
  let _setup_guard = setup();
  let isolate = &mut v8::Isolate::new(Default::default());

  let scope = &mut v8::HandleScope::new(isolate);

  let context = v8::Context::new(scope);
  let scope = &mut v8::ContextScope::new(scope, context);

  let js_test_str: v8::Local<v8::Value> =
    v8::String::new(scope, "test").unwrap().into();
  let js_proto_test_str: v8::Local<v8::Value> =
    v8::String::new(scope, "proto_test").unwrap().into();
  let js_test_symbol: v8::Local<v8::Value> =
    eval(scope, "Symbol('test_symbol')").unwrap();
  let js_null: v8::Local<v8::Value> = v8::null(scope).into();
  let js_sort_fn: v8::Local<v8::Function> = eval(scope, "Array.prototype.sort")
    .unwrap()
    .try_into()
    .unwrap();

  {
    let obj = v8::Object::new(scope);
    obj.set(scope, js_test_str, js_null);

    let proto_obj = v8::Object::new(scope);
    proto_obj.set(scope, js_proto_test_str, js_null);
    obj.set_prototype(scope, proto_obj.into());

    let own_props = obj.get_own_property_names(scope).unwrap();
    assert_eq!(own_props.length(), 1);
    assert!(own_props.get_index(scope, 0).unwrap() == js_test_str);

    let proto_props = proto_obj.get_own_property_names(scope).unwrap();
    assert_eq!(proto_props.length(), 1);
    assert!(proto_props.get_index(scope, 0).unwrap() == js_proto_test_str);

    let all_props = obj.get_property_names(scope).unwrap();
    js_sort_fn.call(scope, all_props.into(), &[]).unwrap();
    assert_eq!(all_props.length(), 2);
    assert!(all_props.get_index(scope, 0).unwrap() == js_proto_test_str);
    assert!(all_props.get_index(scope, 1).unwrap() == js_test_str);
  }

  {
    let obj = v8::Object::new(scope);
    obj.set(scope, js_test_str, js_null);
    obj.set(scope, js_test_symbol, js_null);

    let own_props = obj.get_own_property_names(scope).unwrap();
    assert_eq!(own_props.length(), 1);
    assert!(own_props.get_index(scope, 0).unwrap() == js_test_str);
  }
}

#[test]
fn module_snapshot() {
  let _setup_guard = setup();

  let startup_data = {
    let mut snapshot_creator = v8::SnapshotCreator::new(None);
    // TODO(ry) this shouldn't be necessary. workaround unfinished business in
    // the scope type system.
    let mut isolate = unsafe { snapshot_creator.get_owned_isolate() };
    {
      let scope = &mut v8::HandleScope::new(&mut isolate);
      let context = v8::Context::new(scope);
      let scope = &mut v8::ContextScope::new(scope, context);

      let source_text = v8::String::new(
        scope,
        "import 'globalThis.b = 42';\n\
         globalThis.a = 3",
      )
      .unwrap();
      let origin = mock_script_origin(scope, "foo.js");
      let source = v8::script_compiler::Source::new(source_text, Some(&origin));

      let module = v8::script_compiler::compile_module(scope, source).unwrap();
      assert_eq!(v8::ModuleStatus::Uninstantiated, module.get_status());

      let script_id = module.script_id();
      assert!(script_id.is_some());

      let result = module.instantiate_module(
        scope,
        compile_specifier_as_module_resolve_callback,
      );
      assert!(result.unwrap());
      assert_eq!(v8::ModuleStatus::Instantiated, module.get_status());
      assert_eq!(script_id, module.script_id());

      let result = module.evaluate(scope);
      assert!(result.is_some());
      assert_eq!(v8::ModuleStatus::Evaluated, module.get_status());
      assert_eq!(script_id, module.script_id());

      snapshot_creator.set_default_context(context);
    }
    std::mem::forget(isolate); // TODO(ry) this shouldn't be necessary.
    snapshot_creator
      .create_blob(v8::FunctionCodeHandling::Keep)
      .unwrap()
  };
  assert!(startup_data.len() > 0);
  {
    let params = v8::Isolate::create_params().snapshot_blob(startup_data);
    let isolate = &mut v8::Isolate::new(params);
    {
      let scope = &mut v8::HandleScope::new(isolate);
      let context = v8::Context::new(scope);
      let scope = &mut v8::ContextScope::new(scope, context);

      let true_val = v8::Boolean::new(scope, true).into();

      let source = v8::String::new(scope, "a === 3").unwrap();
      let script = v8::Script::compile(scope, source, None).unwrap();
      let result = script.run(scope).unwrap();
      assert!(result.same_value(true_val));

      let source = v8::String::new(scope, "b === 42").unwrap();
      let script = v8::Script::compile(scope, source, None).unwrap();
      let result = script.run(scope).unwrap();
      assert!(result.same_value(true_val));
    }
  }
}

#[derive(Default)]
struct TestHeapLimitState {
  near_heap_limit_callback_calls: u64,
}

extern "C" fn heap_limit_callback(
  data: *mut c_void,
  current_heap_limit: usize,
  _initial_heap_limit: usize,
) -> usize {
  let state = unsafe { &mut *(data as *mut TestHeapLimitState) };
  state.near_heap_limit_callback_calls += 1;
  current_heap_limit * 2 // Avoid V8 OOM.
}

// This test might fail due to a bug in V8. The upstream bug report is at
// https://bugs.chromium.org/p/v8/issues/detail?id=10843.
#[test]
fn heap_limits() {
  let _setup_guard = setup();

  let params = v8::CreateParams::default().heap_limits(0, 10 << 20); // 10 MB.
  let isolate = &mut v8::Isolate::new(params);

  let mut test_state = TestHeapLimitState::default();
  let state_ptr = &mut test_state as *mut _ as *mut c_void;
  isolate.add_near_heap_limit_callback(heap_limit_callback, state_ptr);

  let scope = &mut v8::HandleScope::new(isolate);
  let context = v8::Context::new(scope);
  let scope = &mut v8::ContextScope::new(scope, context);

  // Allocate JavaScript arrays until V8 calls the near-heap-limit callback.
  // It takes about 50-200k iterations of this loop to get to that point.
  for _ in 0..1_000_000 {
    eval(
      scope,
      r#"
        "hello 🦕 world"
          .repeat(10)
          .split("🦕")
          .map((s) => s.split(""))
          .shift()
      "#,
    )
    .unwrap();
    if test_state.near_heap_limit_callback_calls > 0 {
      break;
    }
  }
  assert_eq!(1, test_state.near_heap_limit_callback_calls);
}

#[test]
fn heap_statistics() {
  let _setup_guard = setup();

  let params = v8::CreateParams::default().heap_limits(0, 10 << 20); // 10 MB.
  let isolate = &mut v8::Isolate::new(params);

  let mut s = v8::HeapStatistics::default();
  isolate.get_heap_statistics(&mut s);

  assert!(s.used_heap_size() > 0);
  assert!(s.total_heap_size() > 0);
  assert!(s.total_heap_size() >= s.used_heap_size());
  assert!(s.heap_size_limit() > 0);
  assert!(s.heap_size_limit() >= s.total_heap_size());

  assert!(s.malloced_memory() > 0);
  assert!(s.peak_malloced_memory() > 0);
  // This invariant broke somewhere between V8 versions 8.6.337 and 8.7.25.
  // TODO(piscisaureus): re-enable this assertion when the underlying V8 bug is
  // fixed.
  // assert!(s.peak_malloced_memory() >= s.malloced_memory());

  assert_eq!(s.used_global_handles_size(), 0);
  assert_eq!(s.total_global_handles_size(), 0);
  assert_eq!(s.number_of_native_contexts(), 0);

  let scope = &mut v8::HandleScope::new(isolate);
  let context = v8::Context::new(scope);
  let scope = &mut v8::ContextScope::new(scope, context);

  let local = eval(scope, "").unwrap();
  let _global = v8::Global::new(scope, local);

  scope.get_heap_statistics(&mut s);

  assert_ne!(s.used_global_handles_size(), 0);
  assert_ne!(s.total_global_handles_size(), 0);
  assert_ne!(s.number_of_native_contexts(), 0);
}

#[test]
fn low_memory_notification() {
  let mut isolate = v8::Isolate::new(Default::default());
  isolate.low_memory_notification();
}

// Clippy thinks the return value doesn't need to be an Option, it's unaware
// of the mapping that MapFnFrom<F> does for ResolveModuleCallback.
#[allow(clippy::unnecessary_wraps)]
fn synthetic_evaluation_steps<'a>(
  context: v8::Local<'a, v8::Context>,
  module: v8::Local<v8::Module>,
) -> Option<v8::Local<'a, v8::Value>> {
  let scope = &mut unsafe { v8::CallbackScope::new(context) };
  let mut set = |name, value| {
    let name = v8::String::new(scope, name).unwrap();
    let value = v8::Number::new(scope, value).into();
    module
      .set_synthetic_module_export(scope, name, value)
      .unwrap();
  };
  set("a", 1.0);
  set("b", 2.0);

  {
    let scope = &mut v8::TryCatch::new(scope);
    let name = v8::String::new(scope, "does not exist").unwrap();
    let value = v8::undefined(scope).into();
    assert!(module.set_synthetic_module_export(scope, name, value) == None);
    assert!(scope.has_caught());
    scope.reset();
  }

  Some(v8::undefined(scope).into())
}

#[test]
fn synthetic_module() {
  let _setup_guard = setup();
  let isolate = &mut v8::Isolate::new(Default::default());

  let scope = &mut v8::HandleScope::new(isolate);

  let context = v8::Context::new(scope);
  let scope = &mut v8::ContextScope::new(scope, context);

  let export_names = [
    v8::String::new(scope, "a").unwrap(),
    v8::String::new(scope, "b").unwrap(),
  ];
  let module_name = v8::String::new(scope, "synthetic module").unwrap();
  let module = v8::Module::create_synthetic_module(
    scope,
    module_name,
    &export_names,
    synthetic_evaluation_steps,
  );
  assert!(!module.is_source_text_module());
  assert!(module.is_synthetic_module());
  assert!(module.script_id().is_none());
  assert_eq!(module.get_status(), v8::ModuleStatus::Uninstantiated);

  module
    .instantiate_module(scope, unexpected_module_resolve_callback)
    .unwrap();
  assert_eq!(module.get_status(), v8::ModuleStatus::Instantiated);

  module.evaluate(scope).unwrap();
  assert_eq!(module.get_status(), v8::ModuleStatus::Evaluated);

  let ns =
    v8::Local::<v8::Object>::try_from(module.get_module_namespace()).unwrap();

  let mut check = |name, value| {
    let name = v8::String::new(scope, name).unwrap().into();
    let value = v8::Number::new(scope, value).into();
    assert!(ns.get(scope, name).unwrap().strict_equals(value));
  };
  check("a", 1.0);
  check("b", 2.0);
}

#[allow(clippy::float_cmp)]
#[test]
fn date() {
  let time = 1_291_404_900_000.; // 2010-12-03 20:35:00 - Mees <3

  let _setup_guard = setup();
  let isolate = &mut v8::Isolate::new(Default::default());

  let scope = &mut v8::HandleScope::new(isolate);

  let context = v8::Context::new(scope);
  let scope = &mut v8::ContextScope::new(scope, context);

  let date = v8::Date::new(scope, time).unwrap();
  assert_eq!(date.value_of(), time);

  let key = v8::String::new(scope, "d").unwrap();
  context.global(scope).set(scope, key.into(), date.into());

  let result = eval(scope, "d.toISOString()").unwrap();
  let result = result.to_string(scope).unwrap();
  let result = result.to_rust_string_lossy(scope);
  assert_eq!(result, "2010-12-03T19:35:00.000Z");

  // V8 chops off fractions.
  let date = v8::Date::new(scope, std::f64::consts::PI).unwrap();
  assert_eq!(date.value_of(), 3.0);
  assert_eq!(date.number_value(scope).unwrap(), 3.0);
}

#[test]
fn symbol() {
  let _setup_guard = setup();
  let isolate = &mut v8::Isolate::new(Default::default());
  let scope = &mut v8::HandleScope::new(isolate);

  let desc = v8::String::new(scope, "a description").unwrap();

  let s = v8::Symbol::new(scope, None);
  assert!(s.description(scope) == v8::undefined(scope));

  let s = v8::Symbol::new(scope, Some(desc));
  assert!(s.description(scope) == desc);

  let s_pub = v8::Symbol::for_global(scope, desc);
  assert!(s_pub.description(scope) == desc);
  assert!(s_pub != s);

  let s_pub2 = v8::Symbol::for_global(scope, desc);
  assert!(s_pub2 != s);
  assert!(s_pub == s_pub2);

  let context = v8::Context::new(scope);
  let scope = &mut v8::ContextScope::new(scope, context);

  let s = eval(scope, "Symbol.asyncIterator").unwrap();
  assert!(s == v8::Symbol::get_async_iterator(scope));
}

#[test]
fn private() {
  let _setup_guard = setup();
  let isolate = &mut v8::Isolate::new(Default::default());
  let scope = &mut v8::HandleScope::new(isolate);

  let p = v8::Private::new(scope, None);
  assert!(p.name(scope) == v8::undefined(scope));

  let name = v8::String::new(scope, "a name").unwrap();
  let p = v8::Private::new(scope, Some(name));
  assert!(p.name(scope) == name);

  let p_api = v8::Private::for_api(scope, Some(name));
  assert!(p_api.name(scope) == name);
  assert!(p_api != p);

  let p_api2 = v8::Private::for_api(scope, Some(name));
  assert!(p_api2 != p);
  assert!(p_api == p_api2);

  let context = v8::Context::new(scope);
  let scope = &mut v8::ContextScope::new(scope, context);

  let object = v8::Object::new(scope);
  let sentinel = v8::Object::new(scope).into();
  assert!(!object.has_private(scope, p).unwrap());
  assert!(object.get_private(scope, p).unwrap().is_undefined());
  // True indicates that the operation didn't throw an
  // exception, not that it found and deleted a key.
  assert!(object.delete_private(scope, p).unwrap());
  assert!(object.set_private(scope, p, sentinel).unwrap());
  assert!(object.has_private(scope, p).unwrap());
  assert!(object
    .get_private(scope, p)
    .unwrap()
    .strict_equals(sentinel));
  assert!(object.delete_private(scope, p).unwrap());
  assert!(!object.has_private(scope, p).unwrap());
  assert!(object.get_private(scope, p).unwrap().is_undefined());
}

#[test]
fn bigint() {
  let _setup_guard = setup();
  let isolate = &mut v8::Isolate::new(Default::default());
  let scope = &mut v8::HandleScope::new(isolate);
  let context = v8::Context::new(scope);
  let scope = &mut v8::ContextScope::new(scope, context);

  let b = v8::BigInt::new_from_u64(scope, 1337);
  assert_eq!(b.u64_value(), (1337, true));

  let b = v8::BigInt::new_from_i64(scope, -1337);
  assert_eq!(b.i64_value(), (-1337, true));

  let words = vec![10, 10];
  let b = v8::BigInt::new_from_words(scope, false, &words).unwrap();
  assert_eq!(b.i64_value(), (10, false));

  let raw_b = eval(scope, "184467440737095516170n").unwrap();
  assert!(b == raw_b);

  let b = v8::BigInt::new_from_words(scope, true, &words).unwrap();
  assert_eq!(b.i64_value(), (-10, false));

  let raw_b = eval(scope, "-184467440737095516170n").unwrap();
  assert!(b == raw_b);

  let raw_b = v8::Local::<v8::BigInt>::try_from(raw_b).unwrap();

  let mut vec = Vec::new();
  vec.resize(raw_b.word_count(), 0);
  assert_eq!(raw_b.to_words_array(&mut vec), (true, &mut [10, 10][..]));

  let mut vec = Vec::new();
  vec.resize(1, 0);
  assert_eq!(raw_b.to_words_array(&mut vec), (true, &mut [10][..]));

  let mut vec = Vec::new();
  vec.resize(20, 1337);
  assert_eq!(raw_b.to_words_array(&mut vec), (true, &mut [10, 10][..]));
}

// SerDes testing
type ArrayBuffers = Vec<v8::SharedRef<v8::BackingStore>>;

struct Custom1Value<'a> {
  array_buffers: &'a mut ArrayBuffers,
}

impl<'a> Custom1Value<'a> {
  fn serializer<'s>(
    scope: &mut v8::HandleScope<'s>,
    array_buffers: &'a mut ArrayBuffers,
  ) -> v8::ValueSerializer<'a, 's> {
    v8::ValueSerializer::new(scope, Box::new(Self { array_buffers }))
  }

  fn deserializer<'s>(
    scope: &mut v8::HandleScope<'s>,
    data: &[u8],
    array_buffers: &'a mut ArrayBuffers,
  ) -> v8::ValueDeserializer<'a, 's> {
    v8::ValueDeserializer::new(scope, Box::new(Self { array_buffers }), data)
  }
}

impl<'a> v8::ValueSerializerImpl for Custom1Value<'a> {
  #[allow(unused_variables)]
  fn throw_data_clone_error<'s>(
    &mut self,
    scope: &mut v8::HandleScope<'s>,
    message: v8::Local<'s, v8::String>,
  ) {
    let error = v8::Exception::error(scope, message);
    scope.throw_exception(error);
  }

  #[allow(unused_variables)]
  fn get_shared_array_buffer_id<'s>(
    &mut self,
    scope: &mut v8::HandleScope<'s>,
    shared_array_buffer: v8::Local<'s, v8::SharedArrayBuffer>,
  ) -> Option<u32> {
    self
      .array_buffers
      .push(v8::SharedArrayBuffer::get_backing_store(
        &shared_array_buffer,
      ));
    Some((self.array_buffers.len() as u32) - 1)
  }

  fn write_host_object<'s>(
    &mut self,
    _scope: &mut v8::HandleScope<'s>,
    _object: v8::Local<'s, v8::Object>,
    value_serializer: &mut dyn v8::ValueSerializerHelper,
  ) -> Option<bool> {
    value_serializer.write_uint64(1);
    None
  }
}

impl<'a> v8::ValueDeserializerImpl for Custom1Value<'a> {
  #[allow(unused_variables)]
  fn get_shared_array_buffer_from_id<'s>(
    &mut self,
    scope: &mut v8::HandleScope<'s>,
    transfer_id: u32,
  ) -> Option<v8::Local<'s, v8::SharedArrayBuffer>> {
    let backing_store = self.array_buffers.get(transfer_id as usize).unwrap();
    Some(v8::SharedArrayBuffer::with_backing_store(
      scope,
      backing_store,
    ))
  }

  fn read_host_object<'s>(
    &mut self,
    _scope: &mut v8::HandleScope<'s>,
    value_deserializer: &mut dyn v8::ValueDeserializerHelper,
  ) -> Option<v8::Local<'s, v8::Object>> {
    let mut value = 0;
    value_deserializer.read_uint64(&mut value);
    None
  }
}

#[test]
fn value_serializer_and_deserializer() {
  use v8::ValueDeserializerHelper;
  use v8::ValueSerializerHelper;

  let _setup_guard = setup();
  let mut array_buffers = ArrayBuffers::new();
  let isolate = &mut v8::Isolate::new(Default::default());

  let scope = &mut v8::HandleScope::new(isolate);

  let context = v8::Context::new(scope);
  let scope = &mut v8::ContextScope::new(scope, context);
  let buffer;
  {
    let mut value_serializer =
      Custom1Value::serializer(scope, &mut array_buffers);
    value_serializer.write_header();
    value_serializer.write_double(55.44);
    value_serializer.write_uint32(22);
    buffer = value_serializer.release();
  }

  let mut double: f64 = 0.0;
  let mut int32: u32 = 0;
  {
    let mut value_deserializer =
      Custom1Value::deserializer(scope, &buffer, &mut array_buffers);
    assert_eq!(value_deserializer.read_header(context), Some(true));
    assert!(value_deserializer.read_double(&mut double));
    assert!(value_deserializer.read_uint32(&mut int32));

    assert!(!value_deserializer.read_uint32(&mut int32));
  }

  assert!((double - 55.44).abs() < f64::EPSILON);
  assert_eq!(int32, 22);
}

#[test]
fn value_serializer_and_deserializer_js_objects() {
  use v8::ValueDeserializerHelper;
  use v8::ValueSerializerHelper;

  let buffer;
  let mut array_buffers = ArrayBuffers::new();
  {
    let _setup_guard = setup();
    let isolate = &mut v8::Isolate::new(Default::default());

    let scope = &mut v8::HandleScope::new(isolate);

    let context = v8::Context::new(scope);
    let scope = &mut v8::ContextScope::new(scope, context);

    let objects: v8::Local<v8::Value> = eval(
      scope,
      r#"[
        undefined,
        true,
        false,
        null,
        33,
        44.444,
        99999.55434344,
        "test",
        new String("test"),
        [1, 2, 3],
        {a: "tt", add: "tsqqqss"}
      ]"#,
    )
    .unwrap();
    let mut value_serializer =
      Custom1Value::serializer(scope, &mut array_buffers);
    value_serializer.write_header();
    assert_eq!(value_serializer.write_value(context, objects), Some(true));

    buffer = value_serializer.release();
  }

  {
    let _setup_guard = setup();
    let isolate = &mut v8::Isolate::new(Default::default());

    let scope = &mut v8::HandleScope::new(isolate);

    let context = v8::Context::new(scope);
    let scope = &mut v8::ContextScope::new(scope, context);

    let mut value_deserializer =
      Custom1Value::deserializer(scope, &buffer, &mut array_buffers);
    assert_eq!(value_deserializer.read_header(context), Some(true));
    let name = v8::String::new(scope, "objects").unwrap();
    let objects: v8::Local<v8::Value> =
      value_deserializer.read_value(context).unwrap();
    drop(value_deserializer);

    context.global(scope).set(scope, name.into(), objects);

    let result: v8::Local<v8::Value> = eval(
      scope,
      r#"
      {
        const compare = [
          undefined,
          true,
          false,
          null,
          33,
          44.444,
          99999.55434344,
          "test",
          new String("test"),
          [1, 2, 3],
          {a: "tt", add: "tsqqqss"}
        ];
        let equal = true;
        function obj_isEquivalent(a, b) {
          if (a == null) return b == null;
          let aProps = Object.getOwnPropertyNames(a);
          let bProps = Object.getOwnPropertyNames(b);
          if (aProps.length != bProps.length) return false;
          for (let i = 0; i < aProps.length; i++) {
            let propName = aProps[i];
            if (a[propName] !== b[propName]) return false;
          }
          return true;
        }
        function arr_isEquivalent(a, b) {
          if (a.length != b.length) return false;
          for (let i = 0; i < Math.max(a.length, b.length); i++) {
              if (a[i] !== b[i]) return false;
          }
          return true;
        }
        objects.forEach(function (item, index) {
          let other = compare[index];
          if (Array.isArray(item)) {
            equal = equal && arr_isEquivalent(item, other);
          } else if (typeof item == 'object') {
            equal = equal && obj_isEquivalent(item, other);
          } else {
            equal = equal && (item == objects[index]);
          }
        });
        equal.toString()
      }
      "#,
    )
    .unwrap();

    let expected = v8::String::new(scope, "true").unwrap();
    assert!(expected.strict_equals(result));
  }
}

#[test]
fn value_serializer_and_deserializer_array_buffers() {
  let buffer;
  let mut array_buffers = ArrayBuffers::new();
  {
    let _setup_guard = setup();
    let isolate = &mut v8::Isolate::new(Default::default());

    let scope = &mut v8::HandleScope::new(isolate);

    let context = v8::Context::new(scope);
    let scope = &mut v8::ContextScope::new(scope, context);

    let objects: v8::Local<v8::Value> = eval(
      scope,
      r#"{
      var sab = new SharedArrayBuffer(10);
      var arr = new Int8Array(sab);
      arr[3] = 4;
      sab
      }"#,
    )
    .unwrap();
    let mut value_serializer =
      Custom1Value::serializer(scope, &mut array_buffers);
    assert_eq!(value_serializer.write_value(context, objects), Some(true));

    buffer = value_serializer.release();
  }

  {
    let _setup_guard = setup();
    let isolate = &mut v8::Isolate::new(Default::default());

    let scope = &mut v8::HandleScope::new(isolate);

    let context = v8::Context::new(scope);
    let scope = &mut v8::ContextScope::new(scope, context);

    let mut value_deserializer =
      Custom1Value::deserializer(scope, &buffer, &mut array_buffers);
    let name = v8::String::new(scope, "objects").unwrap();
    let objects: v8::Local<v8::Value> =
      value_deserializer.read_value(context).unwrap();
    drop(value_deserializer);

    context.global(scope).set(scope, name.into(), objects);

    let result: v8::Local<v8::Value> = eval(
      scope,
      r#"
      {
        var arr = new Int8Array(objects);
        arr.toString()
      }
      "#,
    )
    .unwrap();

    let expected = v8::String::new(scope, "0,0,0,4,0,0,0,0,0,0").unwrap();
    assert!(expected.strict_equals(result));
  }
}

struct Custom2Value {}

impl<'a> Custom2Value {
  fn serializer<'s>(
    scope: &mut v8::HandleScope<'s>,
  ) -> v8::ValueSerializer<'a, 's> {
    v8::ValueSerializer::new(scope, Box::new(Self {}))
  }
}

impl<'a> v8::ValueSerializerImpl for Custom2Value {
  #[allow(unused_variables)]
  fn throw_data_clone_error<'s>(
    &mut self,
    scope: &mut v8::HandleScope<'s>,
    message: v8::Local<'s, v8::String>,
  ) {
    let error = v8::Exception::error(scope, message);
    scope.throw_exception(error);
  }
}

#[test]
fn value_serializer_not_implemented() {
  let _setup_guard = setup();
  let isolate = &mut v8::Isolate::new(Default::default());

  let scope = &mut v8::HandleScope::new(isolate);

  let context = v8::Context::new(scope);
  let scope = &mut v8::ContextScope::new(scope, context);
  let scope = &mut v8::TryCatch::new(scope);

  let objects: v8::Local<v8::Value> = eval(
    scope,
    r#"{
    var sab = new SharedArrayBuffer(10);
    var arr = new Int8Array(sab);
    arr[3] = 4;
    sab
    }"#,
  )
  .unwrap();
  let mut value_serializer = Custom2Value::serializer(scope);
  assert_eq!(value_serializer.write_value(context, objects), None);

  assert!(scope.exception().is_some());
  assert!(scope.stack_trace().is_some());
  assert!(scope.message().is_some());
  assert_eq!(
    scope
      .message()
      .unwrap()
      .get(scope)
      .to_rust_string_lossy(scope),
    "Uncaught Error: #<SharedArrayBuffer> could not be cloned."
  );
}

#[test]
fn clear_kept_objects() {
  let _setup_guard = setup();

  let isolate = &mut v8::Isolate::new(Default::default());
  isolate.set_microtasks_policy(v8::MicrotasksPolicy::Explicit);

  let scope = &mut v8::HandleScope::new(isolate);
  let context = v8::Context::new(scope);
  let scope = &mut v8::ContextScope::new(scope, context);

  let step1 = r#"
    var weakrefs = [];
    for (let i = 0; i < 424242; i++) weakrefs.push(new WeakRef({ i }));
    gc();
    if (weakrefs.some(w => !w.deref())) throw "fail";
  "#;

  let step2 = r#"
    gc();
    if (weakrefs.every(w => w.deref())) throw "fail";
  "#;

  eval(scope, step1).unwrap();
  scope.clear_kept_objects();
  eval(scope, step2).unwrap();
}

#[test]
fn wasm_streaming_callback() {
  thread_local! {
    static WS: RefCell<Option<v8::WasmStreaming>> = RefCell::new(None);
  }

  let callback = |scope: &mut v8::HandleScope,
                  url: v8::Local<v8::Value>,
                  ws: v8::WasmStreaming| {
    assert_eq!("https://example.com", url.to_rust_string_lossy(scope));
    WS.with(|slot| assert!(slot.borrow_mut().replace(ws).is_none()));
  };

  let _setup_guard = setup();

  let isolate = &mut v8::Isolate::new(v8::CreateParams::default());
  isolate.set_wasm_streaming_callback(callback);

  let scope = &mut v8::HandleScope::new(isolate);
  let context = v8::Context::new(scope);
  let scope = &mut v8::ContextScope::new(scope, context);

  let script = r#"
    globalThis.result = null;
    WebAssembly
      .compileStreaming("https://example.com")
      .then(result => globalThis.result = result);
  "#;
  eval(scope, script).unwrap();
  assert!(scope.has_pending_background_tasks());

  let global = context.global(scope);
  let name = v8::String::new(scope, "result").unwrap().into();
  assert!(global.get(scope, name).unwrap().is_null());

  let mut ws = WS.with(|slot| slot.borrow_mut().take().unwrap());
  assert!(global.get(scope, name).unwrap().is_null());

  // MVP of WASM modules: contains only the magic marker and the version (1).
  ws.on_bytes_received(&[0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00]);
  assert!(global.get(scope, name).unwrap().is_null());

  ws.set_url("https://example2.com");
  assert!(global.get(scope, name).unwrap().is_null());

  ws.finish();
  assert!(!scope.has_pending_background_tasks());
  assert!(global.get(scope, name).unwrap().is_null());

  scope.perform_microtask_checkpoint();

  let result = global.get(scope, name).unwrap();
  assert!(result.is_wasm_module_object());

  let wasm_module_object: v8::Local<v8::WasmModuleObject> =
    result.try_into().unwrap();
  let compiled_wasm_module = wasm_module_object.get_compiled_module();
  assert_eq!(compiled_wasm_module.source_url(), "https://example2.com");

  let script = r#"
    globalThis.result = null;
    WebAssembly
      .compileStreaming("https://example.com")
      .catch(result => globalThis.result = result);
  "#;
  eval(scope, script).unwrap();

  let ws = WS.with(|slot| slot.borrow_mut().take().unwrap());
  assert!(global.get(scope, name).unwrap().is_null());

  let exception = v8::Object::new(scope).into(); // Can be anything.
  ws.abort(Some(exception));
  assert!(global.get(scope, name).unwrap().is_null());

  scope.perform_microtask_checkpoint();
  assert!(global.get(scope, name).unwrap().strict_equals(exception));
}

#[test]
fn unbound_script_conversion() {
  let _setup_guard = setup();
  let isolate = &mut v8::Isolate::new(Default::default());
  let scope = &mut v8::HandleScope::new(isolate);
  let unbound_script = {
    let context = v8::Context::new(scope);
    let scope = &mut v8::ContextScope::new(scope, context);
    let source = v8::String::new(scope, "'Hello ' + value").unwrap();
    let script = v8::Script::compile(scope, source, None).unwrap();
    script.get_unbound_script(scope)
  };

  {
    // Execute the script in another context.
    let context = v8::Context::new(scope);
    let scope = &mut v8::ContextScope::new(scope, context);
    let global_object = scope.get_current_context().global(scope);
    let key = v8::String::new(scope, "value").unwrap();
    let value = v8::String::new(scope, "world").unwrap();
    global_object.set(scope, key.into(), value.into());

    let script = unbound_script.bind_to_current_context(scope);
    let result = script.run(scope).unwrap();
    assert_eq!(result.to_rust_string_lossy(scope), "Hello world");
  }
}

#[test]
fn run_with_rust_allocator() {
  use std::sync::Arc;

  unsafe extern "C" fn allocate(count: &AtomicUsize, n: usize) -> *mut c_void {
    count.fetch_add(n, Ordering::SeqCst);
    Box::into_raw(vec![0u8; n].into_boxed_slice()) as *mut [u8] as *mut c_void
  }
  unsafe extern "C" fn allocate_uninitialized(
    count: &AtomicUsize,
    n: usize,
  ) -> *mut c_void {
    count.fetch_add(n, Ordering::SeqCst);
    let mut store = Vec::with_capacity(n);
    store.set_len(n);
    Box::into_raw(store.into_boxed_slice()) as *mut [u8] as *mut c_void
  }
  unsafe extern "C" fn free(count: &AtomicUsize, data: *mut c_void, n: usize) {
    count.fetch_sub(n, Ordering::SeqCst);
    Box::from_raw(std::slice::from_raw_parts_mut(data as *mut u8, n));
  }
  unsafe extern "C" fn reallocate(
    count: &AtomicUsize,
    prev: *mut c_void,
    oldlen: usize,
    newlen: usize,
  ) -> *mut c_void {
    count.fetch_add(newlen.wrapping_sub(oldlen), Ordering::SeqCst);
    let old_store =
      Box::from_raw(std::slice::from_raw_parts_mut(prev as *mut u8, oldlen));
    let mut new_store = Vec::with_capacity(newlen);
    let copy_len = oldlen.min(newlen);
    new_store.extend_from_slice(&old_store[..copy_len]);
    new_store.resize(newlen, 0u8);
    Box::into_raw(new_store.into_boxed_slice()) as *mut [u8] as *mut c_void
  }
  unsafe extern "C" fn drop(count: *const AtomicUsize) {
    Arc::from_raw(count);
  }

  let vtable: &'static v8::RustAllocatorVtable<AtomicUsize> =
    &v8::RustAllocatorVtable {
      allocate,
      allocate_uninitialized,
      free,
      reallocate,
      drop,
    };
  let count = Arc::new(AtomicUsize::new(0));

  let _setup_guard = setup();
  let create_params =
    v8::CreateParams::default().array_buffer_allocator(unsafe {
      v8::new_rust_allocator(Arc::into_raw(count.clone()), vtable)
    });
  let isolate = &mut v8::Isolate::new(create_params);

  {
    let scope = &mut v8::HandleScope::new(isolate);
    let context = v8::Context::new(scope);
    let scope = &mut v8::ContextScope::new(scope, context);
    let source = v8::String::new(
      scope,
      r#"
        for(let i = 0; i < 10; i++) new ArrayBuffer(1024 * i);
        "OK";
      "#,
    )
    .unwrap();
    let script = v8::Script::compile(scope, source, None).unwrap();
    let result = script.run(scope).unwrap();
    assert_eq!(result.to_rust_string_lossy(scope), "OK");
  }
  let mut stats = v8::HeapStatistics::default();
  isolate.get_heap_statistics(&mut stats);
  let count_loaded = count.load(Ordering::SeqCst);
  assert!(count_loaded > 0);
  assert!(count_loaded <= stats.external_memory());

  // Force a GC.
  isolate.low_memory_notification();
  let count_loaded = count.load(Ordering::SeqCst);
  assert_eq!(count_loaded, 0);
}

#[test]
fn oom_callback() {
  extern "C" fn oom_handler(_: *const std::os::raw::c_char, _: bool) {
    unreachable!()
  }

  let _setup_guard = setup();
  let params = v8::CreateParams::default().heap_limits(0, 1048576 * 8);
  let isolate = &mut v8::Isolate::new(params);
  isolate.set_oom_error_handler(oom_handler);

  // Don't attempt to trigger the OOM callback since we don't have a safe way to
  // recover from it.
}

#[test]
fn prepare_stack_trace_callback() {
  thread_local! {
    static SITES: RefCell<Option<v8::Global<v8::Array>>> = RefCell::new(None);
  }

  let script = r#"
    function g() { throw new Error("boom") }
    function f() { g() }
    try {
      f()
    } catch (e) {
      e.stack
    }
  "#;

  let _setup_guard = setup();
  let isolate = &mut v8::Isolate::new(Default::default());
  isolate.set_prepare_stack_trace_callback(callback);

  let scope = &mut v8::HandleScope::new(isolate);
  let context = v8::Context::new(scope);
  let scope = &mut v8::ContextScope::new(scope, context);
  let scope = &mut v8::TryCatch::new(scope);

  let result = eval(scope, script).unwrap();
  assert_eq!(Some(42), result.uint32_value(scope));

  let sites = SITES.with(|slot| slot.borrow_mut().take()).unwrap();
  let sites = v8::Local::new(scope, sites);
  assert_eq!(3, sites.length());

  let scripts = [
    r#"
      if ("g" !== site.getFunctionName()) throw "fail";
      if (2 !== site.getLineNumber()) throw "fail";
    "#,
    r#"
      if ("f" !== site.getFunctionName()) throw "fail";
      if (3 !== site.getLineNumber()) throw "fail";
    "#,
    r#"
      if (null !== site.getFunctionName()) throw "fail";
      if (5 !== site.getLineNumber()) throw "fail";
    "#,
  ];

  let global = context.global(scope);
  let name = v8::String::new(scope, "site").unwrap().into();

  for i in 0..3 {
    let site = sites.get_index(scope, i).unwrap();
    global.set(scope, name, site).unwrap();
    let script = scripts[i as usize];
    let result = eval(scope, script);
    assert!(result.is_some());
  }

  fn callback<'s>(
    scope: &mut v8::HandleScope<'s>,
    error: v8::Local<v8::Value>,
    sites: v8::Local<v8::Array>,
  ) -> v8::Local<'s, v8::Value> {
    let message = v8::Exception::create_message(scope, error);
    let actual = message.get(scope).to_rust_string_lossy(scope);
    assert_eq!(actual, "Uncaught Error: boom");

    SITES.with(|slot| {
      let mut slot = slot.borrow_mut();
      assert!(slot.is_none());
      *slot = Some(v8::Global::new(scope, sites));
    });

    v8::Integer::new(scope, 42).into()
  }
}

#[test]
fn icu_date() {
  let _setup_guard = setup();
  let isolate = &mut v8::Isolate::new(Default::default());
  {
    let scope = &mut v8::HandleScope::new(isolate);
    let context = v8::Context::new(scope);
    let scope = &mut v8::ContextScope::new(scope, context);
    let source = r#"
      (new Date(Date.UTC(2020, 5, 26, 7, 0, 0))).toLocaleString("de-DE", {
        weekday: "long",
        year: "numeric",
        month: "long",
        day: "numeric",
      });
    "#;
    let value = eval(scope, source).unwrap();
    let date_de_val = v8::String::new(scope, "Freitag, 26. Juni 2020").unwrap();
    assert!(value.is_string());
    assert!(value.strict_equals(date_de_val.into()));
  }
}

#[test]
fn icu_set_common_data_fail() {
  assert!(v8::icu::set_common_data_69(&[1, 2, 3]).is_err());
}

#[test]
fn icu_format() {
  let _setup_guard = setup();
  let isolate = &mut v8::Isolate::new(Default::default());
  {
    let scope = &mut v8::HandleScope::new(isolate);
    let context = v8::Context::new(scope);
    let scope = &mut v8::ContextScope::new(scope, context);
    let source = r#"
      new Intl.NumberFormat("ja-JP", { style: "currency", currency: "JPY" }).format(
        1230000,
      );
    "#;
    let value = eval(scope, source).unwrap();
    let currency_jpy_val = v8::String::new(scope, "￥1,230,000").unwrap();
    assert!(value.is_string());
    assert!(value.strict_equals(currency_jpy_val.into()));
  }
}

#[test]
fn icu_collator() {
  let _setup_guard = setup();
  let isolate = &mut v8::Isolate::new(Default::default());
  let scope = &mut v8::HandleScope::new(isolate);
  let context = v8::Context::new(scope);
  let scope = &mut v8::ContextScope::new(scope, context);
  let source = v8::String::new(scope, "new Intl.Collator('en-US')").unwrap();
  let script = v8::Script::compile(scope, source, None).unwrap();
  assert!(script.run(scope).is_some());
}

fn create_module<'s>(
  scope: &mut v8::HandleScope<'s, v8::Context>,
  source: &str,
  code_cache: Option<v8::UniqueRef<v8::CachedData>>,
  options: v8::script_compiler::CompileOptions,
) -> v8::Local<'s, v8::Module> {
  let source = v8::String::new(scope, source).unwrap();
  let resource_name = v8::String::new(scope, "<resource>").unwrap();
  let source_map_url = v8::undefined(scope);
  let script_origin = v8::ScriptOrigin::new(
    scope,
    resource_name.into(),
    0,
    0,
    false,
    0,
    source_map_url.into(),
    false,
    false,
    true,
  );
  let source = match code_cache {
    Some(x) => v8::script_compiler::Source::new_with_cached_data(
      source,
      Some(&script_origin),
      x,
    ),
    None => v8::script_compiler::Source::new(source, Some(&script_origin)),
  };
  let module = v8::script_compiler::compile_module2(
    scope,
    source,
    options,
    v8::script_compiler::NoCacheReason::NoReason,
  )
  .unwrap();
  module
}

fn create_unbound_module_script<'s>(
  scope: &mut v8::HandleScope<'s, v8::Context>,
  source: &str,
  code_cache: Option<v8::UniqueRef<v8::CachedData>>,
) -> v8::Local<'s, v8::UnboundModuleScript> {
  let module = create_module(
    scope,
    source,
    code_cache,
    v8::script_compiler::CompileOptions::NoCompileOptions,
  );
  module.get_unbound_module_script(scope)
}

#[test]
fn unbound_module_script_conversion() {
  let _setup_guard = setup();
  let isolate = &mut v8::Isolate::new(Default::default());
  let scope = &mut v8::HandleScope::new(isolate);
  let context = v8::Context::new(scope);
  let mut scope = v8::ContextScope::new(scope, context);
  create_unbound_module_script(&mut scope, "'Hello ' + value", None);
}

#[test]
fn code_cache() {
  fn resolve_callback<'a>(
    _context: v8::Local<'a, v8::Context>,
    _specifier: v8::Local<'a, v8::String>,
    _import_assertions: v8::Local<'a, v8::FixedArray>,
    _referrer: v8::Local<'a, v8::Module>,
  ) -> Option<v8::Local<'a, v8::Module>> {
    None
  }

  const CODE: &str = "export const hello = 'world';";
  let _setup_guard = setup();

  let code_cache = {
    let isolate = &mut v8::Isolate::new(Default::default());
    let scope = &mut v8::HandleScope::new(isolate);
    let context = v8::Context::new(scope);
    let mut scope = v8::ContextScope::new(scope, context);
    let unbound_module_script =
      create_unbound_module_script(&mut scope, CODE, None);
    unbound_module_script.create_code_cache().unwrap().to_vec()
  };

  let isolate = &mut v8::Isolate::new(Default::default());
  let scope = &mut v8::HandleScope::new(isolate);
  let context = v8::Context::new(scope);
  let mut scope = v8::ContextScope::new(scope, context);
  let module = create_module(
    &mut scope,
    CODE,
    Some(v8::CachedData::new(&code_cache)),
    v8::script_compiler::CompileOptions::ConsumeCodeCache,
  );
  let mut scope = v8::HandleScope::new(&mut scope);
  module
    .instantiate_module(&mut scope, resolve_callback)
    .unwrap();
  module.evaluate(&mut scope).unwrap();
  let top =
    v8::Local::<v8::Object>::try_from(module.get_module_namespace()).unwrap();

  let key = v8::String::new(&mut scope, "hello").unwrap();
  let value =
    v8::Local::<v8::String>::try_from(top.get(&mut scope, key.into()).unwrap())
      .unwrap();
  assert_eq!(&value.to_rust_string_lossy(&mut scope), "world");
}

#[test]
fn eager_compile_script() {
  let _setup_guard = setup();
  let isolate = &mut v8::Isolate::new(Default::default());
  let scope = &mut v8::HandleScope::new(isolate);
  let context = v8::Context::new(scope);
  let scope = &mut v8::ContextScope::new(scope, context);

  let code = v8::String::new(scope, "1 + 1").unwrap();
  let source = v8::script_compiler::Source::new(code, None);
  let script = v8::script_compiler::compile(
    scope,
    source,
    v8::script_compiler::CompileOptions::EagerCompile,
    v8::script_compiler::NoCacheReason::NoReason,
  )
  .unwrap();
  let ret = script.run(scope).unwrap();
  assert_eq!(ret.uint32_value(scope).unwrap(), 2);
}

#[test]
fn code_cache_script() {
  const CODE: &str = "1 + 1";
  let _setup_guard = setup();
  let code_cache = {
    let isolate = &mut v8::Isolate::new(Default::default());
    let scope = &mut v8::HandleScope::new(isolate);
    let context = v8::Context::new(scope);
    let scope = &mut v8::ContextScope::new(scope, context);

    let code = v8::String::new(scope, CODE).unwrap();
    let source = v8::script_compiler::Source::new(code, None);
    let script = v8::script_compiler::compile_unbound_script(
      scope,
      source,
      v8::script_compiler::CompileOptions::EagerCompile,
      v8::script_compiler::NoCacheReason::NoReason,
    )
    .unwrap();
    script.create_code_cache().unwrap().to_vec()
  };

  let isolate = &mut v8::Isolate::new(Default::default());
  let scope = &mut v8::HandleScope::new(isolate);
  let context = v8::Context::new(scope);
  let scope = &mut v8::ContextScope::new(scope, context);

  let code = v8::String::new(scope, CODE).unwrap();
  let source = v8::script_compiler::Source::new_with_cached_data(
    code,
    None,
    v8::CachedData::new(&code_cache),
  );
  let script = v8::script_compiler::compile(
    scope,
    source,
    v8::script_compiler::CompileOptions::ConsumeCodeCache,
    v8::script_compiler::NoCacheReason::NoReason,
  )
  .unwrap();
  let ret = script.run(scope).unwrap();
  assert_eq!(ret.uint32_value(scope).unwrap(), 2);
}

#[test]
fn compile_function_in_context() {
  let _setup_guard = setup();
  let isolate = &mut v8::Isolate::new(Default::default());
  let scope = &mut v8::HandleScope::new(isolate);
  let context = v8::Context::new(scope);
  let scope = &mut v8::ContextScope::new(scope, context);

  let x = v8::Integer::new(scope, 42);
  let y = v8::Integer::new(scope, 1337);

  let argument = v8::String::new(scope, "x").unwrap();
  let extension = v8::Object::new(scope);
  let name = v8::String::new(scope, "y").unwrap();
  extension.set(scope, name.into(), y.into()).unwrap();

  let source = v8::String::new(scope, "return x * y").unwrap();
  let source = v8::script_compiler::Source::new(source, None);
  let function = v8::script_compiler::compile_function_in_context(
    scope,
    source,
    &[argument],
    &[extension],
    v8::script_compiler::CompileOptions::NoCompileOptions,
    v8::script_compiler::NoCacheReason::NoReason,
  )
  .unwrap();

  let undefined = v8::undefined(scope).into();
  let result = function.call(scope, undefined, &[x.into()]).unwrap();
  assert!(result.is_int32());
  assert_eq!(42 * 1337, result.int32_value(scope).unwrap());
}

#[test]
fn external_strings() {
  let _setup_guard = setup();
  let isolate = &mut v8::Isolate::new(Default::default());
  let scope = &mut v8::HandleScope::new(isolate);
  let context = v8::Context::new(scope);
  let scope = &mut v8::ContextScope::new(scope, context);

  // Parse JSON from an external string
  let json_static = b"{\"a\": 1, \"b\": 2}";
  let json_external =
    v8::String::new_external_onebyte_static(scope, json_static).unwrap();
  let maybe_value = v8::json::parse(scope, json_external);
  assert!(maybe_value.is_some());
  // Check length
  assert!(json_external.length() == 16);
  // Externality checks
  assert!(json_external.is_external());
  assert!(json_external.is_external_onebyte());
  assert!(!json_external.is_external_twobyte());
  assert!(json_external.is_onebyte());
  assert!(json_external.contains_only_onebyte());

  // In & out
  let hello =
    v8::String::new_external_onebyte_static(scope, b"hello world").unwrap();
  let rust_str = hello.to_rust_string_lossy(scope);
  assert_eq!(rust_str, "hello world");
  // Externality checks
  assert!(hello.is_external());
  assert!(hello.is_external_onebyte());
  assert!(!hello.is_external_twobyte());
  assert!(hello.is_onebyte());
  assert!(hello.contains_only_onebyte());

  // Two-byte static
  let two_byte = v8::String::new_external_twobyte_static(
    scope,
    &[0xDD95, 0x0020, 0xD83E, 0xDD95],
  )
  .unwrap();
  let rust_str = two_byte.to_rust_string_lossy(scope);
  assert_eq!(rust_str, "\u{FFFD} 🦕");
  assert!(two_byte.length() == 4);
  // Externality checks
  assert!(two_byte.is_external());
  assert!(!two_byte.is_external_onebyte());
  assert!(two_byte.is_external_twobyte());
  assert!(!two_byte.is_onebyte());
  assert!(!two_byte.contains_only_onebyte());

  // two-byte "internal" test
  let gradients = v8::String::new(scope, "∇gradients").unwrap();
  assert!(!gradients.is_external());
  assert!(!gradients.is_external_onebyte());
  assert!(!gradients.is_external_twobyte());
  assert!(!gradients.is_onebyte());
  assert!(!gradients.contains_only_onebyte());
}

#[test]
fn counter_lookup_callback() {
  #[derive(Eq, PartialEq, Hash)]
  struct Name(*const c_char);
  struct Count(*mut i32);

  unsafe impl Send for Name {}
  unsafe impl Send for Count {}

  lazy_static! {
    static ref MAP: Arc<Mutex<HashMap<Name, Count>>> = Arc::default();
  }

  // |name| points to a static zero-terminated C string.
  extern "C" fn callback(name: *const c_char) -> *mut i32 {
    MAP
      .lock()
      .unwrap()
      .entry(Name(name))
      .or_insert_with(|| Count(Box::leak(Box::new(0))))
      .0
  }

  let _setup_guard = setup();
  let params = v8::CreateParams::default().counter_lookup_callback(callback);
  let isolate = &mut v8::Isolate::new(params);
  let scope = &mut v8::HandleScope::new(isolate);
  let context = v8::Context::new(scope);
  let scope = &mut v8::ContextScope::new(scope, context);
  let _ = eval(scope, "console.log(42);").unwrap();

  let count = MAP
    .lock()
    .unwrap()
    .iter()
    .find_map(|(name, count)| {
      let name = unsafe { CStr::from_ptr(name.0) };
      // Note: counter names start with a "c:" prefix.
      if "c:V8.TotalParseSize" == name.to_string_lossy() {
        Some(unsafe { *count.0 })
      } else {
        None
      }
    })
    .unwrap();

  assert_ne!(count, 0);
}

#[test]
fn compiled_wasm_module() {
  let _setup_guard = setup();

  let compiled_module = {
    let isolate = &mut v8::Isolate::new(Default::default());
    let scope = &mut v8::HandleScope::new(isolate);
    let context = v8::Context::new(scope);
    let scope = &mut v8::ContextScope::new(scope, context);

    let module: v8::Local<v8::WasmModuleObject> = eval(
      scope,
      r#"
        new WebAssembly.Module(Uint8Array.from([
          0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00,
          0x00, 0x07, 0x03, 0x66, 0x6F, 0x6F, 0x62, 0x61, 0x72
        ]));
      "#,
    )
    .unwrap()
    .try_into()
    .unwrap();

    module.get_compiled_module()
  };

  assert_eq!(
    compiled_module.get_wire_bytes_ref(),
    &[
      0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00, 0x00, 0x07, 0x03, 0x66,
      0x6F, 0x6F, 0x62, 0x61, 0x72
    ]
  );
  assert_eq!(compiled_module.source_url(), "wasm://wasm/3e495052");

  {
    let isolate = &mut v8::Isolate::new(Default::default());
    let scope = &mut v8::HandleScope::new(isolate);
    let context = v8::Context::new(scope);
    let scope = &mut v8::ContextScope::new(scope, context);

    let global = context.global(scope);

    let module =
      v8::WasmModuleObject::from_compiled_module(scope, &compiled_module)
        .unwrap();

    let key = v8::String::new(scope, "module").unwrap().into();
    global.set(scope, key, module.into());

    let foo_ab: v8::Local<v8::ArrayBuffer> =
      eval(scope, "WebAssembly.Module.customSections(module, 'foo')[0]")
        .unwrap()
        .try_into()
        .unwrap();
    let foo_bs = foo_ab.get_backing_store();
    let foo_section = unsafe {
      std::slice::from_raw_parts(foo_bs.data() as *mut u8, foo_bs.byte_length())
    };
    assert_eq!(foo_section, b"bar");
  }
}

#[test]
fn function_names() {
  // Setup isolate
  let isolate = &mut v8::Isolate::new(Default::default());
  let scope = &mut v8::HandleScope::new(isolate);
  let context = v8::Context::new(scope);
  let scope = &mut v8::ContextScope::new(scope, context);

  // Rust function
  fn callback(
    scope: &mut v8::HandleScope,
    _args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
  ) {
    rv.set(v8::Integer::new(scope, 42).into())
  }

  // named v8 function
  {
    let key = v8::String::new(scope, "magicFn").unwrap();
    let name = v8::String::new(scope, "fooBar").unwrap();
    let tmpl = v8::FunctionTemplate::new(scope, callback);
    let func = tmpl.get_function(scope).unwrap();
    func.set_name(name);

    let global = context.global(scope);
    global.set(scope, key.into(), func.into());
    let is_42: v8::Local<v8::Boolean> =
      eval(scope, "magicFn() === 42").unwrap().try_into().unwrap();
    assert!(is_42.is_true());
    let js_str: v8::Local<v8::String> = eval(scope, "magicFn.toString()")
      .unwrap()
      .try_into()
      .unwrap();
    assert_eq!(
      js_str.to_rust_string_lossy(scope),
      "function fooBar() { [native code] }"
    );
    let v8_name = func.get_name(scope);
    assert_eq!(v8_name.to_rust_string_lossy(scope), "fooBar");
  }

  // anon v8 function
  {
    let key = v8::String::new(scope, "anonFn").unwrap();
    let tmpl = v8::FunctionTemplate::new(scope, callback);
    let func = tmpl.get_function(scope).unwrap();

    let global = context.global(scope);
    global.set(scope, key.into(), func.into());
    let is_42: v8::Local<v8::Boolean> =
      eval(scope, "anonFn() === 42").unwrap().try_into().unwrap();
    assert!(is_42.is_true());
    let js_str: v8::Local<v8::String> = eval(scope, "anonFn.toString()")
      .unwrap()
      .try_into()
      .unwrap();
    assert_eq!(
      js_str.to_rust_string_lossy(scope),
      "function () { [native code] }"
    );
    let v8_name = func.get_name(scope);
    assert_eq!(v8_name.to_rust_string_lossy(scope), "");
  }
}
