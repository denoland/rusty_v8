// Copyright 2019-2021 the Deno authors. All rights reserved. MIT license.
use once_cell::sync::Lazy;
use std::any::type_name;
use std::cell::RefCell;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::convert::{Into, TryFrom, TryInto};
use std::ffi::c_void;
use std::ffi::CStr;
use std::hash::Hash;
use std::mem::MaybeUninit;
use std::os::raw::c_char;
use std::ptr::{addr_of, NonNull};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::sync::Mutex;
use v8::fast_api::CType;
use v8::fast_api::Type::*;
use v8::inspector::ChannelBase;
use v8::{fast_api, AccessorConfiguration};

// TODO(piscisaureus): Ideally there would be no need to import this trait.
use v8::MapFnTo;

mod setup {
  use std::sync::Once;
  use std::sync::RwLock;
  use std::sync::RwLockReadGuard;
  use std::sync::RwLockWriteGuard;

  static PROCESS_LOCK: RwLock<()> = RwLock::new(());

  /// Set up global state for a test that can run in parallel with other tests.
  pub(super) fn parallel_test() -> SetupGuard<RwLockReadGuard<'static, ()>> {
    initialize_once();
    SetupGuard::new(PROCESS_LOCK.read().unwrap())
  }

  /// Set up global state for a test that must be the only test running.
  pub(super) fn sequential_test() -> SetupGuard<RwLockWriteGuard<'static, ()>> {
    initialize_once();
    SetupGuard::new(PROCESS_LOCK.write().unwrap())
  }

  fn initialize_once() {
    static START: Once = Once::new();
    START.call_once(|| {
    assert!(v8::icu::set_common_data_72(align_data::include_aligned!(
      align_data::Align16,
      "../third_party/icu/common/icudtl.dat"
    ))
    .is_ok());
    v8::V8::set_flags_from_string(
      "--no_freeze_flags_after_init --expose_gc --harmony-import-assertions --harmony-shadow-realm --allow_natives_syntax --turbo_fast_api_calls",
    );
    v8::V8::initialize_platform(
      v8::new_default_platform(0, false).make_shared(),
    );
    v8::V8::initialize();
  });
  }

  #[must_use]
  pub(super) struct SetupGuard<G> {
    _inner: G,
  }

  impl<G> SetupGuard<G> {
    fn new(inner: G) -> Self {
      Self { _inner: inner }
    }
  }
}

#[test]
fn handle_scope_nested() {
  let _setup_guard = setup::parallel_test();
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
  let _setup_guard = setup::parallel_test();
  let isolate = &mut v8::Isolate::new(Default::default());
  {
    let scope1 = &mut v8::HandleScope::new(isolate);
    let l1 = v8::Integer::new(scope1, -123);
    let l2 = v8::Integer::new_from_unsigned(scope1, 456);
    {
      let scope2 = &mut v8::HandleScope::new(scope1);
      let l3 = v8::Number::new(scope2, 78.9);
      let l4 = v8::Local::<v8::Int32>::try_from(l1).unwrap();
      let l5 = v8::Local::<v8::Uint32>::try_from(l2).unwrap();
      assert_eq!(l1.value(), -123);
      assert_eq!(l2.value(), 456);
      assert_eq!(l3.value(), 78.9);
      assert_eq!(l4.value(), -123);
      assert_eq!(l5.value(), 456);
      assert_eq!(v8::Number::value(&l1), -123f64);
      assert_eq!(v8::Number::value(&l2), 456f64);
    }
  }
}

#[test]
fn handle_scope_non_lexical_lifetime() {
  let _setup_guard = setup::parallel_test();
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
  let _setup_guard = setup::parallel_test();
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
  {
    let g1_ptr = g1.clone().into_raw();
    let g1_reconstructed = unsafe { v8::Global::from_raw(isolate, g1_ptr) };
    assert_eq!(g1, g1_reconstructed);
  }
}

#[test]
fn global_from_into_raw() {
  let _setup_guard = setup::parallel_test();

  let isolate = &mut v8::Isolate::new(Default::default());
  let scope = &mut v8::HandleScope::new(isolate);
  let context = v8::Context::new(scope);
  let scope = &mut v8::ContextScope::new(scope, context);

  let (raw, weak) = {
    let scope = &mut v8::HandleScope::new(scope);
    let local = v8::Object::new(scope);
    let global = v8::Global::new(scope, local);

    let weak = v8::Weak::new(scope, &global);
    let raw = global.into_raw();
    (raw, weak)
  };

  scope.request_garbage_collection_for_testing(v8::GarbageCollectionType::Full);
  assert!(!weak.is_empty());

  {
    let reconstructed = unsafe { v8::Global::from_raw(scope, raw) };

    let global_from_weak = weak.to_global(scope).unwrap();
    assert_eq!(global_from_weak, reconstructed);
  }

  scope.request_garbage_collection_for_testing(v8::GarbageCollectionType::Full);
  assert!(weak.is_empty());
}

#[test]
fn local_handle_deref() {
  let _setup_guard = setup::parallel_test();
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
  let _setup_guard = setup::parallel_test();

  // Global 'g1' will be dropped _after_ the Isolate has been disposed.
  #[allow(clippy::needless_late_init)]
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
  let _setup_guard = setup::parallel_test();
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
  {
    let scope = &mut v8::HandleScope::new(isolate);
    let buffer = (0..v8::String::max_length() / 4)
      .map(|_| '\u{10348}') // UTF8: 0xF0 0x90 0x8D 0x88
      .collect::<String>();
    let local = v8::String::new_from_utf8(
      scope,
      buffer.as_bytes(),
      v8::NewStringType::Normal,
    )
    .unwrap();
    // U+10348 is 2 UTF-16 code units, which is the unit of v8::String.length().
    assert_eq!(v8::String::max_length() / 2, local.length());
    assert_eq!(buffer, local.to_rust_string_lossy(scope));

    let too_long = (0..(v8::String::max_length() / 4) + 1)
      .map(|_| '\u{10348}') // UTF8: 0xF0 0x90 0x8D 0x88
      .collect::<String>();
    let none = v8::String::new_from_utf8(
      scope,
      too_long.as_bytes(),
      v8::NewStringType::Normal,
    );
    assert!(none.is_none());
  }
  {
    let scope = &mut v8::HandleScope::new(isolate);
    let invalid_sequence_identifier = v8::String::new_from_utf8(
      scope,
      &[0xa0, 0xa1],
      v8::NewStringType::Normal,
    );
    assert!(invalid_sequence_identifier.is_some());
    let invalid_sequence_identifier = invalid_sequence_identifier.unwrap();
    assert_eq!(invalid_sequence_identifier.length(), 2);

    let invalid_3_octet_sequence = v8::String::new_from_utf8(
      scope,
      &[0xe2, 0x28, 0xa1],
      v8::NewStringType::Normal,
    );
    assert!(invalid_3_octet_sequence.is_some());
    let invalid_3_octet_sequence = invalid_3_octet_sequence.unwrap();
    assert_eq!(invalid_3_octet_sequence.length(), 3);

    let invalid_3_octet_sequence = v8::String::new_from_utf8(
      scope,
      &[0xe2, 0x82, 0x28],
      v8::NewStringType::Normal,
    );
    assert!(invalid_3_octet_sequence.is_some());
    let invalid_3_octet_sequence = invalid_3_octet_sequence.unwrap();
    assert_eq!(invalid_3_octet_sequence.length(), 2);

    let invalid_4_octet_sequence = v8::String::new_from_utf8(
      scope,
      &[0xf0, 0x28, 0x8c, 0xbc],
      v8::NewStringType::Normal,
    );
    assert!(invalid_4_octet_sequence.is_some());
    let invalid_4_octet_sequence = invalid_4_octet_sequence.unwrap();
    assert_eq!(invalid_4_octet_sequence.length(), 4);

    let invalid_4_octet_sequence = v8::String::new_from_utf8(
      scope,
      &[0xf0, 0x90, 0x28, 0xbc],
      v8::NewStringType::Normal,
    );
    assert!(invalid_4_octet_sequence.is_some());
    let invalid_4_octet_sequence = invalid_4_octet_sequence.unwrap();
    assert_eq!(invalid_4_octet_sequence.length(), 3);

    let invalid_4_octet_sequence = v8::String::new_from_utf8(
      scope,
      &[0xf0, 0x28, 0x8c, 0x28],
      v8::NewStringType::Normal,
    );
    assert!(invalid_4_octet_sequence.is_some());
    let invalid_4_octet_sequence = invalid_4_octet_sequence.unwrap();
    assert_eq!(invalid_4_octet_sequence.length(), 4);

    let valid_5_octet_sequence = v8::String::new_from_utf8(
      scope,
      &[0xf8, 0xa1, 0xa1, 0xa1, 0xa1],
      v8::NewStringType::Normal,
    );
    assert!(valid_5_octet_sequence.is_some());
    let invalid_4_octet_sequence = valid_5_octet_sequence.unwrap();
    assert_eq!(invalid_4_octet_sequence.length(), 5);

    let valid_6_octet_sequence = v8::String::new_from_utf8(
      scope,
      &[0xfc, 0xa1, 0xa1, 0xa1, 0xa1, 0xa1],
      v8::NewStringType::Normal,
    );
    assert!(valid_6_octet_sequence.is_some());
    let invalid_4_octet_sequence = valid_6_octet_sequence.unwrap();
    assert_eq!(invalid_4_octet_sequence.length(), 6);
  }
}

#[test]
#[allow(clippy::float_cmp)]
fn escapable_handle_scope() {
  let _setup_guard = setup::parallel_test();
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
  let _setup_guard = setup::parallel_test();
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
  let _setup_guard = setup::parallel_test();
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
  let _setup_guard = setup::parallel_test();
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
  let _setup_guard = setup::parallel_test();
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
  let _setup_guard = setup::parallel_test();
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

  let _setup_guard = setup::parallel_test();
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
  check_handle(scope, Some(true), v8::Object::new);
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
fn handles_from_isolate() {
  let _setup_guard = setup::parallel_test();
  let isolate = &mut v8::Isolate::new(Default::default());
  let _ = v8::null(isolate);
  let _ = v8::undefined(isolate);
  let _ = v8::Boolean::new(isolate, true);
}

#[test]
fn array_buffer() {
  let _setup_guard = setup::parallel_test();
  let isolate = &mut v8::Isolate::new(Default::default());
  {
    let scope = &mut v8::HandleScope::new(isolate);
    let context = v8::Context::new(scope);
    let scope = &mut v8::ContextScope::new(scope, context);

    let ab = v8::ArrayBuffer::new(scope, 42);
    assert_eq!(42, ab.byte_length());
    assert!(!ab.was_detached());
    assert!(ab.is_detachable());

    assert!(ab.detach(None).unwrap());
    assert_eq!(0, ab.byte_length());
    assert!(ab.was_detached());
    assert!(ab.detach(None).unwrap()); // Calling it twice should be a no-op.

    // detecting if it was detached on a zero-length ArrayBuffer should work
    let empty_ab = v8::ArrayBuffer::new(scope, 0);
    assert!(!empty_ab.was_detached());
    assert!(empty_ab.detach(None).unwrap());
    assert!(empty_ab.was_detached());

    let bs = v8::ArrayBuffer::new_backing_store(scope, 84);
    assert_eq!(84, bs.byte_length());
    assert!(!bs.is_shared());

    // SAFETY: Manually deallocating memory once V8 calls the
    // deleter callback.
    unsafe extern "C" fn backing_store_deleter_callback(
      data: *mut c_void,
      byte_length: usize,
      deleter_data: *mut c_void,
    ) {
      let slice = std::slice::from_raw_parts(data as *const u8, byte_length);
      assert_eq!(slice, &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
      assert_eq!(byte_length, 10);
      assert_eq!(deleter_data, std::ptr::null_mut());
      let layout = std::alloc::Layout::new::<[u8; 10]>();
      std::alloc::dealloc(data as *mut u8, layout);
    }

    // SAFETY: Manually allocating memory so that it will be only
    // deleted when V8 calls deleter callback.
    let data = unsafe {
      let layout = std::alloc::Layout::new::<[u8; 10]>();
      let ptr = std::alloc::alloc(layout);
      (ptr as *mut [u8; 10]).write([0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
      ptr as *mut c_void
    };
    let unique_bs = unsafe {
      v8::ArrayBuffer::new_backing_store_from_ptr(
        data,
        10,
        backing_store_deleter_callback,
        std::ptr::null_mut(),
      )
    };
    assert_eq!(10, unique_bs.byte_length());
    assert!(!unique_bs.is_shared());
    assert_eq!(unique_bs[0].get(), 0);
    assert_eq!(unique_bs[9].get(), 9);

    // From Box<[u8]>
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

    // From Vec<u8>
    let data = vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
    let unique_bs = v8::ArrayBuffer::new_backing_store_from_vec(data);
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
  let _setup_guard = setup::parallel_test();
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
  let _setup_guard = setup::parallel_test();
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

  let _setup_guard = setup::parallel_test();
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
  let _setup_guard = setup::parallel_test();
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
  let _setup_guard = setup::parallel_test();
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
  let _setup_guard = setup::parallel_test();
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
  let _setup_guard = setup::parallel_test();
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
  let _setup_guard = setup::parallel_test();
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
  let _setup_guard = setup::parallel_test();
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

// QEMU doesn't like when we spawn threads
// This works just fine on real hardware
#[cfg(not(target_os = "android"))]
#[test]
fn terminate_execution() {
  let _setup_guard = setup::parallel_test();
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
  let _setup_guard = setup::parallel_test();
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
  let _setup_guard = setup::parallel_test();
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
  let _setup_guard = setup::parallel_test();
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
  let _setup_guard = setup::parallel_test();
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
  let _setup_guard = setup::parallel_test();
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
  let _setup_guard = setup::parallel_test();
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
  let _setup_guard = setup::parallel_test();
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
  let _setup_guard = setup::parallel_test();
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
  let _setup_guard = setup::parallel_test();
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
  let _setup_guard = setup::parallel_test();
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
  let _setup_guard = setup::parallel_test();
  let isolate = &mut v8::Isolate::new(Default::default());
  {
    let scope = &mut v8::HandleScope::new(isolate);
    let object_templ = v8::ObjectTemplate::new(scope);
    let function_templ = v8::FunctionTemplate::new(scope, fortytwo_callback);
    let name = v8::String::new(scope, "f").unwrap();
    let attr = v8::READ_ONLY | v8::DONT_ENUM | v8::DONT_DELETE;
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
  let _setup_guard = setup::parallel_test();
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
fn object_template_immutable_proto() {
  let _setup_guard = setup::parallel_test();
  let isolate = &mut v8::Isolate::new(Default::default());
  {
    let scope = &mut v8::HandleScope::new(isolate);
    let object_templ = v8::ObjectTemplate::new(scope);
    object_templ.set_immutable_proto();
    let context = v8::Context::new_from_template(scope, object_templ);
    let scope = &mut v8::ContextScope::new(scope, context);
    let source = r#"
      {
        let r = 0;

        try {
          Object.setPrototypeOf(globalThis, {});
        } catch {
          r = 42;
        }

        String(r);
      }
    "#;
    let actual = eval(scope, source).unwrap();
    let expected = v8::String::new(scope, "42").unwrap();

    assert!(actual == expected);
  }
}

#[test]
fn function_template_signature() {
  let _setup_guard = setup::parallel_test();
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
  let _setup_guard = setup::parallel_test();
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
fn instance_template_with_internal_field() {
  let _setup_guard = setup::parallel_test();
  let isolate = &mut v8::Isolate::new(Default::default());
  let scope = &mut v8::HandleScope::new(isolate);
  let context = v8::Context::new(scope);
  let scope = &mut v8::ContextScope::new(scope, context);

  pub fn constructor_callback(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
  ) {
    let this = args.this();

    assert_eq!(args.holder(), this);
    assert!(args.data().is_undefined());

    assert!(this.set_internal_field(0, v8::Integer::new(scope, 42).into()));
    retval.set(this.into())
  }

  let function_templ = v8::FunctionTemplate::new(scope, constructor_callback);
  let instance_templ = function_templ.instance_template(scope);
  instance_templ.set_internal_field_count(1);

  let name = v8::String::new(scope, "WithInternalField").unwrap();
  let val = function_templ.get_function(scope).unwrap();
  context.global(scope).set(scope, name.into(), val.into());

  let new_instance = eval(scope, "new WithInternalField()").unwrap();
  let internal_field = new_instance
    .to_object(scope)
    .unwrap()
    .get_internal_field(scope, 0)
    .unwrap();

  assert_eq!(internal_field.integer_value(scope).unwrap(), 42);
}

#[test]
fn object_template_set_accessor() {
  let _setup_guard = setup::parallel_test();
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

      assert_eq!(args.holder(), this);
      assert!(args.data().is_undefined());
      assert!(!args.should_throw_on_error());

      let expected_key = v8::String::new(scope, "key").unwrap();
      assert!(key.strict_equals(expected_key.into()));

      rv.set(this.get_internal_field(scope, 0).unwrap());
    };

    let setter = |scope: &mut v8::HandleScope,
                  key: v8::Local<v8::Name>,
                  value: v8::Local<v8::Value>,
                  args: v8::PropertyCallbackArguments| {
      let this = args.this();

      assert_eq!(args.holder(), this);
      assert!(args.data().is_undefined());
      assert!(!args.should_throw_on_error());

      let expected_key = v8::String::new(scope, "key").unwrap();
      assert!(key.strict_equals(expected_key.into()));

      assert!(value.is_int32());
      assert!(this.set_internal_field(0, value));
    };

    let getter_with_data =
      |scope: &mut v8::HandleScope,
       key: v8::Local<v8::Name>,
       args: v8::PropertyCallbackArguments,
       mut rv: v8::ReturnValue| {
        let this = args.this();

        assert_eq!(args.holder(), this);
        assert!(args.data().is_string());
        assert!(!args.should_throw_on_error());
        assert_eq!(args.data().to_rust_string_lossy(scope), "data");

        let expected_key = v8::String::new(scope, "key").unwrap();
        assert!(key.strict_equals(expected_key.into()));

        rv.set(this.get_internal_field(scope, 0).unwrap());
      };

    let setter_with_data =
      |scope: &mut v8::HandleScope,
       key: v8::Local<v8::Name>,
       value: v8::Local<v8::Value>,
       args: v8::PropertyCallbackArguments| {
        let this = args.this();

        assert_eq!(args.holder(), this);
        assert!(args.data().is_string());
        assert!(!args.should_throw_on_error());
        assert_eq!(args.data().to_rust_string_lossy(scope), "data");

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

    // Getter + setter + data

    let templ = v8::ObjectTemplate::new(scope);
    templ.set_internal_field_count(1);
    let data = v8::String::new(scope, "data").unwrap();
    templ.set_accessor_with_configuration(
      key.into(),
      AccessorConfiguration::new(getter_with_data)
        .setter(setter_with_data)
        .data(data.into()),
    );

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

    // Accessor property
    let getter = v8::FunctionTemplate::new(scope, fortytwo_callback);
    fn property_setter(
      scope: &mut v8::HandleScope,
      args: v8::FunctionCallbackArguments,
      _: v8::ReturnValue,
    ) {
      let this = args.this();

      assert_eq!(args.holder(), this);
      assert!(args.data().is_undefined());

      let ret = v8::Integer::new(scope, 69);
      assert!(this.set_internal_field(0, ret.into()));
    }

    let setter = v8::FunctionTemplate::new(scope, property_setter);

    let templ = v8::ObjectTemplate::new(scope);
    templ.set_internal_field_count(1);

    // Getter
    let key = v8::String::new(scope, "key1").unwrap();
    templ.set_accessor_property(
      key.into(),
      Some(getter),
      None,
      v8::PropertyAttribute::default(),
    );

    // Setter
    let key = v8::String::new(scope, "key2").unwrap();
    templ.set_accessor_property(
      key.into(),
      None,
      Some(setter),
      v8::PropertyAttribute::default(),
    );

    let obj = templ.new_instance(scope).unwrap();
    let int = v8::Integer::new(scope, 42);
    obj.set_internal_field(0, int.into());
    scope.get_current_context().global(scope).set(
      scope,
      name.into(),
      obj.into(),
    );
    assert!(eval(scope, "obj.key1").unwrap().strict_equals(int.into()));
    assert!(eval(scope, "obj.key2 = 123; obj.key2")
      .unwrap()
      .is_undefined());
  }
}

#[test]
fn object_template_set_named_property_handler() {
  let _setup_guard = setup::parallel_test();
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

      assert_eq!(args.holder(), this);
      assert!(args.data().is_undefined());
      assert!(!args.should_throw_on_error());

      let expected_key = v8::String::new(scope, "key").unwrap();
      assert!(key.strict_equals(expected_key.into()));

      rv.set(this.get_internal_field(scope, 0).unwrap());
    };

    let setter = |scope: &mut v8::HandleScope,
                  key: v8::Local<v8::Name>,
                  value: v8::Local<v8::Value>,
                  args: v8::PropertyCallbackArguments| {
      let this = args.this();

      assert_eq!(args.holder(), this);
      assert!(args.data().is_undefined());
      assert!(!args.should_throw_on_error());

      let expected_key = v8::String::new(scope, "key").unwrap();
      assert!(key.strict_equals(expected_key.into()));

      assert!(value.is_int32());
      assert!(this.set_internal_field(0, value));
    };

    let query = |scope: &mut v8::HandleScope,
                 key: v8::Local<v8::Name>,
                 args: v8::PropertyCallbackArguments,
                 mut rv: v8::ReturnValue| {
      let this = args.this();

      assert_eq!(args.holder(), this);
      assert!(args.data().is_undefined());
      assert!(!args.should_throw_on_error());

      let expected_key = v8::String::new(scope, "key").unwrap();
      assert!(key.strict_equals(expected_key.into()));
      //PropertyAttribute::READ_ONLY
      rv.set_int32(1);
      let expected_value = v8::Integer::new(scope, 42);
      assert!(this
        .get_internal_field(scope, 0)
        .unwrap()
        .strict_equals(expected_value.into()));
    };
    let deleter = |scope: &mut v8::HandleScope,
                   key: v8::Local<v8::Name>,
                   _args: v8::PropertyCallbackArguments,
                   mut rv: v8::ReturnValue| {
      let expected_key = v8::String::new(scope, "key").unwrap();
      assert!(key.strict_equals(expected_key.into()));

      rv.set_bool(true);
    };

    let enumerator = |scope: &mut v8::HandleScope,
                      args: v8::PropertyCallbackArguments,
                      mut rv: v8::ReturnValue| {
      let this = args.this();

      assert_eq!(args.holder(), this);
      assert!(args.data().is_undefined());
      assert!(!args.should_throw_on_error());

      // Validate is the current object.
      let expected_value = v8::Integer::new(scope, 42);
      assert!(this
        .get_internal_field(scope, 0)
        .unwrap()
        .strict_equals(expected_value.into()));

      let key = v8::String::new(scope, "key").unwrap();
      let result = v8::Array::new_with_elements(scope, &[key.into()]);
      rv.set(result.into());
    };

    let name = v8::String::new(scope, "obj").unwrap();

    // Lone getter
    let templ = v8::ObjectTemplate::new(scope);
    templ.set_internal_field_count(1);
    templ.set_named_property_handler(
      v8::NamedPropertyHandlerConfiguration::new().getter(getter),
    );

    let obj = templ.new_instance(scope).unwrap();
    let int = v8::Integer::new(scope, 42);
    obj.set_internal_field(0, int.into());
    scope.get_current_context().global(scope).set(
      scope,
      name.into(),
      obj.into(),
    );
    assert!(eval(scope, "obj.key").unwrap().strict_equals(int.into()));

    // Getter + setter + deleter
    let templ = v8::ObjectTemplate::new(scope);
    templ.set_internal_field_count(1);
    templ.set_named_property_handler(
      v8::NamedPropertyHandlerConfiguration::new()
        .getter(getter)
        .setter(setter)
        .deleter(deleter),
    );

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

    assert!(eval(scope, "delete obj.key").unwrap().boolean_value(scope));

    // query descriptor
    let templ = v8::ObjectTemplate::new(scope);
    templ.set_internal_field_count(1);
    templ.set_named_property_handler(
      v8::NamedPropertyHandlerConfiguration::new().query(query),
    );

    let obj = templ.new_instance(scope).unwrap();
    obj.set_internal_field(0, int.into());
    scope.get_current_context().global(scope).set(
      scope,
      name.into(),
      obj.into(),
    );
    let result =
      eval(scope, "Object.getOwnPropertyDescriptor(obj, 'key')").unwrap();
    let object = result.to_object(scope).unwrap();
    let key = v8::String::new(scope, "writable").unwrap();
    let value = object.get(scope, key.into()).unwrap();

    let non_writable = v8::Boolean::new(scope, false);
    assert!(value.strict_equals(non_writable.into()));

    //enumerator
    let templ = v8::ObjectTemplate::new(scope);
    templ.set_internal_field_count(1);
    templ.set_named_property_handler(
      v8::NamedPropertyHandlerConfiguration::new().enumerator(enumerator),
    );

    let obj = templ.new_instance(scope).unwrap();
    obj.set_internal_field(0, int.into());
    scope.get_current_context().global(scope).set(
      scope,
      name.into(),
      obj.into(),
    );
    let arr = v8::Local::<v8::Array>::try_from(
      eval(scope, "Object.keys(obj)").unwrap(),
    )
    .unwrap();
    assert_eq!(arr.length(), 1);
    let index = v8::Integer::new(scope, 0);
    let result = arr.get(scope, index.into()).unwrap();
    let expected = v8::String::new(scope, "key").unwrap();
    assert!(expected.strict_equals(result))
  }
}

#[test]
fn object_template_set_indexed_property_handler() {
  let _setup_guard = setup::parallel_test();
  let isolate = &mut v8::Isolate::new(Default::default());
  let scope = &mut v8::HandleScope::new(isolate);
  let context = v8::Context::new(scope);
  let scope = &mut v8::ContextScope::new(scope, context);

  let getter = |scope: &mut v8::HandleScope,
                index: u32,
                args: v8::PropertyCallbackArguments,
                mut rv: v8::ReturnValue| {
    let this = args.this();

    assert_eq!(args.holder(), this);
    assert!(args.data().is_undefined());
    assert!(!args.should_throw_on_error());

    let expected_index = 37;
    assert!(index.eq(&expected_index));
    rv.set(this.get_internal_field(scope, 0).unwrap());
  };

  let setter = |_scope: &mut v8::HandleScope,
                index: u32,
                value: v8::Local<v8::Value>,
                args: v8::PropertyCallbackArguments| {
    let this = args.this();

    assert_eq!(args.holder(), this);
    assert!(args.data().is_undefined());
    assert!(!args.should_throw_on_error());

    assert_eq!(index, 37);

    assert!(value.is_int32());
    assert!(this.set_internal_field(0, value));
  };

  let deleter = |_scope: &mut v8::HandleScope,
                 index: u32,
                 _args: v8::PropertyCallbackArguments,
                 mut rv: v8::ReturnValue| {
    assert_eq!(index, 37);

    rv.set_bool(false);
  };

  let enumerator = |scope: &mut v8::HandleScope,
                    args: v8::PropertyCallbackArguments,
                    mut rv: v8::ReturnValue| {
    let this = args.this();

    assert_eq!(args.holder(), this);
    assert!(args.data().is_undefined());
    assert!(!args.should_throw_on_error());

    // Validate is the current object.
    let expected_value = v8::Integer::new(scope, 42);
    assert!(this
      .get_internal_field(scope, 0)
      .unwrap()
      .strict_equals(expected_value.into()));

    let key = v8::Integer::new(scope, 37);
    let result = v8::Array::new_with_elements(scope, &[key.into()]);
    rv.set(result.into());
  };

  let name = v8::String::new(scope, "obj").unwrap();

  // Lone getter
  let templ = v8::ObjectTemplate::new(scope);
  templ.set_internal_field_count(1);
  templ.set_indexed_property_handler(
    v8::IndexedPropertyHandlerConfiguration::new().getter(getter),
  );

  let obj = templ.new_instance(scope).unwrap();
  let int = v8::Integer::new(scope, 42);
  obj.set_internal_field(0, int.into());
  scope
    .get_current_context()
    .global(scope)
    .set(scope, name.into(), obj.into());
  assert!(eval(scope, "obj[37]").unwrap().strict_equals(int.into()));

  // Getter + setter + deleter
  let templ = v8::ObjectTemplate::new(scope);
  templ.set_internal_field_count(1);
  templ.set_indexed_property_handler(
    v8::IndexedPropertyHandlerConfiguration::new()
      .getter(getter)
      .setter(setter)
      .deleter(deleter),
  );

  let obj = templ.new_instance(scope).unwrap();
  obj.set_internal_field(0, int.into());
  scope
    .get_current_context()
    .global(scope)
    .set(scope, name.into(), obj.into());
  let new_int = v8::Integer::new(scope, 9);
  eval(scope, "obj[37] = 9");
  assert!(obj
    .get_internal_field(scope, 0)
    .unwrap()
    .strict_equals(new_int.into()));

  assert!(!eval(scope, "delete obj[37]").unwrap().boolean_value(scope));

  //Enumerator
  let templ = v8::ObjectTemplate::new(scope);
  templ.set_internal_field_count(1);
  templ.set_indexed_property_handler(
    v8::IndexedPropertyHandlerConfiguration::new()
      .getter(getter)
      .enumerator(enumerator),
  );

  let obj = templ.new_instance(scope).unwrap();
  obj.set_internal_field(0, int.into());
  scope
    .get_current_context()
    .global(scope)
    .set(scope, name.into(), obj.into());

  let value = eval(
    scope,
    "
    let value = -1;
    for (const i in obj) {
      value = obj[i];
   }
   value
   ",
  )
  .unwrap();

  assert!(value.strict_equals(int.into()));
}

#[test]
fn object() {
  let _setup_guard = setup::parallel_test();
  let isolate = &mut v8::Isolate::new(Default::default());
  {
    let scope = &mut v8::HandleScope::new(isolate);
    let context = v8::Context::new(scope);
    let scope = &mut v8::ContextScope::new(scope, context);
    let null: v8::Local<v8::Value> = v8::null(scope).into();
    let n1: v8::Local<v8::Name> = v8::String::new(scope, "a").unwrap().into();
    let n2: v8::Local<v8::Name> = v8::String::new(scope, "b").unwrap().into();
    let p = v8::String::new(scope, "p").unwrap().into();
    let v1: v8::Local<v8::Value> = v8::Number::new(scope, 1.0).into();
    let v2: v8::Local<v8::Value> = v8::Number::new(scope, 2.0).into();
    let object = v8::Object::with_prototype_and_properties(
      scope,
      null,
      &[n1, n2],
      &[v1, v2],
    );
    assert!(!object.is_null_or_undefined());
    let lhs = object.get_creation_context(scope).unwrap().global(scope);
    let rhs = context.global(scope);
    assert!(lhs.strict_equals(rhs.into()));

    let object_ = v8::Object::new(scope);
    assert!(!object_.is_null_or_undefined());
    let id = object_.get_identity_hash();
    assert_eq!(id, object_.get_hash());
    assert_ne!(id, v8::Object::new(scope).get_hash());

    assert!(object.has(scope, n1.into()).unwrap());
    assert!(object.has_own_property(scope, n1).unwrap());
    let n_unused = v8::String::new(scope, "unused").unwrap();
    assert!(!object.has(scope, n_unused.into()).unwrap());
    assert!(!object.has_own_property(scope, n_unused.into()).unwrap());
    assert!(object.delete(scope, n1.into()).unwrap());
    assert!(!object.has(scope, n1.into()).unwrap());
    assert!(!object.has_own_property(scope, n1).unwrap());

    let global = context.global(scope);
    let object_string = v8::String::new(scope, "o").unwrap().into();
    global.set(scope, object_string, object.into());

    assert!(eval(scope, "Object.isExtensible(o)").unwrap().is_true());
    assert!(eval(scope, "Object.isSealed(o)").unwrap().is_false());
    assert!(eval(scope, "Object.isFrozen(o)").unwrap().is_false());

    assert!(object
      .set_integrity_level(scope, v8::IntegrityLevel::Sealed)
      .unwrap());

    assert!(eval(scope, "Object.isExtensible(o)").unwrap().is_false());
    assert!(eval(scope, "Object.isSealed(o)").unwrap().is_true());
    assert!(eval(scope, "Object.isFrozen(o)").unwrap().is_false());
    // Creating new properties is not allowed anymore
    eval(scope, "o.p = true").unwrap();
    assert!(!object.has(scope, p).unwrap());
    // Deleting properties is not allowed anymore
    eval(scope, "delete o.b").unwrap();
    assert!(object.has(scope, n2.into()).unwrap());
    // But we can still write new values.
    assert!(eval(scope, "o.b = true; o.b").unwrap().is_true());

    assert!(object
      .set_integrity_level(scope, v8::IntegrityLevel::Frozen)
      .unwrap());

    assert!(eval(scope, "Object.isExtensible(o)").unwrap().is_false());
    assert!(eval(scope, "Object.isSealed(o)").unwrap().is_true());
    assert!(eval(scope, "Object.isFrozen(o)").unwrap().is_true());
    // Creating new properties is not allowed anymore
    eval(scope, "o.p = true").unwrap();
    assert!(!object.has(scope, p).unwrap());
    // Deleting properties is not allowed anymore
    eval(scope, "delete o.b").unwrap();
    assert!(object.has(scope, n2.into()).unwrap());
    // And we can also not write new values
    assert!(eval(scope, "o.b = false; o.b").unwrap().is_true());
  }
}

#[test]
fn map() {
  let _setup_guard = setup::parallel_test();
  let isolate = &mut v8::Isolate::new(Default::default());
  {
    let scope = &mut v8::HandleScope::new(isolate);
    let context = v8::Context::new(scope);
    let scope = &mut v8::ContextScope::new(scope, context);

    let map = v8::Map::new(scope);
    assert_eq!(map.size(), 0);

    let undefined = v8::undefined(scope).into();

    {
      let key = v8::Object::new(scope).into();
      let value = v8::Integer::new(scope, 1337).into();
      assert_eq!(map.has(scope, key), Some(false));
      assert_eq!(map.get(scope, key), Some(undefined));
      assert_eq!(map.set(scope, key, value), Some(map));

      assert_eq!(map.has(scope, key), Some(true));
      assert_eq!(map.size(), 1);
      assert_eq!(map.get(scope, key), Some(value));
    }

    map.clear();
    assert_eq!(map.size(), 0);

    {
      let key = v8::String::new(scope, "key").unwrap().into();
      let value = v8::Integer::new(scope, 42).into();

      assert_eq!(map.delete(scope, key), Some(false));

      map.set(scope, key, value);
      assert_eq!(map.size(), 1);

      assert_eq!(map.delete(scope, key), Some(true));
      assert_eq!(map.size(), 0);
    }
  }
}

#[test]
fn set() {
  let _setup_guard = setup::parallel_test();
  let isolate = &mut v8::Isolate::new(Default::default());
  {
    let scope = &mut v8::HandleScope::new(isolate);
    let context = v8::Context::new(scope);
    let scope = &mut v8::ContextScope::new(scope, context);

    let set = v8::Set::new(scope);
    assert_eq!(set.size(), 0);

    {
      let key = v8::Object::new(scope).into();
      assert_eq!(set.has(scope, key), Some(false));
      assert_eq!(set.add(scope, key), Some(set));

      assert_eq!(set.has(scope, key), Some(true));
      assert_eq!(set.size(), 1);
    }

    set.clear();
    assert_eq!(set.size(), 0);

    {
      let key = v8::String::new(scope, "key").unwrap().into();

      assert_eq!(set.delete(scope, key), Some(false));

      set.add(scope, key);
      assert_eq!(set.size(), 1);

      assert_eq!(set.delete(scope, key), Some(true));
      assert_eq!(set.size(), 0);
    }
  }
}

#[test]
fn array() {
  let _setup_guard = setup::parallel_test();
  let isolate = &mut v8::Isolate::new(Default::default());
  {
    let scope = &mut v8::HandleScope::new(isolate);
    let context = v8::Context::new(scope);
    let scope = &mut v8::ContextScope::new(scope, context);
    let s1 = v8::String::new(scope, "a").unwrap();
    let s2 = v8::String::new(scope, "b").unwrap();
    let array = v8::Array::new(scope, 2);
    assert_eq!(array.length(), 2);
    let lhs = array.get_creation_context(scope).unwrap().global(scope);
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
  let _setup_guard = setup::parallel_test();
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
  let _setup_guard = setup::parallel_test();
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

      assert_eq!(args.holder(), this);
      assert!(args.data().is_undefined());
      assert!(!args.should_throw_on_error());

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
  let _setup_guard = setup::parallel_test();
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

      assert_eq!(args.holder(), this);
      assert!(args.data().is_undefined());
      assert!(!args.should_throw_on_error());

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

      assert_eq!(args.holder(), this);
      assert!(args.data().is_undefined());
      assert!(!args.should_throw_on_error());

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
fn object_set_accessor_with_setter_with_property() {
  let _setup_guard = setup::parallel_test();
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

      assert_eq!(args.holder(), this);
      assert!(args.data().is_undefined());
      assert!(!args.should_throw_on_error());

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

      assert_eq!(args.holder(), this);
      assert!(args.data().is_undefined());
      assert!(!args.should_throw_on_error());

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
    obj.set_accessor_with_configuration(
      scope,
      getter_setter_key.into(),
      AccessorConfiguration::new(getter)
        .setter(setter)
        .property_attribute(v8::READ_ONLY),
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
      42 //Since it is read only
    );

    assert_eq!(CALL_COUNT.load(Ordering::SeqCst), 1);
  }
}

#[test]
fn object_set_accessor_with_data() {
  let _setup_guard = setup::parallel_test();
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

      assert_eq!(args.holder(), this);
      assert!(args.data().is_string());
      assert!(!args.should_throw_on_error());

      let data = v8::String::new(scope, "data").unwrap();
      assert!(data.strict_equals(args.data()));

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

      assert_eq!(args.holder(), this);
      assert!(args.data().is_string());
      assert!(!args.should_throw_on_error());

      let data = v8::String::new(scope, "data").unwrap();
      assert!(data.strict_equals(args.data()));

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

    let data = v8::String::new(scope, "data").unwrap();
    obj.set_accessor_with_configuration(
      scope,
      getter_setter_key.into(),
      AccessorConfiguration::new(getter)
        .setter(setter)
        .data(data.into()),
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
  let _setup_guard = setup::parallel_test();
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
  let _setup_guard = setup::parallel_test();
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
  let _setup_guard = setup::parallel_test();
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

fn fn_callback_external(
  scope: &mut v8::HandleScope,
  args: v8::FunctionCallbackArguments,
  mut rv: v8::ReturnValue,
) {
  assert_eq!(args.length(), 0);
  let data = args.data();
  let external = v8::Local::<v8::External>::try_from(data).unwrap();
  let data =
    unsafe { std::slice::from_raw_parts(external.value() as *mut u8, 5) };
  assert_eq!(&[0, 1, 2, 3, 4], data);
  let s = v8::String::new(scope, "Hello callback!").unwrap();
  assert!(rv.get(scope).is_undefined());
  rv.set(s.into());
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

fn fn_callback_new(
  scope: &mut v8::HandleScope,
  args: v8::FunctionCallbackArguments,
  mut rv: v8::ReturnValue,
) {
  assert_eq!(args.length(), 0);
  assert!(args.new_target().is_object());
  let recv = args.this();
  let key = v8::String::new(scope, "works").unwrap();
  let value = v8::Boolean::new(scope, true);
  assert!(recv.set(scope, key.into(), value.into()).unwrap());
  assert!(rv.get(scope).is_undefined());
  rv.set(recv.into());
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
  _: &mut v8::HandleScope,
  _: v8::FunctionCallbackArguments,
  mut rv: v8::ReturnValue,
) {
  rv.set_int32(42);
}

fn data_is_true_callback(
  _scope: &mut v8::HandleScope,
  args: v8::FunctionCallbackArguments,
  _rv: v8::ReturnValue,
) {
  let data = args.data();
  assert!(data.is_true());
}

fn nested_builder<'a>(
  scope: &mut v8::HandleScope<'a>,
  args: v8::FunctionCallbackArguments<'a>,
  _: v8::ReturnValue,
) {
  let arg0 = args.get(0);
  v8::Function::builder(
    |_: &mut v8::HandleScope,
     _: v8::FunctionCallbackArguments,
     _: v8::ReturnValue| {},
  )
  .data(arg0)
  .build(scope);
}

#[test]
fn function_builder_raw() {
  let _setup_guard = setup::parallel_test();
  let isolate = &mut v8::Isolate::new(Default::default());
  {
    let scope = &mut v8::HandleScope::new(isolate);
    let context = v8::Context::new(scope);
    let scope = &mut v8::ContextScope::new(scope, context);
    let global = context.global(scope);
    let recv: v8::Local<v8::Value> = global.into();

    extern "C" fn callback(info: *const v8::FunctionCallbackInfo) {
      let info = unsafe { &*info };
      let scope = unsafe { &mut v8::CallbackScope::new(info) };
      let args =
        v8::FunctionCallbackArguments::from_function_callback_info(info);
      assert!(args.length() == 1);
      assert!(args.get(0).is_string());

      let mut rv = v8::ReturnValue::from_function_callback_info(info);
      rv.set(
        v8::String::new(scope, "Hello from function!")
          .unwrap()
          .into(),
      );
    }
    let func = v8::Function::new_raw(scope, callback).unwrap();

    let arg0 = v8::String::new(scope, "Hello").unwrap();
    let value = func.call(scope, recv, &[arg0.into()]).unwrap();
    assert!(value.is_string());
    assert_eq!(value.to_rust_string_lossy(scope), "Hello from function!");
  }
}

#[test]
fn return_value() {
  let _setup_guard = setup::parallel_test();
  let isolate = &mut v8::Isolate::new(Default::default());
  {
    let scope = &mut v8::HandleScope::new(isolate);
    let context = v8::Context::new(scope);
    let scope = &mut v8::ContextScope::new(scope, context);
    let global = context.global(scope);
    let recv: v8::Local<v8::Value> = global.into();

    // set_bool
    {
      let template = v8::FunctionTemplate::new(
        scope,
        |scope: &mut v8::HandleScope,
         args: v8::FunctionCallbackArguments,
         mut rv: v8::ReturnValue| {
          assert_eq!(args.length(), 0);
          assert!(rv.get(scope).is_undefined());
          rv.set_bool(false);
        },
      );

      let function = template
        .get_function(scope)
        .expect("Unable to create function");
      let value = function
        .call(scope, recv, &[])
        .expect("Function call failed");
      assert!(value.is_boolean());
      assert!(!value.is_true());
    }

    // set_int32
    {
      let template = v8::FunctionTemplate::new(
        scope,
        |scope: &mut v8::HandleScope,
         args: v8::FunctionCallbackArguments,
         mut rv: v8::ReturnValue| {
          assert_eq!(args.length(), 0);
          assert!(rv.get(scope).is_undefined());
          rv.set_int32(69);
        },
      );

      let function = template
        .get_function(scope)
        .expect("Unable to create function");
      let value = function
        .call(scope, recv, &[])
        .expect("Function call failed");
      assert!(value.is_int32());
      assert_eq!(value.int32_value(scope).unwrap(), 69);
    }

    // set_uint32
    {
      let template = v8::FunctionTemplate::new(
        scope,
        |scope: &mut v8::HandleScope,
         args: v8::FunctionCallbackArguments,
         mut rv: v8::ReturnValue| {
          assert_eq!(args.length(), 0);
          assert!(rv.get(scope).is_undefined());
          rv.set_uint32(69);
        },
      );

      let function = template
        .get_function(scope)
        .expect("Unable to create function");
      let value = function
        .call(scope, recv, &[])
        .expect("Function call failed");
      assert!(value.is_uint32());
      assert_eq!(value.uint32_value(scope).unwrap(), 69);
    }

    // set_null
    {
      let template = v8::FunctionTemplate::new(
        scope,
        |scope: &mut v8::HandleScope,
         args: v8::FunctionCallbackArguments,
         mut rv: v8::ReturnValue| {
          assert_eq!(args.length(), 0);
          assert!(rv.get(scope).is_undefined());
          rv.set_null();
        },
      );

      let function = template
        .get_function(scope)
        .expect("Unable to create function");
      let value = function
        .call(scope, recv, &[])
        .expect("Function call failed");
      assert!(value.is_null());
    }

    // set_undefined
    {
      let template = v8::FunctionTemplate::new(
        scope,
        |scope: &mut v8::HandleScope,
         args: v8::FunctionCallbackArguments,
         mut rv: v8::ReturnValue| {
          assert_eq!(args.length(), 0);
          assert!(rv.get(scope).is_undefined());
          rv.set_undefined();
        },
      );

      let function = template
        .get_function(scope)
        .expect("Unable to create function");
      let value = function
        .call(scope, recv, &[])
        .expect("Function call failed");
      assert!(value.is_undefined());
    }

    // set_double
    {
      let template = v8::FunctionTemplate::new(
        scope,
        |scope: &mut v8::HandleScope,
         args: v8::FunctionCallbackArguments,
         mut rv: v8::ReturnValue| {
          assert_eq!(args.length(), 0);
          assert!(rv.get(scope).is_undefined());
          rv.set_double(69.420);
        },
      );

      let function = template
        .get_function(scope)
        .expect("Unable to create function");
      let value = function
        .call(scope, recv, &[])
        .expect("Function call failed");
      assert!(value.is_number());
      assert_eq!(value.number_value(scope).unwrap(), 69.420);
    }

    // set_empty_string
    {
      let template = v8::FunctionTemplate::new(
        scope,
        |scope: &mut v8::HandleScope,
         args: v8::FunctionCallbackArguments,
         mut rv: v8::ReturnValue| {
          assert_eq!(args.length(), 0);
          assert!(rv.get(scope).is_undefined());
          rv.set_empty_string();
        },
      );

      let function = template
        .get_function(scope)
        .expect("Unable to create function");
      let value = function
        .call(scope, recv, &[])
        .expect("Function call failed");
      assert!(value.is_string());
      assert_eq!(value.to_rust_string_lossy(scope), "");
    }
  }
}

#[test]
fn function() {
  let _setup_guard = setup::parallel_test();
  let isolate = &mut v8::Isolate::new(Default::default());

  {
    let scope = &mut v8::HandleScope::new(isolate);
    let context = v8::Context::new(scope);
    let scope = &mut v8::ContextScope::new(scope, context);
    let global = context.global(scope);
    let recv: v8::Local<v8::Value> = global.into();

    // Just check that this compiles.
    v8::Function::builder(nested_builder);

    // create function using template
    let fn_template = v8::FunctionTemplate::new(scope, fn_callback);
    let function = fn_template
      .get_function(scope)
      .expect("Unable to create function");
    let lhs = function.get_creation_context(scope).unwrap().global(scope);
    let rhs = context.global(scope);
    assert!(lhs.strict_equals(rhs.into()));
    let value = function
      .call(scope, recv, &[])
      .expect("Function call failed");
    let value_str = value.to_string(scope).unwrap();
    let rust_str = value_str.to_rust_string_lossy(scope);
    assert_eq!(rust_str, "Hello callback!".to_string());

    // create function using template from a raw ptr
    let fn_template =
      v8::FunctionTemplate::new_raw(scope, fn_callback.map_fn_to());
    let function = fn_template
      .get_function(scope)
      .expect("Unable to create function");
    let lhs = function.get_creation_context(scope).unwrap().global(scope);
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

    // create function without a template from a raw ptr
    let function = v8::Function::new_raw(scope, fn_callback2.map_fn_to())
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

    // create a function with associated data from a raw ptr
    let true_data = v8::Boolean::new(scope, true);
    let function = v8::Function::builder_raw(data_is_true_callback.map_fn_to())
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

    let function = v8::Function::builder(fn_callback_new).build(scope).unwrap();
    let name = v8::String::new(scope, "f2").unwrap();
    global.set(scope, name.into(), function.into()).unwrap();
    let f2: v8::Local<v8::Object> =
      eval(scope, "new f2()").unwrap().try_into().unwrap();
    let key = v8::String::new(scope, "works").unwrap();
    let value = f2.get(scope, key.into()).unwrap();
    assert!(value.is_boolean());
    assert!(value.boolean_value(scope));
  }
}

#[test]
fn function_column_and_line_numbers() {
  let _setup_guard = setup::parallel_test();
  let isolate = &mut v8::Isolate::new(Default::default());
  {
    let scope = &mut v8::HandleScope::new(isolate);
    let context = v8::Context::new(scope);
    let scope = &mut v8::ContextScope::new(scope, context);
    let source = mock_source(
      scope,
      "google.com",
      r#"export function f(a, b) {
  return a;
}

export function anotherFunctionG(a, b) {
  return b;
}"#,
    );
    let module = v8::script_compiler::compile_module(scope, source).unwrap();
    let result =
      module.instantiate_module(scope, unexpected_module_resolve_callback);
    assert!(result.is_some());
    module.evaluate(scope).unwrap();
    assert_eq!(v8::ModuleStatus::Evaluated, module.get_status());

    let namespace = module.get_module_namespace();
    assert!(namespace.is_module_namespace_object());
    let namespace_obj = namespace.to_object(scope).unwrap();

    let f_str = v8::String::new(scope, "f").unwrap();
    let f_function_obj: v8::Local<v8::Function> = namespace_obj
      .get(scope, f_str.into())
      .unwrap()
      .try_into()
      .unwrap();
    // The column number is zero-indexed and indicates the position of the end of the name.
    assert_eq!(f_function_obj.get_script_column_number(), Some(17));
    // The line number is zero-indexed as well.
    assert_eq!(f_function_obj.get_script_line_number(), Some(0));

    let g_str = v8::String::new(scope, "anotherFunctionG").unwrap();
    let g_function_obj: v8::Local<v8::Function> = namespace_obj
      .get(scope, g_str.into())
      .unwrap()
      .try_into()
      .unwrap();
    assert_eq!(g_function_obj.get_script_column_number(), Some(32));
    assert_eq!(g_function_obj.get_script_line_number(), Some(4));

    let fn_template = v8::FunctionTemplate::new(scope, fn_callback);
    let function = fn_template
      .get_function(scope)
      .expect("Unable to create function");
    assert_eq!(function.get_script_column_number(), None);
    assert_eq!(function.get_script_line_number(), None);
  }
}

#[test]
fn constructor() {
  let _setup_guard = setup::parallel_test();
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
  let _setup_guard = setup::parallel_test();
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
  let _setup_guard = setup::parallel_test();
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
    let context = promise.get_creation_context(scope).unwrap();
    let scope = &mut v8::ContextScope::new(scope, context);
    let global = context.global(scope);
    let name = v8::String::new(scope, "hook").unwrap();
    let func = global.get(scope, name.into()).unwrap();
    let func = v8::Local::<v8::Function>::try_from(func).unwrap();
    let args = &[v8::Integer::new(scope, type_ as i32).into(), promise.into()];
    func.call(scope, global.into(), args).unwrap();
  }
  let _setup_guard = setup::parallel_test();
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
fn context_get_extras_binding_object() {
  let _setup_guard = setup::parallel_test();
  let isolate = &mut v8::Isolate::new(Default::default());
  {
    let scope = &mut v8::HandleScope::new(isolate);
    let context = v8::Context::new(scope);
    let scope = &mut v8::ContextScope::new(scope, context);
    let extras_binding = context.get_extras_binding_object(scope);
    assert!(extras_binding.is_object());
  }
}

#[test]
fn context_promise_hooks() {
  let _setup_guard = setup::parallel_test();
  let isolate = &mut v8::Isolate::new(Default::default());
  {
    let scope = &mut v8::HandleScope::new(isolate);
    let context = v8::Context::new(scope);
    let scope = &mut v8::ContextScope::new(scope, context);
    let init_hook = v8::Local::<v8::Function>::try_from(
      eval(
        scope,
        r#"
      globalThis.promises = new Set();
      function initHook(promise) {
        promises.add(promise);
      }
      initHook;
    "#,
      )
      .unwrap(),
    )
    .unwrap();
    let before_hook = v8::Local::<v8::Function>::try_from(
      eval(
        scope,
        r#"
      globalThis.promiseStack = [];
      function beforeHook(promise) {
        promiseStack.push(promise);
      }
      beforeHook;
    "#,
      )
      .unwrap(),
    )
    .unwrap();
    let after_hook = v8::Local::<v8::Function>::try_from(
      eval(
        scope,
        r#"
      function afterHook(promise) {
        const it = promiseStack.pop();
        if (it !== promise) throw new Error("unexpected promise");
      }
      afterHook;
    "#,
      )
      .unwrap(),
    )
    .unwrap();
    let resolve_hook = v8::Local::<v8::Function>::try_from(
      eval(
        scope,
        r#"
      function resolveHook(promise) {
        promises.delete(promise);
      }
      resolveHook;
    "#,
      )
      .unwrap(),
    )
    .unwrap();
    scope.set_promise_hooks(
      Some(init_hook),
      Some(before_hook),
      Some(after_hook),
      Some(resolve_hook),
    );

    let source = r#"
      function expect(expected, actual = promises.size) {
        if (actual !== expected) throw `expected ${expected}, actual ${actual}`;
      }
      expect(0);
      var p = new Promise(resolve => {
        expect(1);
        resolve();
        expect(0);
      });
      expect(0);
      new Promise(() => {});
      expect(1);

      expect(0, promiseStack.length);
      p.then(() => {
        expect(1, promiseStack.length);
      });
      promises.values().next().value
    "#;
    let promise = eval(scope, source).unwrap();
    let promise = v8::Local::<v8::Promise>::try_from(promise).unwrap();
    assert!(!promise.has_handler());
    assert_eq!(promise.state(), v8::PromiseState::Pending);

    scope.perform_microtask_checkpoint();
    let _ = eval(
      scope,
      r#"
      expect(0, promiseStack.length);
    "#,
    )
    .unwrap();
  }
}

#[test]
fn context_promise_hooks_partial() {
  let _setup_guard = setup::parallel_test();
  let isolate = &mut v8::Isolate::new(Default::default());
  {
    let scope = &mut v8::HandleScope::new(isolate);
    let context = v8::Context::new(scope);
    let scope = &mut v8::ContextScope::new(scope, context);
    let init_hook = v8::Local::<v8::Function>::try_from(
      eval(
        scope,
        r#"
      globalThis.promises = new Set();
      function initHook(promise) {
        promises.add(promise);
      }
      initHook;
    "#,
      )
      .unwrap(),
    )
    .unwrap();
    let before_hook = v8::Local::<v8::Function>::try_from(
      eval(
        scope,
        r#"
      globalThis.promiseStack = [];
      function beforeHook(promise) {
        promiseStack.push(promise);
      }
      beforeHook;
    "#,
      )
      .unwrap(),
    )
    .unwrap();
    scope.set_promise_hooks(Some(init_hook), Some(before_hook), None, None);

    let source = r#"
      function expect(expected, actual = promises.size) {
        if (actual !== expected) throw `expected ${expected}, actual ${actual}`;
      }
      expect(0);
      var p = new Promise(resolve => {
        expect(1);
        resolve();
        expect(1);
      });
      expect(1);
      new Promise(() => {});
      expect(2);

      expect(0, promiseStack.length);
      p.then(() => {
        expect(1, promiseStack.length);
      });
      promises.values().next().value
    "#;
    let promise = eval(scope, source).unwrap();
    let promise = v8::Local::<v8::Promise>::try_from(promise).unwrap();
    assert!(promise.has_handler());
    assert_eq!(promise.state(), v8::PromiseState::Fulfilled);

    scope.perform_microtask_checkpoint();
    let _ = eval(
      scope,
      r#"
      expect(1, promiseStack.length);
    "#,
    )
    .unwrap();
  }
}

#[test]
fn security_token() {
  let _setup_guard = setup::parallel_test();
  let isolate = &mut v8::Isolate::new(Default::default());
  {
    let scope = &mut v8::HandleScope::new(isolate);
    let context = v8::Context::new(scope);
    let scope = &mut v8::ContextScope::new(scope, context);

    // Define a variable in the parent context
    let global = {
      let global = context.global(scope);
      let variable_key = v8::String::new(scope, "variable").unwrap();
      let variable_value = v8::String::new(scope, "value").unwrap();
      global.set(scope, variable_key.into(), variable_value.into());
      v8::Global::new(scope, global)
    };
    // This code will try to access the variable defined in the parent context
    let source = r#"
      if (variable !== 'value') {
        throw new Error('Expected variable to be value');
      }
    "#;

    let templ = v8::ObjectTemplate::new(scope);
    let global = v8::Local::new(scope, global);
    templ.set_named_property_handler(
      v8::NamedPropertyHandlerConfiguration::new()
        .getter(
          |scope: &mut v8::HandleScope,
           key: v8::Local<v8::Name>,
           args: v8::PropertyCallbackArguments,
           mut rv: v8::ReturnValue| {
            let obj = v8::Local::<v8::Object>::try_from(args.data()).unwrap();
            if let Some(val) = obj.get(scope, key.into()) {
              rv.set(val);
            }
          },
        )
        .data(global.into()),
    );

    // Creates a child context
    {
      let security_token = context.get_security_token(scope);
      let child_context = v8::Context::new_from_template(scope, templ);
      // Without the security context, the variable can not be shared
      child_context.set_security_token(security_token);
      let child_scope = &mut v8::ContextScope::new(scope, child_context);
      let try_catch = &mut v8::TryCatch::new(child_scope);
      let result = eval(try_catch, source);
      assert!(!try_catch.has_caught());
      assert!(result.unwrap().is_undefined());
    }

    // Runs the same code but without the security token, it should fail
    {
      let child_context = v8::Context::new_from_template(scope, templ);
      let child_scope = &mut v8::ContextScope::new(scope, child_context);
      let try_catch = &mut v8::TryCatch::new(child_scope);
      let result = eval(try_catch, source);
      assert!(try_catch.has_caught());
      let exc = try_catch.exception().unwrap();
      let exc = exc.to_string(try_catch).unwrap();
      let exc = exc.to_rust_string_lossy(try_catch);
      assert!(exc.contains("no access"));
      assert!(result.is_none());
    }
  }
}

#[test]
fn allow_code_generation_from_strings() {
  let _setup_guard = setup::parallel_test();
  let isolate = &mut v8::Isolate::new(Default::default());
  {
    let scope = &mut v8::HandleScope::new(isolate);
    let context = v8::Context::new(scope);
    // The code generation is allowed by default
    assert!(context.is_code_generation_from_strings_allowed());
    // This code will try to use generation from strings
    let source = r#"
     eval("const i = 1; i")
    "#;
    {
      let scope = &mut v8::ContextScope::new(scope, context);

      let try_catch = &mut v8::TryCatch::new(scope);
      let result = eval(try_catch, source).unwrap();
      let expected = v8::Integer::new(try_catch, 1);
      assert!(expected.strict_equals(result));
      assert!(!try_catch.has_caught());
    }
    context.set_allow_generation_from_strings(false);
    assert!(!context.is_code_generation_from_strings_allowed());
    {
      let scope = &mut v8::ContextScope::new(scope, context);

      let try_catch = &mut v8::TryCatch::new(scope);
      let result = eval(try_catch, source);
      assert!(try_catch.has_caught());
      let exc = try_catch.exception().unwrap();
      let exc = exc.to_string(try_catch).unwrap();
      let exc = exc.to_rust_string_lossy(try_catch);
      assert!(exc
        .contains("Code generation from strings disallowed for this context"));
      assert!(result.is_none());
    }
  }
}

#[test]
fn allow_atomics_wait() {
  let _setup_guard = setup::parallel_test();
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
  let _setup_guard = setup::parallel_test();
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

    assert!(source.get_cached_data().is_none());

    let result = v8::script_compiler::compile_module(scope, source);
    assert!(result.is_some());
  }
}

#[test]
fn module_instantiation_failures1() {
  let _setup_guard = setup::parallel_test();
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
  let _setup_guard = setup::parallel_test();
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
fn module_stalled_top_level_await() {
  let _setup_guard = setup::parallel_test();
  let isolate = &mut v8::Isolate::new(Default::default());
  {
    let scope = &mut v8::HandleScope::new(isolate);
    let context = v8::Context::new(scope);
    let scope = &mut v8::ContextScope::new(scope, context);

    let source_text =
      v8::String::new(scope, "await new Promise((_resolve, _reject) => {});")
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

    let promise: v8::Local<v8::Promise> = result.unwrap().try_into().unwrap();
    scope.perform_microtask_checkpoint();
    assert_eq!(promise.state(), v8::PromiseState::Pending);
    let stalled = module.get_stalled_top_level_await_message(scope);
    assert_eq!(stalled.len(), 1);
    let (_module, message) = stalled[0];
    let message_str = message.get(scope);
    assert_eq!(
      message_str.to_rust_string_lossy(scope),
      "Top-level await promise never resolved"
    );
    assert_eq!(Some(1), message.get_line_number(scope));
    assert_eq!(
      message
        .get_script_resource_name(scope)
        .unwrap()
        .to_rust_string_lossy(scope),
      "foo.js"
    );
    assert_eq!(
      message
        .get_source_line(scope)
        .unwrap()
        .to_rust_string_lossy(scope),
      "await new Promise((_resolve, _reject) => {});"
    );
  }
}

#[test]
fn import_assertions() {
  let _setup_guard = setup::parallel_test();
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

  fn dynamic_import_cb<'s>(
    scope: &mut v8::HandleScope<'s>,
    _host_defined_options: v8::Local<'s, v8::Data>,
    _resource_name: v8::Local<'s, v8::Value>,
    _specifier: v8::Local<'s, v8::String>,
    import_assertions: v8::Local<'s, v8::FixedArray>,
  ) -> Option<v8::Local<'s, v8::Promise>> {
    // "type" keyword, value
    assert_eq!(import_assertions.length(), 2);
    let assert1 = import_assertions.get(scope, 0).unwrap();
    let assert1_val = v8::Local::<v8::Value>::try_from(assert1).unwrap();
    assert_eq!(assert1_val.to_rust_string_lossy(scope), "type");
    let assert2 = import_assertions.get(scope, 1).unwrap();
    let assert2_val = v8::Local::<v8::Value>::try_from(assert2).unwrap();
    assert_eq!(assert2_val.to_rust_string_lossy(scope), "json");
    None
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
  let _setup_guard = setup::parallel_test();
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
  let _setup_guard = setup::parallel_test();
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
  let _setup_guard = setup::parallel_test();
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

  let _setup_guard = setup::parallel_test();
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
  let _setup_guard = setup::parallel_test();
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
fn continuation_preserved_embedder_data() {
  let _setup_guard = setup::parallel_test();
  let isolate = &mut v8::Isolate::new(Default::default());
  {
    let scope = &mut v8::HandleScope::new(isolate);
    let context = v8::Context::new(scope);
    let scope = &mut v8::ContextScope::new(scope, context);
    let data = scope.get_continuation_preserved_embedder_data();
    assert!(data.is_undefined());

    let value = v8::String::new(scope, "hello").unwrap();
    scope.set_continuation_preserved_embedder_data(value.into());
    let data = scope.get_continuation_preserved_embedder_data();
    assert!(data.is_string());
    assert_eq!(data.to_rust_string_lossy(scope), "hello");

    eval(scope, "b = 2 + 3").unwrap();
    let data = scope.get_continuation_preserved_embedder_data();
    assert!(data.is_string());
    assert_eq!(data.to_rust_string_lossy(scope), "hello");
  }
}

#[test]
fn snapshot_creator() {
  let _setup_guard = setup::sequential_test();
  // First we create the snapshot, there is a single global variable 'a' set to
  // the value 3.
  let isolate_data_index;
  let context_data_index;
  let context_data_index_2;
  let startup_data = {
    let mut snapshot_creator = v8::Isolate::snapshot_creator(None);
    {
      let scope = &mut v8::HandleScope::new(&mut snapshot_creator);
      let context = v8::Context::new(scope);
      let scope = &mut v8::ContextScope::new(scope, context);
      eval(scope, "b = 2 + 3").unwrap();
      scope.set_default_context(context);
    }

    snapshot_creator
      .create_blob(v8::FunctionCodeHandling::Clear)
      .unwrap()
  };

  let startup_data = {
    let mut snapshot_creator =
      v8::Isolate::snapshot_creator_from_existing_snapshot(startup_data, None);
    {
      // Check that the SnapshotCreator isolate has been set up correctly.
      let _ = snapshot_creator.thread_safe_handle();

      let scope = &mut v8::HandleScope::new(&mut snapshot_creator);
      let context = v8::Context::new(scope);
      let scope = &mut v8::ContextScope::new(scope, context);
      eval(scope, "a = 1 + 2").unwrap();

      scope.set_default_context(context);

      let n1 = v8::Number::new(scope, 1.0);
      let n2 = v8::Number::new(scope, 2.0);
      let n3 = v8::Number::new(scope, 3.0);
      isolate_data_index = scope.add_isolate_data(n1);
      context_data_index = scope.add_context_data(context, n2);
      context_data_index_2 = scope.add_context_data(context, n3);
    }
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
      let result = eval(scope, "a === 3").unwrap();
      let true_val = v8::Boolean::new(scope, true).into();
      assert!(result.same_value(true_val));

      let result = eval(scope, "b === 5").unwrap();
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

#[test]
fn snapshot_creator_multiple_contexts() {
  let _setup_guard = setup::sequential_test();
  let startup_data = {
    let mut snapshot_creator = v8::Isolate::snapshot_creator(None);
    {
      let mut scope = v8::HandleScope::new(&mut snapshot_creator);
      let context = v8::Context::new(&mut scope);
      let scope = &mut v8::ContextScope::new(&mut scope, context);
      eval(scope, "globalThis.__bootstrap = { defaultContextProp: 1};")
        .unwrap();
      {
        let value =
          eval(scope, "globalThis.__bootstrap.defaultContextProp").unwrap();
        let one_val = v8::Number::new(scope, 1.0).into();
        assert!(value.same_value(one_val));
      }
      scope.set_default_context(context);
    }
    {
      let scope = &mut v8::HandleScope::new(&mut snapshot_creator);
      let context = v8::Context::new(scope);
      let scope = &mut v8::ContextScope::new(scope, context);
      eval(scope, "globalThis.__bootstrap = { context0Prop: 2 };").unwrap();
      {
        let value = eval(scope, "globalThis.__bootstrap.context0Prop").unwrap();
        let two_val = v8::Number::new(scope, 2.0).into();
        assert!(value.same_value(two_val));
      }
      assert_eq!(0, scope.add_context(context));
    }

    snapshot_creator
      .create_blob(v8::FunctionCodeHandling::Clear)
      .unwrap()
  };

  let startup_data = {
    let mut snapshot_creator =
      v8::Isolate::snapshot_creator_from_existing_snapshot(startup_data, None);
    {
      let scope = &mut v8::HandleScope::new(&mut snapshot_creator);
      let context = v8::Context::new(scope);
      let scope = &mut v8::ContextScope::new(scope, context);
      {
        let value =
          eval(scope, "globalThis.__bootstrap.defaultContextProp").unwrap();
        let one_val = v8::Number::new(scope, 1.0).into();
        assert!(value.same_value(one_val));
      }
      {
        let value = eval(scope, "globalThis.__bootstrap.context0Prop").unwrap();
        assert!(value.is_undefined());
      }
      {
        eval(scope, "globalThis.__bootstrap.defaultContextProp2 = 3;").unwrap();
        let value =
          eval(scope, "globalThis.__bootstrap.defaultContextProp2").unwrap();
        let three_val = v8::Number::new(scope, 3.0).into();
        assert!(value.same_value(three_val));
      }
      scope.set_default_context(context);
    }
    {
      let scope = &mut v8::HandleScope::new(&mut snapshot_creator);
      let context = v8::Context::from_snapshot(scope, 0).unwrap();
      let scope = &mut v8::ContextScope::new(scope, context);
      {
        let value =
          eval(scope, "globalThis.__bootstrap.defaultContextProp").unwrap();
        assert!(value.is_undefined());
      }
      {
        let value = eval(scope, "globalThis.__bootstrap.context0Prop").unwrap();
        let two_val = v8::Number::new(scope, 2.0).into();
        assert!(value.same_value(two_val));
      }
      {
        eval(scope, "globalThis.__bootstrap.context0Prop2 = 4;").unwrap();
        let value =
          eval(scope, "globalThis.__bootstrap.context0Prop2").unwrap();
        let four_val = v8::Number::new(scope, 4.0).into();
        assert!(value.same_value(four_val));
      }
      assert_eq!(scope.add_context(context), 0);
    }
    snapshot_creator
      .create_blob(v8::FunctionCodeHandling::Clear)
      .unwrap()
  };
  {
    let params = v8::Isolate::create_params().snapshot_blob(startup_data);
    let isolate = &mut v8::Isolate::new(params);
    {
      let scope = &mut v8::HandleScope::new(isolate);
      let context = v8::Context::new(scope);
      let scope = &mut v8::ContextScope::new(scope, context);
      {
        let value = eval(scope, "globalThis.__bootstrap.context0Prop").unwrap();
        assert!(value.is_undefined());
      }
      {
        let value =
          eval(scope, "globalThis.__bootstrap.context0Prop2").unwrap();
        assert!(value.is_undefined());
      }
      {
        let value =
          eval(scope, "globalThis.__bootstrap.defaultContextProp").unwrap();
        let one_val = v8::Number::new(scope, 1.0).into();
        assert!(value.same_value(one_val));
      }
      {
        let value =
          eval(scope, "globalThis.__bootstrap.defaultContextProp2").unwrap();
        let three_val = v8::Number::new(scope, 3.0).into();
        assert!(value.same_value(three_val));
      }
    }
    {
      let scope = &mut v8::HandleScope::new(isolate);
      let context = v8::Context::from_snapshot(scope, 0).unwrap();
      let scope = &mut v8::ContextScope::new(scope, context);
      {
        let value =
          eval(scope, "globalThis.__bootstrap.defaultContextProp").unwrap();
        assert!(value.is_undefined());
      }
      {
        let value =
          eval(scope, "globalThis.__bootstrap.defaultContextProp2").unwrap();
        assert!(value.is_undefined());
      }
      {
        let value = eval(scope, "globalThis.__bootstrap.context0Prop").unwrap();
        let two_val = v8::Number::new(scope, 2.0).into();
        assert!(value.same_value(two_val));
      }
      {
        let value =
          eval(scope, "globalThis.__bootstrap.context0Prop2").unwrap();
        let four_val = v8::Number::new(scope, 4.0).into();
        assert!(value.same_value(four_val));
      }
    }
  }
}

#[test]
fn external_references() {
  let _setup_guard = setup::sequential_test();
  // Allocate externals for the test.
  let external_ptr = Box::into_raw(vec![0_u8, 1, 2, 3, 4].into_boxed_slice())
    as *mut [u8] as *mut c_void;
  // Push them to the external reference table.
  let refs = v8::ExternalReferences::new(&[
    v8::ExternalReference {
      function: fn_callback.map_fn_to(),
    },
    v8::ExternalReference {
      function: fn_callback_external.map_fn_to(),
    },
    v8::ExternalReference {
      pointer: external_ptr,
    },
  ]);
  // TODO(piscisaureus): leaking the `ExternalReferences` collection shouldn't
  // be necessary. The reference needs to remain valid for the lifetime of the
  // `SnapshotCreator` or `Isolate` that uses it, which would be the case here
  // even without leaking.
  let refs: &'static v8::ExternalReferences = Box::leak(Box::new(refs));
  // First we create the snapshot, there is a single global variable 'a' set to
  // the value 3.
  let startup_data = {
    let mut snapshot_creator = v8::Isolate::snapshot_creator(Some(refs));
    {
      let scope = &mut v8::HandleScope::new(&mut snapshot_creator);
      let context = v8::Context::new(scope);
      let scope = &mut v8::ContextScope::new(scope, context);

      // create function using template
      let external = v8::External::new(scope, external_ptr);
      let fn_template = v8::FunctionTemplate::builder(fn_callback_external)
        .data(external.into())
        .build(scope);
      let function = fn_template
        .get_function(scope)
        .expect("Unable to create function");

      let global = context.global(scope);
      let key = v8::String::new(scope, "F").unwrap();
      global.set(scope, key.into(), function.into());

      scope.set_default_context(context);
    }
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
      .external_references(&**refs);
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
  let _setup_guard = setup::parallel_test();
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
  let _setup_guard = setup::parallel_test();
  let isolate = &mut v8::Isolate::new(Default::default());
  let scope = &mut v8::HandleScope::new(isolate);
  let context = v8::Context::new(scope);
  let scope = &mut v8::ContextScope::new(scope, context);

  let ab = v8::ArrayBuffer::new(scope, 8);

  let t = v8::Uint8Array::new(scope, ab, 0, 0).unwrap();
  assert!(t.is_uint8_array());
  assert_eq!(t.length(), 0);

  let t = v8::Uint8ClampedArray::new(scope, ab, 0, 0).unwrap();
  assert!(t.is_uint8_clamped_array());
  assert_eq!(t.length(), 0);

  let t = v8::Int8Array::new(scope, ab, 0, 0).unwrap();
  assert!(t.is_int8_array());
  assert_eq!(t.length(), 0);

  let t = v8::Uint16Array::new(scope, ab, 0, 0).unwrap();
  assert!(t.is_uint16_array());
  assert_eq!(t.length(), 0);

  let t = v8::Int16Array::new(scope, ab, 0, 0).unwrap();
  assert!(t.is_int16_array());
  assert_eq!(t.length(), 0);

  let t = v8::Uint32Array::new(scope, ab, 0, 0).unwrap();
  assert!(t.is_uint32_array());
  assert_eq!(t.length(), 0);

  let t = v8::Int32Array::new(scope, ab, 0, 0).unwrap();
  assert!(t.is_int32_array());
  assert_eq!(t.length(), 0);

  let t = v8::Float32Array::new(scope, ab, 0, 0).unwrap();
  assert!(t.is_float32_array());
  assert_eq!(t.length(), 0);

  let t = v8::Float64Array::new(scope, ab, 0, 0).unwrap();
  assert!(t.is_float64_array());
  assert_eq!(t.length(), 0);

  let t = v8::BigUint64Array::new(scope, ab, 0, 0).unwrap();
  assert!(t.is_big_uint64_array());
  assert_eq!(t.length(), 0);

  let t = v8::BigInt64Array::new(scope, ab, 0, 0).unwrap();
  assert!(t.is_big_int64_array());
  assert_eq!(t.length(), 0);

  // TypedArray::max_length() ought to be >= 2^30 < 2^32 in 64 bits
  #[cfg(target_pointer_width = "64")]
  assert!(((2 << 30)..(2 << 32)).contains(&v8::TypedArray::max_length()));

  // TypedArray::max_length() ought to be >= 2^28 < 2^30 in 32 bits
  #[cfg(target_pointer_width = "32")]
  assert!(((2 << 28)..(2 << 30)).contains(&v8::TypedArray::max_length()));

  // v8::ArrayBuffer::new raises a fatal if the length is > kMaxLength, so we test this behavior
  // through the JS side of things, where a non-fatal RangeError is thrown in such cases.
  {
    let scope = &mut v8::TryCatch::new(scope);
    let _ = eval(
      scope,
      &format!("new Uint8Array({})", v8::TypedArray::max_length()),
    )
    .unwrap();
    assert!(!scope.has_caught());
  }

  {
    let scope = &mut v8::TryCatch::new(scope);
    eval(
      scope,
      &format!("new Uint8Array({})", v8::TypedArray::max_length() + 1),
    );
    // Array is too big (> max_length) - expecting this threw a RangeError
    assert!(scope.has_caught());
  }
}

#[test]
fn dynamic_import() {
  let _setup_guard = setup::parallel_test();
  let isolate = &mut v8::Isolate::new(Default::default());

  static CALL_COUNT: AtomicUsize = AtomicUsize::new(0);

  fn dynamic_import_cb<'s>(
    scope: &mut v8::HandleScope<'s>,
    _host_defined_options: v8::Local<'s, v8::Data>,
    _resource_name: v8::Local<'s, v8::Value>,
    specifier: v8::Local<'s, v8::String>,
    _import_assertions: v8::Local<'s, v8::FixedArray>,
  ) -> Option<v8::Local<'s, v8::Promise>> {
    assert!(
      specifier.strict_equals(v8::String::new(scope, "bar.js").unwrap().into())
    );
    let e = v8::String::new(scope, "boom").unwrap();
    scope.throw_exception(e.into());
    CALL_COUNT.fetch_add(1, Ordering::SeqCst);
    None
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
  let _setup_guard = setup::parallel_test();
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
fn typeof_checker() {
  let _setup_guard = setup::parallel_test();

  let isolate = &mut v8::Isolate::new(Default::default());
  let scope = &mut v8::HandleScope::new(isolate);
  let context = v8::Context::new(scope);
  let scope = &mut v8::ContextScope::new(scope, context);

  let value_1 = eval(scope, "").unwrap();
  let type_of = value_1.type_of(scope);
  let value_2 = eval(scope, "").unwrap();
  let type_of_2 = value_2.type_of(scope);
  assert_eq!(type_of, type_of_2);
  let value_3 = eval(scope, "1").unwrap();
  let type_of_3 = value_3.type_of(scope);
  assert_ne!(type_of_2, type_of_3);
}

#[test]
#[allow(clippy::cognitive_complexity)]
#[allow(clippy::eq_op)]
fn value_checker() {
  let _setup_guard = setup::parallel_test();
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

    let value = eval(
      scope,
      r#"
    function* values() {
      for (var i = 0; i < arguments.length; i++) {
        yield arguments[i];
      }
    }
    values(1, 2, 3)"#,
    )
    .unwrap();
    assert!(value.is_generator_object());
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
  let _setup_guard = setup::parallel_test();

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
  let _setup_guard = setup::parallel_test();
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

  unsafe fn base_ptr(
    this: *const Self,
  ) -> *const v8::inspector::V8InspectorClientBase
  where
    Self: Sized,
  {
    unsafe { addr_of!((*this).base) }
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
  notifications: Vec<String>,
  count_flush_protocol_notifications: usize,
}

impl ChannelCounter {
  pub fn new() -> Self {
    Self {
      base: v8::inspector::ChannelBase::new::<Self>(),
      count_send_response: 0,
      count_send_notification: 0,
      notifications: vec![],
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
  unsafe fn base_ptr(_this: *const Self) -> *const ChannelBase
  where
    Self: Sized,
  {
    unsafe { addr_of!((*_this).base) }
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
    let msg = message.unwrap().string().to_string();
    println!("send_notification message {}", msg);
    self.count_send_notification += 1;
    self.notifications.push(msg);
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
  let _setup_guard = setup::parallel_test();
  let isolate = &mut v8::Isolate::new(Default::default());

  use v8::inspector::*;
  let mut default_client = ClientCounter::new();
  let mut inspector = V8Inspector::create(isolate, &mut default_client);

  let scope = &mut v8::HandleScope::new(isolate);
  let context = v8::Context::new(scope);
  let mut _scope = v8::ContextScope::new(scope, context);

  let name = b"";
  let name_view = StringView::from(&name[..]);
  let aux_data = StringView::from(&name[..]);
  inspector.context_created(context, 1, name_view, aux_data);
  let mut channel = ChannelCounter::new();
  let state = b"{}";
  let state_view = StringView::from(&state[..]);
  let mut session = inspector.connect(
    1,
    &mut channel,
    state_view,
    V8InspectorClientTrustLevel::Untrusted,
  );
  let message = String::from(
    r#"{"id":1,"method":"Network.enable","params":{"maxPostDataSize":65536}}"#,
  );
  let message = &message.into_bytes()[..];
  let string_view = StringView::from(message);
  session.dispatch_protocol_message(string_view);
  assert_eq!(channel.count_send_response, 1);
  assert_eq!(channel.count_send_notification, 0);
  assert_eq!(channel.count_flush_protocol_notifications, 0);
  inspector.context_destroyed(context);
}

#[test]
fn inspector_exception_thrown() {
  let _setup_guard = setup::parallel_test();
  let isolate = &mut v8::Isolate::new(Default::default());

  use v8::inspector::*;
  let mut default_client = ClientCounter::new();
  let mut inspector = V8Inspector::create(isolate, &mut default_client);

  let scope = &mut v8::HandleScope::new(isolate);
  let context = v8::Context::new(scope);
  let mut context_scope = v8::ContextScope::new(scope, context);

  let name = b"";
  let name_view = StringView::from(&name[..]);
  let aux_data = b"";
  let aux_data_view = StringView::from(&aux_data[..]);
  inspector.context_created(context, 1, name_view, aux_data_view);
  let mut channel = ChannelCounter::new();
  let state = b"{}";
  let state_view = StringView::from(&state[..]);
  let mut session = inspector.connect(
    1,
    &mut channel,
    state_view,
    V8InspectorClientTrustLevel::Untrusted,
  );
  let message = String::from(r#"{"id":1,"method":"Runtime.enable"}"#);
  let message = &message.into_bytes()[..];
  let string_view = StringView::from(message);
  session.dispatch_protocol_message(string_view);
  assert_eq!(channel.count_send_response, 1);
  assert_eq!(channel.count_send_notification, 1);
  assert_eq!(channel.count_flush_protocol_notifications, 0);

  let message = "test exception".to_string();
  let message = &message.into_bytes()[..];
  let message_string_view = StringView::from(message);
  let detailed_message = "detailed message".to_string();
  let detailed_message = &detailed_message.into_bytes()[..];
  let detailed_message_string_view = StringView::from(detailed_message);
  let url = "file://exception.js".to_string();
  let url = &url.into_bytes()[..];
  let url_string_view = StringView::from(url);
  let exception_msg =
    v8::String::new(&mut context_scope, "This is a test error").unwrap();
  let exception = v8::Exception::error(&mut context_scope, exception_msg);
  let stack_trace =
    v8::Exception::get_stack_trace(&mut context_scope, exception).unwrap();
  let stack_trace_ptr = inspector.create_stack_trace(stack_trace);
  let _id = inspector.exception_thrown(
    context,
    message_string_view,
    exception,
    detailed_message_string_view,
    url_string_view,
    1,
    1,
    stack_trace_ptr,
    1,
  );

  assert_eq!(channel.count_send_notification, 2);
  let notification = channel.notifications.get(1).unwrap().clone();
  let expected_notification = "{\"method\":\"Runtime.exceptionThrown\",\"params\":{\"timestamp\":0,\"exceptionDetails\":{\"exceptionId\":1,\"text\":\"test exception\",\"lineNumber\":0,\"columnNumber\":0,\"scriptId\":\"1\",\"url\":\"file://exception.js\",\"exception\":{\"type\":\"object\",\"subtype\":\"error\",\"className\":\"Error\",\"description\":\"Error: This is a test error\",\"objectId\":\"1.1.1\",\"preview\":{\"type\":\"object\",\"subtype\":\"error\",\"description\":\"Error: This is a test error\",\"overflow\":false,\"properties\":[{\"name\":\"stack\",\"type\":\"string\",\"value\":\"Error: This is a test error\"},{\"name\":\"message\",\"type\":\"string\",\"value\":\"This is a test error\"}]}},\"executionContextId\":1}}}";
  assert_eq!(notification, expected_notification);
}

#[test]
fn inspector_schedule_pause_on_next_statement() {
  let _setup_guard = setup::parallel_test();
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
  let mut session = inspector.connect(
    1,
    &mut channel,
    state_view,
    V8InspectorClientTrustLevel::FullyTrusted,
  );

  let name = b"";
  let name_view = StringView::from(&name[..]);
  let aux_data = StringView::from(&name[..]);
  inspector.context_created(context, 1, name_view, aux_data);

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
  let _setup_guard = setup::parallel_test();
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

    unsafe fn base_ptr(
      _this: *const Self,
    ) -> *const v8::inspector::V8InspectorClientBase {
      unsafe { addr_of!((*_this).base) }
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
  let aux_data = b"{\"isDefault\": true}";
  let aux_data_view = StringView::from(&aux_data[..]);
  inspector.context_created(context, 1, name_view, aux_data_view);

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
  let _setup_guard = setup::parallel_test();
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
  let _setup_guard = setup::parallel_test();
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
fn get_constructor_name() {
  let _setup_guard = setup::parallel_test();
  let isolate = &mut v8::Isolate::new(Default::default());
  let scope = &mut v8::HandleScope::new(isolate);
  let context = v8::Context::new(scope);
  let scope = &mut v8::ContextScope::new(scope, context);

  fn check_ctor_name(
    scope: &mut v8::HandleScope,
    script: &str,
    expected_name: &str,
  ) {
    let val = eval(scope, script).unwrap();
    let obj: v8::Local<v8::Object> = val.try_into().unwrap();
    assert_eq!(
      obj.get_constructor_name().to_rust_string_lossy(scope),
      expected_name
    );
  }

  let code = r#"
  function Parent() {};
  function Child() {};
  Child.prototype = new Parent();
  Child.prototype.constructor = Child;
  var outer = { inner: (0, function() { }) };
  var p = new Parent();
  var c = new Child();
  var x = new outer.inner();
  var proto = Child.prototype;
  "#;
  eval(scope, code).unwrap();
  check_ctor_name(scope, "p", "Parent");
  check_ctor_name(scope, "c", "Child");
  check_ctor_name(scope, "x", "outer.inner");
  check_ctor_name(scope, "proto", "Parent");
}

#[test]
fn test_prototype_api() {
  let _setup_guard = setup::parallel_test();
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
  let _setup_guard = setup::parallel_test();
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
  let _setup_guard = setup::parallel_test();
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

    let own_props = obj
      .get_own_property_names(scope, Default::default())
      .unwrap();
    assert_eq!(own_props.length(), 1);
    assert!(own_props.get_index(scope, 0).unwrap() == js_test_str);

    let proto_props = proto_obj
      .get_own_property_names(scope, Default::default())
      .unwrap();
    assert_eq!(proto_props.length(), 1);
    assert!(proto_props.get_index(scope, 0).unwrap() == js_proto_test_str);

    let all_props = obj.get_property_names(scope, Default::default()).unwrap();
    js_sort_fn.call(scope, all_props.into(), &[]).unwrap();
    assert_eq!(all_props.length(), 2);
    assert!(all_props.get_index(scope, 0).unwrap() == js_proto_test_str);
    assert!(all_props.get_index(scope, 1).unwrap() == js_test_str);
  }

  {
    let obj = v8::Object::new(scope);
    obj.set(scope, js_test_str, js_null);
    obj.set(scope, js_test_symbol, js_null);

    let own_props = obj
      .get_own_property_names(scope, Default::default())
      .unwrap();
    assert_eq!(own_props.length(), 1);
    assert!(own_props.get_index(scope, 0).unwrap() == js_test_str);
  }

  {
    let obj = v8::Object::new(scope);
    obj.set(scope, js_test_str, js_null);
    obj.set(scope, js_test_symbol, js_null);

    let own_props = obj
      .get_property_names(
        scope,
        v8::GetPropertyNamesArgs {
          mode: v8::KeyCollectionMode::IncludePrototypes,
          property_filter: v8::ONLY_ENUMERABLE | v8::SKIP_SYMBOLS,
          index_filter: v8::IndexFilter::IncludeIndices,
          key_conversion: v8::KeyConversionMode::KeepNumbers,
        },
      )
      .unwrap();
    assert_eq!(own_props.length(), 1);
    assert!(own_props.get_index(scope, 0).unwrap() == js_test_str);
  }

  {
    let context = v8::Context::new(scope);
    let scope = &mut v8::ContextScope::new(scope, context);

    let val = eval(scope, "({ 'a': 3, 2: 'b', '7': 'c' })").unwrap();
    let obj = val.to_object(scope).unwrap();

    {
      let own_props = obj
        .get_own_property_names(scope, Default::default())
        .unwrap();

      assert_eq!(own_props.length(), 3);

      assert!(own_props.get_index(scope, 0).unwrap().is_number());
      assert_eq!(
        own_props.get_index(scope, 0).unwrap(),
        v8::Integer::new(scope, 2)
      );

      assert!(own_props.get_index(scope, 1).unwrap().is_number());
      assert_eq!(
        own_props.get_index(scope, 1).unwrap(),
        v8::Integer::new(scope, 7)
      );

      assert!(own_props.get_index(scope, 2).unwrap().is_string());
      assert_eq!(
        own_props.get_index(scope, 2).unwrap(),
        v8::String::new(scope, "a").unwrap()
      );
    }

    {
      let own_props = obj
        .get_own_property_names(
          scope,
          v8::GetPropertyNamesArgsBuilder::new()
            .key_conversion(v8::KeyConversionMode::ConvertToString)
            .build(),
        )
        .unwrap();

      assert_eq!(own_props.length(), 3);

      assert!(own_props.get_index(scope, 0).unwrap().is_string());
      assert_eq!(
        own_props.get_index(scope, 0).unwrap(),
        v8::String::new(scope, "2").unwrap()
      );

      assert!(own_props.get_index(scope, 1).unwrap().is_string());
      assert_eq!(
        own_props.get_index(scope, 1).unwrap(),
        v8::String::new(scope, "7").unwrap()
      );

      assert!(own_props.get_index(scope, 2).unwrap().is_string());
      assert_eq!(
        own_props.get_index(scope, 2).unwrap(),
        v8::String::new(scope, "a").unwrap()
      );
    }

    {
      let own_props = obj
        .get_property_names(
          scope,
          v8::GetPropertyNamesArgsBuilder::new()
            .key_conversion(v8::KeyConversionMode::ConvertToString)
            .build(),
        )
        .unwrap();

      assert_eq!(own_props.length(), 3);

      assert!(own_props.get_index(scope, 0).unwrap().is_string());
      assert_eq!(
        own_props.get_index(scope, 0).unwrap(),
        v8::String::new(scope, "2").unwrap()
      );

      assert!(own_props.get_index(scope, 1).unwrap().is_string());
      assert_eq!(
        own_props.get_index(scope, 1).unwrap(),
        v8::String::new(scope, "7").unwrap()
      );

      assert!(own_props.get_index(scope, 2).unwrap().is_string());
      assert_eq!(
        own_props.get_index(scope, 2).unwrap(),
        v8::String::new(scope, "a").unwrap()
      );
    }
  }
}

#[test]
fn module_snapshot() {
  let _setup_guard = setup::sequential_test();

  let startup_data = {
    let mut snapshot_creator = v8::Isolate::snapshot_creator(None);
    {
      let scope = &mut v8::HandleScope::new(&mut snapshot_creator);
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

      scope.set_default_context(context);
    }
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

      let result = eval(scope, "a === 3").unwrap();
      assert!(result.same_value(true_val));

      let result = eval(scope, "b === 42").unwrap();
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
  let _setup_guard = setup::parallel_test();

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
          .map((s) => s.repeat(100).split("o"))
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
  let _setup_guard = setup::parallel_test();

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
    assert!(module
      .set_synthetic_module_export(scope, name, value)
      .is_none());
    assert!(scope.has_caught());
    scope.reset();
  }

  Some(v8::undefined(scope).into())
}

#[test]
fn synthetic_module() {
  let _setup_guard = setup::parallel_test();
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

  let _setup_guard = setup::parallel_test();
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
  let _setup_guard = setup::parallel_test();
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
  let _setup_guard = setup::parallel_test();
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
  let _setup_guard = setup::parallel_test();
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

  let _setup_guard = setup::parallel_test();
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
    let _setup_guard = setup::parallel_test();
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
    let _setup_guard = setup::parallel_test();
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
    let _setup_guard = setup::parallel_test();
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
    let _setup_guard = setup::parallel_test();
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

impl v8::ValueSerializerImpl for Custom2Value {
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
  let _setup_guard = setup::parallel_test();
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
fn memory_pressure_notification() {
  let _setup_guard = setup::parallel_test();
  let isolate = &mut v8::Isolate::new(Default::default());
  isolate.memory_pressure_notification(v8::MemoryPressureLevel::Moderate);
  isolate.memory_pressure_notification(v8::MemoryPressureLevel::Critical);
  isolate.memory_pressure_notification(v8::MemoryPressureLevel::None);
}

// Flaky on aarch64-qemu (Stack corruption).
#[cfg(not(target_os = "android"))]
#[test]
fn clear_kept_objects() {
  let _setup_guard = setup::parallel_test();

  let isolate = &mut v8::Isolate::new(Default::default());
  isolate.set_microtasks_policy(v8::MicrotasksPolicy::Explicit);

  let scope = &mut v8::HandleScope::new(isolate);
  let context = v8::Context::new(scope);
  let scope = &mut v8::ContextScope::new(scope, context);

  let step1 = r#"
    var weakrefs = [];
    for (let i = 0; i < 424242; i++) weakrefs.push(new WeakRef({ i }));
  "#;
  let step2 = r#"
    if (weakrefs.some(w => !w.deref())) throw "fail";
  "#;

  let step3 = r#"
    if (weakrefs.every(w => w.deref())) throw "fail";
  "#;

  eval(scope, step1).unwrap();
  scope.request_garbage_collection_for_testing(v8::GarbageCollectionType::Full);
  eval(scope, step2).unwrap();
  scope.clear_kept_objects();
  scope.request_garbage_collection_for_testing(v8::GarbageCollectionType::Full);
  eval(scope, step3).unwrap();
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

  let _setup_guard = setup::parallel_test();

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

  while v8::Platform::pump_message_loop(
    &v8::V8::get_current_platform(),
    scope,
    false, // don't block if there are no tasks
  ) {}

  // We did not set wasm resolve callback so V8 uses the default one that
  // runs microtasks automatically.
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
  // We did not set wasm resolve callback so V8 uses the default one that
  // runs microtasks automatically.
  while v8::Platform::pump_message_loop(
    &v8::V8::get_current_platform(),
    scope,
    false, // don't block if there are no tasks
  ) {}
  assert!(global.get(scope, name).unwrap().strict_equals(exception));
}

#[test]
fn unbound_script_conversion() {
  let _setup_guard = setup::parallel_test();
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
    let mut store: Vec<MaybeUninit<u8>> = Vec::with_capacity(n);
    store.set_len(n);
    Box::into_raw(store.into_boxed_slice()) as *mut [u8] as *mut c_void
  }
  unsafe extern "C" fn free(count: &AtomicUsize, data: *mut c_void, n: usize) {
    count.fetch_sub(n, Ordering::SeqCst);
    let _ = Box::from_raw(std::slice::from_raw_parts_mut(data as *mut u8, n));
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

  let _setup_guard = setup::parallel_test();
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
  extern "C" fn oom_handler(
    _: *const std::os::raw::c_char,
    _: &v8::OomDetails,
  ) {
    unreachable!()
  }

  let _setup_guard = setup::parallel_test();
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

  let _setup_guard = setup::parallel_test();
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
  let _setup_guard = setup::parallel_test();
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
  assert!(v8::icu::set_common_data_72(&[1, 2, 3]).is_err());
}

#[test]
fn icu_format() {
  let _setup_guard = setup::parallel_test();
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
  let _setup_guard = setup::parallel_test();
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
  let has_cache = code_cache.is_some();
  let source = match code_cache {
    Some(x) => v8::script_compiler::Source::new_with_cached_data(
      source,
      Some(&script_origin),
      x,
    ),
    None => v8::script_compiler::Source::new(source, Some(&script_origin)),
  };
  assert_eq!(source.get_cached_data().is_some(), has_cache);
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
  let _setup_guard = setup::parallel_test();
  let isolate = &mut v8::Isolate::new(Default::default());
  let scope = &mut v8::HandleScope::new(isolate);
  let context = v8::Context::new(scope);
  let mut scope = v8::ContextScope::new(scope, context);
  create_unbound_module_script(&mut scope, "'Hello ' + value", None);
}

#[test]
fn cached_data_version_tag() {
  let _setup_guard = setup::sequential_test();
  // The value is unpredictable/unstable, as it is generated from a combined
  // hash of the V8 version number and select configuration flags. This test
  // asserts that it returns the same value twice in a row (the value ought to
  // be stable for a given v8 build), which also verifies the binding does not
  // result in a crash.
  assert_eq!(
    v8::script_compiler::cached_data_version_tag(),
    v8::script_compiler::cached_data_version_tag()
  );
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
  let _setup_guard = setup::parallel_test();

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
fn function_code_cache() {
  const CODE: &str = "return word.split('').reverse().join('');";
  let _setup_guard = setup::parallel_test();

  let code_cache = {
    let isolate = &mut v8::Isolate::new(Default::default());
    let scope = &mut v8::HandleScope::new(isolate);
    let context = v8::Context::new(scope);
    let scope = &mut v8::ContextScope::new(scope, context);
    let source = v8::script_compiler::Source::new(
      v8::String::new(scope, CODE).unwrap(),
      None,
    );
    let word = v8::String::new(scope, "word").unwrap();
    let function = v8::script_compiler::compile_function(
      scope,
      source,
      &[word],
      &[],
      v8::script_compiler::CompileOptions::EagerCompile,
      v8::script_compiler::NoCacheReason::NoReason,
    )
    .unwrap();
    function.create_code_cache().unwrap()
  };

  let isolate = &mut v8::Isolate::new(Default::default());
  let scope = &mut v8::HandleScope::new(isolate);
  let context = v8::Context::new(scope);
  let scope = &mut v8::ContextScope::new(scope, context);

  let source = v8::script_compiler::Source::new_with_cached_data(
    v8::String::new(scope, CODE).unwrap(),
    None,
    code_cache,
  );
  let word = v8::String::new(scope, "word").unwrap();
  let function = v8::script_compiler::compile_function(
    scope,
    source,
    &[word],
    &[],
    v8::script_compiler::CompileOptions::EagerCompile,
    v8::script_compiler::NoCacheReason::NoReason,
  )
  .unwrap();

  let input = v8::String::new(scope, "input").unwrap().into();
  let expected = v8::String::new(scope, "tupni").unwrap();
  let undefined = v8::undefined(scope).into();
  assert_eq!(expected, function.call(scope, undefined, &[input]).unwrap());
}

#[test]
fn eager_compile_script() {
  let _setup_guard = setup::parallel_test();
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
  let _setup_guard = setup::parallel_test();
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
fn compile_function() {
  let _setup_guard = setup::parallel_test();
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
  let function = v8::script_compiler::compile_function(
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
  let _setup_guard = setup::parallel_test();
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

  static MAP: Lazy<Arc<Mutex<HashMap<Name, Count>>>> = Lazy::new(Arc::default);

  // |name| points to a static zero-terminated C string.
  extern "C" fn callback(name: *const c_char) -> *mut i32 {
    MAP
      .lock()
      .unwrap()
      .entry(Name(name))
      .or_insert_with(|| Count(Box::leak(Box::new(0))))
      .0
  }

  let _setup_guard = setup::parallel_test();
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
      if "c:V8.CompilationCacheMisses" == name.to_string_lossy() {
        Some(unsafe { *count.0 })
      } else {
        None
      }
    })
    .unwrap();

  assert_ne!(count, 0);
}

#[cfg(not(target_os = "android"))]
#[test]
fn compiled_wasm_module() {
  let _setup_guard = setup::parallel_test();

  let compiled_module = {
    let isolate = &mut v8::Isolate::new(Default::default());
    let scope = &mut v8::HandleScope::new(isolate);
    let context = v8::Context::new(scope);
    let scope = &mut v8::ContextScope::new(scope, context);

    let wire_bytes = &[
      0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00, 0x00, 0x07, 0x03, 0x66,
      0x6F, 0x6F, 0x62, 0x61, 0x72,
    ];
    let module = v8::WasmModuleObject::compile(scope, wire_bytes).unwrap();

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
      std::slice::from_raw_parts(
        foo_bs.data().unwrap().as_ptr() as *mut u8,
        foo_bs.byte_length(),
      )
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

// https://github.com/denoland/rusty_v8/issues/849
#[test]
fn backing_store_from_empty_boxed_slice() {
  let _setup_guard = setup::parallel_test();

  let mut isolate = v8::Isolate::new(Default::default());
  let mut scope = v8::HandleScope::new(&mut isolate);
  let context = v8::Context::new(&mut scope);
  let mut scope = v8::ContextScope::new(&mut scope, context);

  let store = v8::ArrayBuffer::new_backing_store_from_boxed_slice(Box::new([]))
    .make_shared();
  let _ = v8::ArrayBuffer::with_backing_store(&mut scope, &store);
}

#[test]
fn backing_store_from_empty_vec() {
  let _setup_guard = setup::parallel_test();

  let mut isolate = v8::Isolate::new(Default::default());
  let mut scope = v8::HandleScope::new(&mut isolate);
  let context = v8::Context::new(&mut scope);
  let mut scope = v8::ContextScope::new(&mut scope, context);

  let store =
    v8::ArrayBuffer::new_backing_store_from_vec(Vec::new()).make_shared();
  let _ = v8::ArrayBuffer::with_backing_store(&mut scope, &store);
}

#[test]
fn backing_store_data() {
  let _setup_guard = setup::parallel_test();

  let mut isolate = v8::Isolate::new(Default::default());
  let mut scope = v8::HandleScope::new(&mut isolate);
  let context = v8::Context::new(&mut scope);
  let mut scope = v8::ContextScope::new(&mut scope, context);

  let v = vec![1, 2, 3, 4, 5];
  let len = v.len();
  let store = v8::ArrayBuffer::new_backing_store_from_vec(v).make_shared();
  let buf = v8::ArrayBuffer::with_backing_store(&mut scope, &store);
  assert_eq!(buf.byte_length(), len);
  assert!(buf.data().is_some());
  assert_eq!(
    unsafe {
      std::slice::from_raw_parts_mut(
        buf.data().unwrap().cast::<u8>().as_ptr(),
        len,
      )
    },
    &[1, 2, 3, 4, 5]
  );
}

#[test]
fn backing_store_resizable() {
  let _setup_guard = setup::parallel_test();

  let v = vec![1, 2, 3, 4, 5];
  let store_fixed =
    v8::ArrayBuffer::new_backing_store_from_vec(v).make_shared();
  assert!(!store_fixed.is_resizable_by_user_javascript());

  let mut isolate = v8::Isolate::new(Default::default());
  let mut scope = v8::HandleScope::new(&mut isolate);
  let context = v8::Context::new(&mut scope);
  let mut scope = v8::ContextScope::new(&mut scope, context);

  let ab_val =
    eval(&mut scope, "new ArrayBuffer(100, {maxByteLength: 200})").unwrap();
  assert!(ab_val.is_array_buffer());
  let ab = v8::Local::<v8::ArrayBuffer>::try_from(ab_val).unwrap();
  let store_resizable = ab.get_backing_store();
  assert!(store_resizable.is_resizable_by_user_javascript());
}

#[test]
fn current_stack_trace() {
  // Setup isolate
  let isolate = &mut v8::Isolate::new(Default::default());
  let scope = &mut v8::HandleScope::new(isolate);
  let context = v8::Context::new(scope);
  let scope = &mut v8::ContextScope::new(scope, context);

  // A simple JS-facing function that returns its call depth, max of 5
  fn call_depth(
    scope: &mut v8::HandleScope,
    _args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
  ) {
    let stack = v8::StackTrace::current_stack_trace(scope, 5).unwrap();
    let count = stack.get_frame_count();
    rv.set(v8::Integer::new(scope, count as i32).into())
  }

  let key = v8::String::new(scope, "callDepth").unwrap();
  let tmpl = v8::FunctionTemplate::new(scope, call_depth);
  let func = tmpl.get_function(scope).unwrap();
  let global = context.global(scope);
  global.set(scope, key.into(), func.into());

  let top_level = eval(scope, "callDepth()")
    .unwrap()
    .uint32_value(scope)
    .unwrap();
  assert_eq!(top_level, 1);

  let nested = eval(scope, "(_ => (_ => callDepth())())()")
    .unwrap()
    .uint32_value(scope)
    .unwrap();
  assert_eq!(nested, 3);

  let too_deep = eval(
    scope,
    "(_ => (_ => (_ => (_ => (_ => (_ => (_ => callDepth())())())())())())())()",
  )
  .unwrap()
  .uint32_value(scope)
  .unwrap();
  assert_eq!(too_deep, 5);
}

#[test]
fn current_script_name_or_source_url() {
  let _setup_guard = setup::parallel_test();

  static mut USED: u32 = 0;

  fn analyze_script_url_in_stack(
    scope: &mut v8::HandleScope,
    _args: v8::FunctionCallbackArguments,
    _rv: v8::ReturnValue,
  ) {
    let maybe_name = v8::StackTrace::current_script_name_or_source_url(scope);
    assert!(maybe_name.is_some());
    unsafe { USED = 1 };
    assert_eq!(maybe_name.unwrap().to_rust_string_lossy(scope), "foo.js")
  }

  // Setup isolate
  let isolate = &mut v8::Isolate::new(Default::default());
  let scope = &mut v8::HandleScope::new(isolate);
  let key = v8::String::new(scope, "analyzeScriptURLInStack").unwrap();
  let tmpl = v8::FunctionTemplate::new(scope, analyze_script_url_in_stack);
  let obj_template = v8::ObjectTemplate::new(scope);
  obj_template.set(key.into(), tmpl.into());
  let context = v8::Context::new_from_template(scope, obj_template);

  let scope = &mut v8::ContextScope::new(scope, context);
  let src = r#"function foo() {
    analyzeScriptURLInStack();
  }
  foo();"#;
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
  let source = v8::String::new(scope, src).unwrap();
  let script =
    v8::Script::compile(scope, source, Some(&script_origin)).unwrap();
  script.run(scope).unwrap();
  unsafe { assert_eq!(USED, 1) };
}

#[test]
fn instance_of() {
  let _setup_guard = setup::parallel_test();

  let mut isolate = v8::Isolate::new(Default::default());
  let mut scope = v8::HandleScope::new(&mut isolate);
  let context = v8::Context::new(&mut scope);
  let mut scope = v8::ContextScope::new(&mut scope, context);
  let global = context.global(&mut scope);
  let array_name = v8::String::new(&mut scope, "Array").unwrap();
  let array_constructor = global.get(&mut scope, array_name.into()).unwrap();
  let array_constructor =
    v8::Local::<v8::Object>::try_from(array_constructor).unwrap();
  let array: v8::Local<v8::Value> =
    v8::Array::new_with_elements(&mut scope, &[]).into();

  assert!(array.instance_of(&mut scope, array_constructor).unwrap());
}

#[test]
fn get_default_locale() {
  v8::icu::set_default_locale("nb_NO");
  let default_locale = v8::icu::get_language_tag();
  assert_eq!(default_locale, "nb-NO");
}

#[test]
fn weak_handle() {
  let _setup_guard = setup::parallel_test();

  let isolate = &mut v8::Isolate::new(Default::default());
  let scope = &mut v8::HandleScope::new(isolate);
  let context = v8::Context::new(scope);
  let scope = &mut v8::ContextScope::new(scope, context);

  let weak = {
    let scope = &mut v8::HandleScope::new(scope);
    let local = v8::Object::new(scope);

    let weak = v8::Weak::new(scope, local);
    assert!(!weak.is_empty());
    assert_eq!(weak, local);
    assert_eq!(weak.to_local(scope), Some(local));

    weak
  };

  let scope = &mut v8::HandleScope::new(scope);

  scope.request_garbage_collection_for_testing(v8::GarbageCollectionType::Full);

  assert!(weak.is_empty());
  assert_eq!(weak.to_local(scope), None);
}

#[test]
fn finalizers() {
  use std::cell::Cell;
  use std::ops::Deref;
  use std::rc::Rc;

  let _setup_guard = setup::parallel_test();

  let isolate = &mut v8::Isolate::new(Default::default());
  let scope = &mut v8::HandleScope::new(isolate);
  let context = v8::Context::new(scope);
  let scope = &mut v8::ContextScope::new(scope, context);

  // The finalizer for a dropped Weak is never called.
  {
    {
      let scope = &mut v8::HandleScope::new(scope);
      let local = v8::Object::new(scope);
      let _ =
        v8::Weak::with_finalizer(scope, local, Box::new(|_| unreachable!()));
    }

    let scope = &mut v8::HandleScope::new(scope);
    scope
      .request_garbage_collection_for_testing(v8::GarbageCollectionType::Full);
  }

  let finalizer_called = Rc::new(Cell::new(false));
  let weak = {
    let scope = &mut v8::HandleScope::new(scope);
    let local = v8::Object::new(scope);

    // We use a channel to send data into the finalizer without having to worry
    // about lifetimes.
    let (tx, rx) = std::sync::mpsc::sync_channel::<(
      Rc<v8::Weak<v8::Object>>,
      Rc<Cell<bool>>,
    )>(1);

    let weak = Rc::new(v8::Weak::with_finalizer(
      scope,
      local,
      Box::new(move |_| {
        let (weak, finalizer_called) = rx.try_recv().unwrap();
        finalizer_called.set(true);
        assert!(weak.is_empty());
      }),
    ));

    tx.send((weak.clone(), finalizer_called.clone())).unwrap();

    assert!(!weak.is_empty());
    assert_eq!(weak.deref(), &local);
    assert_eq!(weak.to_local(scope), Some(local));

    weak
  };

  let scope = &mut v8::HandleScope::new(scope);
  scope.request_garbage_collection_for_testing(v8::GarbageCollectionType::Full);
  assert!(weak.is_empty());
  assert!(finalizer_called.get());
}

#[test]
fn guaranteed_finalizers() {
  // Test that guaranteed finalizers behave the same as regular finalizers for
  // everything except being guaranteed.

  use std::cell::Cell;
  use std::ops::Deref;
  use std::rc::Rc;

  let _setup_guard = setup::parallel_test();

  let isolate = &mut v8::Isolate::new(Default::default());
  let scope = &mut v8::HandleScope::new(isolate);
  let context = v8::Context::new(scope);
  let scope = &mut v8::ContextScope::new(scope, context);

  // The finalizer for a dropped Weak is never called.
  {
    {
      let scope = &mut v8::HandleScope::new(scope);
      let local = v8::Object::new(scope);
      let _ = v8::Weak::with_guaranteed_finalizer(
        scope,
        local,
        Box::new(|| unreachable!()),
      );
    }

    let scope = &mut v8::HandleScope::new(scope);
    scope
      .request_garbage_collection_for_testing(v8::GarbageCollectionType::Full);
  }

  let finalizer_called = Rc::new(Cell::new(false));
  let weak = {
    let scope = &mut v8::HandleScope::new(scope);
    let local = v8::Object::new(scope);

    // We use a channel to send data into the finalizer without having to worry
    // about lifetimes.
    let (tx, rx) = std::sync::mpsc::sync_channel::<(
      Rc<v8::Weak<v8::Object>>,
      Rc<Cell<bool>>,
    )>(1);

    let weak = Rc::new(v8::Weak::with_guaranteed_finalizer(
      scope,
      local,
      Box::new(move || {
        let (weak, finalizer_called) = rx.try_recv().unwrap();
        finalizer_called.set(true);
        assert!(weak.is_empty());
      }),
    ));

    tx.send((weak.clone(), finalizer_called.clone())).unwrap();

    assert!(!weak.is_empty());
    assert_eq!(weak.deref(), &local);
    assert_eq!(weak.to_local(scope), Some(local));

    weak
  };

  let scope = &mut v8::HandleScope::new(scope);
  scope.request_garbage_collection_for_testing(v8::GarbageCollectionType::Full);
  assert!(weak.is_empty());
  assert!(finalizer_called.get());
}

#[test]
fn weak_from_global() {
  let _setup_guard = setup::parallel_test();

  let isolate = &mut v8::Isolate::new(Default::default());
  let scope = &mut v8::HandleScope::new(isolate);
  let context = v8::Context::new(scope);
  let scope = &mut v8::ContextScope::new(scope, context);

  let global = {
    let scope = &mut v8::HandleScope::new(scope);
    let object = v8::Object::new(scope);
    v8::Global::new(scope, object)
  };

  let weak = v8::Weak::new(scope, &global);
  assert!(!weak.is_empty());
  assert_eq!(weak.to_global(scope).unwrap(), global);

  drop(global);
  scope.request_garbage_collection_for_testing(v8::GarbageCollectionType::Full);
  assert!(weak.is_empty());
}

#[test]
fn weak_from_into_raw() {
  use std::cell::Cell;
  use std::rc::Rc;

  let _setup_guard = setup::parallel_test();

  let isolate = &mut v8::Isolate::new(Default::default());
  let scope = &mut v8::HandleScope::new(isolate);
  let context = v8::Context::new(scope);
  let scope = &mut v8::ContextScope::new(scope, context);

  let finalizer_called = Rc::new(Cell::new(false));

  assert_eq!(v8::Weak::<v8::Object>::empty(scope).into_raw(), None);
  assert!(unsafe { v8::Weak::<v8::Object>::from_raw(scope, None) }.is_empty());

  // regular back and forth
  {
    finalizer_called.take();
    let (weak1, weak2) = {
      let scope = &mut v8::HandleScope::new(scope);
      let local = v8::Object::new(scope);
      let weak = v8::Weak::new(scope, local);
      let weak_with_finalizer = v8::Weak::with_finalizer(
        scope,
        local,
        Box::new({
          let finalizer_called = finalizer_called.clone();
          move |_| {
            finalizer_called.set(true);
          }
        }),
      );
      let raw1 = weak.into_raw();
      let raw2 = weak_with_finalizer.into_raw();
      assert!(raw1.is_some());
      assert!(raw2.is_some());
      let weak1 = unsafe { v8::Weak::from_raw(scope, raw1) };
      let weak2 = unsafe { v8::Weak::from_raw(scope, raw2) };
      assert_eq!(weak1.to_local(scope), Some(local));
      assert_eq!(weak2.to_local(scope), Some(local));
      assert!(!finalizer_called.get());
      (weak1, weak2)
    };
    scope
      .request_garbage_collection_for_testing(v8::GarbageCollectionType::Full);
    assert!(weak1.is_empty());
    assert!(weak2.is_empty());
    assert!(finalizer_called.get());
  }

  // into_raw from a GC'd pointer
  {
    let weak = {
      let scope = &mut v8::HandleScope::new(scope);
      let local = v8::Object::new(scope);
      v8::Weak::new(scope, local)
    };
    assert!(!weak.is_empty());
    scope
      .request_garbage_collection_for_testing(v8::GarbageCollectionType::Full);
    assert!(weak.is_empty());
    assert_eq!(weak.into_raw(), None);
  }

  // It's fine if there's a GC while the Weak is leaked.
  {
    finalizer_called.take();
    let (weak, weak_with_finalizer) = {
      let scope = &mut v8::HandleScope::new(scope);
      let local = v8::Object::new(scope);
      let weak = v8::Weak::new(scope, local);
      let weak_with_finalizer = v8::Weak::with_finalizer(
        scope,
        local,
        Box::new({
          let finalizer_called = finalizer_called.clone();
          move |_| {
            finalizer_called.set(true);
          }
        }),
      );
      (weak, weak_with_finalizer)
    };
    assert!(!weak.is_empty());
    assert!(!weak_with_finalizer.is_empty());
    assert!(!finalizer_called.get());
    let raw1 = weak.into_raw();
    let raw2 = weak_with_finalizer.into_raw();
    assert!(raw1.is_some());
    assert!(raw2.is_some());
    scope
      .request_garbage_collection_for_testing(v8::GarbageCollectionType::Full);
    assert!(finalizer_called.get());
    let weak1 = unsafe { v8::Weak::from_raw(scope, raw1) };
    let weak2 = unsafe { v8::Weak::from_raw(scope, raw2) };
    assert!(weak1.is_empty());
    assert!(weak2.is_empty());
  }

  // Leaking a Weak will not crash the isolate.
  {
    let scope = &mut v8::HandleScope::new(scope);
    let local = v8::Object::new(scope);
    v8::Weak::new(scope, local).into_raw();
    v8::Weak::with_finalizer(scope, local, Box::new(|_| {})).into_raw();
    scope
      .request_garbage_collection_for_testing(v8::GarbageCollectionType::Full);
  }
  scope.request_garbage_collection_for_testing(v8::GarbageCollectionType::Full);
}

#[test]
fn drop_weak_from_raw_in_finalizer() {
  use std::cell::Cell;
  use std::rc::Rc;

  let _setup_guard = setup::parallel_test();

  let isolate = &mut v8::Isolate::new(Default::default());
  let scope = &mut v8::HandleScope::new(isolate);
  let context = v8::Context::new(scope);
  let scope = &mut v8::ContextScope::new(scope, context);

  let weak_ptr = Rc::new(Cell::new(None));
  let finalized = Rc::new(Cell::new(false));

  {
    let scope = &mut v8::HandleScope::new(scope);
    let local = v8::Object::new(scope);
    let weak = v8::Weak::with_finalizer(
      scope,
      local,
      Box::new({
        let weak_ptr = weak_ptr.clone();
        let finalized = finalized.clone();
        move |isolate| {
          let weak_ptr = weak_ptr.get().unwrap();
          let weak: v8::Weak<v8::Object> =
            unsafe { v8::Weak::from_raw(isolate, Some(weak_ptr)) };
          drop(weak);
          finalized.set(true);
        }
      }),
    );
    weak_ptr.set(weak.into_raw());
  }

  assert!(!finalized.get());
  scope.request_garbage_collection_for_testing(v8::GarbageCollectionType::Full);
  assert!(finalized.get());
}

#[test]
fn finalizer_on_kept_global() {
  // If a global is kept alive after an isolate is dropped, regular finalizers
  // won't be called, but guaranteed ones will.

  use std::cell::Cell;
  use std::rc::Rc;

  let _setup_guard = setup::parallel_test();

  let global;
  let weak1;
  let weak2;
  let regular_finalized = Rc::new(Cell::new(false));
  let guaranteed_finalized = Rc::new(Cell::new(false));

  {
    let isolate = &mut v8::Isolate::new(Default::default());
    let scope = &mut v8::HandleScope::new(isolate);
    let context = v8::Context::new(scope);
    let scope = &mut v8::ContextScope::new(scope, context);

    let object = v8::Object::new(scope);
    global = v8::Global::new(scope, object);
    weak1 = v8::Weak::with_finalizer(
      scope,
      object,
      Box::new({
        let finalized = regular_finalized.clone();
        move |_| finalized.set(true)
      }),
    );
    weak2 = v8::Weak::with_guaranteed_finalizer(
      scope,
      object,
      Box::new({
        let guaranteed_finalized = guaranteed_finalized.clone();
        move || guaranteed_finalized.set(true)
      }),
    );
  }

  assert!(weak1.is_empty());
  assert!(weak2.is_empty());
  assert!(!regular_finalized.get());
  assert!(guaranteed_finalized.get());
  drop(weak1);
  drop(weak2);
  drop(global);
}

#[test]
fn isolate_data_slots() {
  let _setup_guard = setup::parallel_test();
  let mut isolate = v8::Isolate::new(Default::default());

  assert_eq!(isolate.get_number_of_data_slots(), 2);

  let expected0 = "Bla";
  isolate.set_data(0, &expected0 as *const _ as *mut &str as *mut c_void);

  let expected1 = 123.456f64;
  isolate.set_data(1, &expected1 as *const _ as *mut f64 as *mut c_void);

  let actual0 = isolate.get_data(0) as *mut &str;
  let actual0 = unsafe { *actual0 };
  assert_eq!(actual0, expected0);

  let actual1 = isolate.get_data(1) as *mut f64;
  let actual1 = unsafe { *actual1 };
  assert_eq!(actual1, expected1);
}

#[test]
fn context_embedder_data() {
  let _setup_guard = setup::parallel_test();
  let isolate = &mut v8::Isolate::new(Default::default());
  let global_context;

  let expected0 = "Bla";
  let expected1 = 123.456f64;
  {
    let scope = &mut v8::HandleScope::new(isolate);
    let context = v8::Context::new(scope);

    unsafe {
      context.set_aligned_pointer_in_embedder_data(
        0,
        &expected0 as *const _ as *mut &str as *mut c_void,
      );
      context.set_aligned_pointer_in_embedder_data(
        1,
        &expected1 as *const _ as *mut f64 as *mut c_void,
      );
    }

    global_context = v8::Global::new(scope, context);
  }

  {
    let scope = &mut v8::HandleScope::new(isolate);
    let context = global_context.open(scope);
    let actual0 =
      context.get_aligned_pointer_from_embedder_data(0) as *mut &str;
    let actual0 = unsafe { *actual0 };
    assert_eq!(actual0, expected0);

    let actual1 = context.get_aligned_pointer_from_embedder_data(1) as *mut f64;
    let actual1 = unsafe { *actual1 };
    assert_eq!(actual1, expected1);
  }
}

#[test]
fn host_create_shadow_realm_context_callback() {
  let _setup_guard = setup::parallel_test();

  let isolate = &mut v8::Isolate::new(Default::default());
  let scope = &mut v8::HandleScope::new(isolate);
  let context = v8::Context::new(scope);
  let scope = &mut v8::ContextScope::new(scope, context);

  {
    let tc_scope = &mut v8::TryCatch::new(scope);
    assert!(eval(tc_scope, "new ShadowRealm()").is_none());
    assert!(tc_scope.has_caught());
  }

  struct CheckData {
    callback_called: bool,
    main_context: v8::Global<v8::Context>,
  }

  let main_context = v8::Global::new(scope, context);
  scope.set_slot(CheckData {
    callback_called: false,
    main_context,
  });

  scope.set_host_create_shadow_realm_context_callback(|scope| {
    let main_context = {
      let data = scope.get_slot_mut::<CheckData>().unwrap();
      data.callback_called = true;
      data.main_context.clone()
    };
    assert_eq!(scope.get_current_context(), main_context);

    // Can't return None without throwing.
    let message = v8::String::new(scope, "Unsupported").unwrap();
    let exception = v8::Exception::type_error(scope, message);
    scope.throw_exception(exception);
    None
  });

  {
    let tc_scope = &mut v8::TryCatch::new(scope);
    assert!(eval(tc_scope, "new ShadowRealm()").is_none());
    assert!(tc_scope.has_caught());
    assert!(tc_scope.get_slot::<CheckData>().unwrap().callback_called);
  }

  scope.set_host_create_shadow_realm_context_callback(|scope| {
    let main_context = {
      let data = scope.get_slot_mut::<CheckData>().unwrap();
      data.callback_called = true;
      data.main_context.clone()
    };
    assert_eq!(scope.get_current_context(), main_context);

    let new_context = v8::Context::new(scope);
    {
      let scope = &mut v8::ContextScope::new(scope, new_context);
      let global = new_context.global(scope);
      let key = v8::String::new(scope, "test").unwrap();
      let value = v8::Integer::new(scope, 42);
      global.set(scope, key.into(), value.into()).unwrap();
    }
    Some(new_context)
  });

  let value =
    eval(scope, "new ShadowRealm().evaluate(`globalThis.test`)").unwrap();
  assert_eq!(value.uint32_value(scope), Some(42));
  assert!(scope.get_slot::<CheckData>().unwrap().callback_called);
}

#[test]
fn test_fast_calls() {
  static mut WHO: &str = "none";
  fn fast_fn(_recv: v8::Local<v8::Object>, a: u32, b: u32) -> u32 {
    unsafe { WHO = "fast" };
    a + b
  }

  const FAST_TEST: fast_api::FastFunction = fast_api::FastFunction::new(
    &[V8Value, Uint32, Uint32],
    fast_api::CType::Uint32,
    fast_fn as _,
  );

  fn slow_fn(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
  ) {
    unsafe { WHO = "slow" };
    let a = args.get(0).uint32_value(scope).unwrap();
    let b = args.get(1).uint32_value(scope).unwrap();
    rv.set_uint32(a + b);
  }

  let _setup_guard = setup::parallel_test();
  let isolate = &mut v8::Isolate::new(Default::default());
  let scope = &mut v8::HandleScope::new(isolate);
  let context = v8::Context::new(scope);
  let scope = &mut v8::ContextScope::new(scope, context);

  let global = context.global(scope);

  let template = v8::FunctionTemplate::builder(slow_fn)
    .build_fast(scope, &FAST_TEST, None, None, None);

  let name = v8::String::new(scope, "func").unwrap();
  let value = template.get_function(scope).unwrap();
  global.set(scope, name.into(), value.into()).unwrap();
  let source = r#"
    function f(x, y) { return func(x, y); }
    %PrepareFunctionForOptimization(f);
    if (42 !== f(19, 23)) throw "unexpected";
  "#;
  eval(scope, source).unwrap();
  assert_eq!("slow", unsafe { WHO });

  let source = r#"
    %OptimizeFunctionOnNextCall(f);
    if (42 !== f(19, 23)) throw "unexpected";
  "#;
  eval(scope, source).unwrap();
  assert_eq!("fast", unsafe { WHO });
}

#[test]
fn test_fast_calls_sequence() {
  static mut WHO: &str = "none";
  fn fast_fn(
    _recv: v8::Local<v8::Object>,
    a: u32,
    b: u32,
    array: v8::Local<v8::Array>,
  ) -> u32 {
    unsafe { WHO = "fast" };
    assert_eq!(array.length(), 2);
    a + b + array.length()
  }

  const FAST_TEST: fast_api::FastFunction = fast_api::FastFunction::new(
    &[V8Value, Uint32, Uint32, Sequence(fast_api::CType::Void)],
    fast_api::CType::Uint32,
    fast_fn as _,
  );

  fn slow_fn(
    scope: &mut v8::HandleScope,
    _: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
  ) {
    unsafe { WHO = "slow" };
    rv.set(v8::Boolean::new(scope, false).into());
  }

  let _setup_guard = setup::parallel_test();
  let isolate = &mut v8::Isolate::new(Default::default());
  let scope = &mut v8::HandleScope::new(isolate);
  let context = v8::Context::new(scope);
  let scope = &mut v8::ContextScope::new(scope, context);

  let global = context.global(scope);

  let template = v8::FunctionTemplate::builder(slow_fn)
    .build_fast(scope, &FAST_TEST, None, None, None);

  let name = v8::String::new(scope, "func").unwrap();
  let value = template.get_function(scope).unwrap();
  global.set(scope, name.into(), value.into()).unwrap();
  let source = r#"
  function f(x, y, data) { return func(x, y, data); }
  %PrepareFunctionForOptimization(f);
  const arr = [3, 4];
  f(1, 2, arr);
"#;
  eval(scope, source).unwrap();
  assert_eq!("slow", unsafe { WHO });

  let source = r#"
    %OptimizeFunctionOnNextCall(f);
    f(1, 2, arr);
  "#;
  eval(scope, source).unwrap();
  assert_eq!("fast", unsafe { WHO });
}

#[test]
fn test_fast_calls_arraybuffer() {
  static mut WHO: &str = "none";
  fn fast_fn(
    _recv: v8::Local<v8::Object>,
    a: u32,
    b: u32,
    data: *const fast_api::FastApiTypedArray<u32>,
  ) -> u32 {
    unsafe { WHO = "fast" };
    a + b + unsafe { &*data }.get(0)
  }

  const FAST_TEST: fast_api::FastFunction = fast_api::FastFunction::new(
    &[V8Value, Uint32, Uint32, TypedArray(fast_api::CType::Uint32)],
    fast_api::CType::Uint32,
    fast_fn as _,
  );

  fn slow_fn(
    scope: &mut v8::HandleScope,
    _: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
  ) {
    unsafe { WHO = "slow" };
    rv.set(v8::Boolean::new(scope, false).into());
  }

  let _setup_guard = setup::parallel_test();
  let isolate = &mut v8::Isolate::new(Default::default());
  let scope = &mut v8::HandleScope::new(isolate);
  let context = v8::Context::new(scope);
  let scope = &mut v8::ContextScope::new(scope, context);

  let global = context.global(scope);

  let template = v8::FunctionTemplate::builder(slow_fn)
    .build_fast(scope, &FAST_TEST, None, None, None);

  let name = v8::String::new(scope, "func").unwrap();
  let value = template.get_function(scope).unwrap();
  global.set(scope, name.into(), value.into()).unwrap();
  let source = r#"
  function f(x, y, data) { return func(x, y, data); }
  %PrepareFunctionForOptimization(f);
  const arr = new Uint32Array([3, 4]);
  f(1, 2, arr);
"#;
  eval(scope, source).unwrap();
  assert_eq!("slow", unsafe { WHO });

  let source = r#"
    %OptimizeFunctionOnNextCall(f);
    f(1, 2, arr);
  "#;
  eval(scope, source).unwrap();
  assert_eq!("fast", unsafe { WHO });
}

#[test]
fn test_fast_calls_typedarray() {
  static mut WHO: &str = "none";
  fn fast_fn(
    _recv: v8::Local<v8::Object>,
    data: *const fast_api::FastApiTypedArray<u8>,
  ) -> u32 {
    unsafe { WHO = "fast" };
    let first = unsafe { &*data }.get(0);
    let second = unsafe { &*data }.get(1);
    let third = unsafe { &*data }.get(2);
    assert_eq!(first, 4);
    assert_eq!(second, 5);
    assert_eq!(third, 6);
    let sum = first + second + third;
    sum.into()
  }

  const FAST_TEST: fast_api::FastFunction = fast_api::FastFunction::new(
    &[V8Value, TypedArray(fast_api::CType::Uint8)],
    fast_api::CType::Uint32,
    fast_fn as _,
  );

  fn slow_fn(
    scope: &mut v8::HandleScope,
    _: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
  ) {
    unsafe { WHO = "slow" };
    rv.set(v8::Boolean::new(scope, false).into());
  }

  let _setup_guard = setup::parallel_test();
  let isolate = &mut v8::Isolate::new(Default::default());
  let scope = &mut v8::HandleScope::new(isolate);
  let context = v8::Context::new(scope);
  let scope = &mut v8::ContextScope::new(scope, context);

  let global = context.global(scope);

  let template = v8::FunctionTemplate::builder(slow_fn)
    .build_fast(scope, &FAST_TEST, None, None, None);

  let name = v8::String::new(scope, "func").unwrap();
  let value = template.get_function(scope).unwrap();
  global.set(scope, name.into(), value.into()).unwrap();
  let source = r#"
  function f(data) { return func(data); }
  %PrepareFunctionForOptimization(f);
  const arr = new Uint8Array([4, 5, 6]);
  f(arr);
"#;
  eval(scope, source).unwrap();
  assert_eq!("slow", unsafe { WHO });

  let source = r#"
    %OptimizeFunctionOnNextCall(f);
    const result = f(arr);
    if (result != 15) {
      throw new Error("wrong result");
    }
  "#;
  eval(scope, source).unwrap();
  assert_eq!("fast", unsafe { WHO });
}

#[test]
fn test_fast_calls_reciever() {
  const V8_WRAPPER_TYPE_INDEX: i32 = 0;
  const V8_WRAPPER_OBJECT_INDEX: i32 = 1;

  static mut WHO: &str = "none";
  fn fast_fn(recv: v8::Local<v8::Object>) -> u32 {
    unsafe {
      WHO = "fast";
      let embedder_obj =
        recv.get_aligned_pointer_from_internal_field(V8_WRAPPER_OBJECT_INDEX);

      let i = *(embedder_obj as *const u32);
      assert_eq!(i, 69);
      i
    }
  }

  const FAST_TEST: fast_api::FastFunction = fast_api::FastFunction::new(
    &[V8Value],
    fast_api::CType::Uint32,
    fast_fn as _,
  );

  fn slow_fn(
    scope: &mut v8::HandleScope,
    _: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
  ) {
    unsafe { WHO = "slow" };
    rv.set(v8::Boolean::new(scope, false).into());
  }

  let _setup_guard = setup::parallel_test();
  let isolate = &mut v8::Isolate::new(
    v8::CreateParams::default().embedder_wrapper_type_info_offsets(
      V8_WRAPPER_TYPE_INDEX,
      V8_WRAPPER_OBJECT_INDEX,
    ),
  );
  let scope = &mut v8::HandleScope::new(isolate);
  let context = v8::Context::new(scope);
  let scope = &mut v8::ContextScope::new(scope, context);

  let object_template = v8::ObjectTemplate::new(scope);
  assert!(object_template
    .set_internal_field_count((V8_WRAPPER_OBJECT_INDEX + 1) as usize));

  let obj = object_template.new_instance(scope).unwrap();
  let embedder_obj = Box::into_raw(Box::new(69u32));
  obj.set_aligned_pointer_in_internal_field(
    V8_WRAPPER_OBJECT_INDEX,
    embedder_obj as _,
  );

  let template = v8::FunctionTemplate::builder(slow_fn)
    .build_fast(scope, &FAST_TEST, None, None, None);

  let name = v8::String::new(scope, "method").unwrap();
  let value = template.get_function(scope).unwrap();
  obj.set(scope, name.into(), value.into()).unwrap();

  let obj_str = v8::String::new(scope, "obj").unwrap();
  let global = context.global(scope);
  global.set(scope, obj_str.into(), obj.into()).unwrap();

  let source = r#"
  function f() { return obj.method(); }
  %PrepareFunctionForOptimization(f);
  f();
"#;
  eval(scope, source).unwrap();
  assert_eq!("slow", unsafe { WHO });

  let source = r#"
    %OptimizeFunctionOnNextCall(f);
    f();
  "#;
  eval(scope, source).unwrap();
  assert_eq!("fast", unsafe { WHO });
}

#[test]
fn test_fast_calls_overload() {
  static mut WHO: &str = "none";
  fn fast_fn(
    _recv: v8::Local<v8::Object>,
    data: *const fast_api::FastApiTypedArray<u32>,
  ) {
    unsafe { WHO = "fast_buf" };
    let buf = unsafe { &*data };
    assert_eq!(buf.length, 2);
    assert_eq!(buf.get(0), 6);
    assert_eq!(buf.get(1), 9);
  }

  fn fast_fn2(_recv: v8::Local<v8::Object>, data: v8::Local<v8::Array>) {
    unsafe { WHO = "fast_array" };
    assert_eq!(data.length(), 2);
  }

  const FAST_TEST: fast_api::FastFunction = fast_api::FastFunction::new(
    &[V8Value, TypedArray(CType::Uint32)],
    CType::Void,
    fast_fn as _,
  );

  const FAST_TEST2: fast_api::FastFunction = fast_api::FastFunction::new(
    &[V8Value, Sequence(CType::Void)],
    CType::Void,
    fast_fn2 as _,
  );

  fn slow_fn(
    scope: &mut v8::HandleScope,
    _: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
  ) {
    unsafe { WHO = "slow" };
    rv.set(v8::Boolean::new(scope, false).into());
  }

  let _setup_guard = setup::parallel_test();
  let isolate = &mut v8::Isolate::new(Default::default());
  let scope = &mut v8::HandleScope::new(isolate);
  let context = v8::Context::new(scope);
  let scope = &mut v8::ContextScope::new(scope, context);

  let global = context.global(scope);

  let template = v8::FunctionTemplate::builder(slow_fn).build_fast(
    scope,
    &FAST_TEST,
    None,
    Some(&FAST_TEST2),
    None,
  );

  let name = v8::String::new(scope, "func").unwrap();
  let value = template.get_function(scope).unwrap();
  global.set(scope, name.into(), value.into()).unwrap();
  let source = r#"
  function f(data) { return func(data); }
  %PrepareFunctionForOptimization(f);
  const arr = [6, 9];
  const buf = new Uint32Array(arr);
  f(buf);
  f(arr);
"#;
  eval(scope, source).unwrap();
  assert_eq!("slow", unsafe { WHO });

  let source = r#"
    %OptimizeFunctionOnNextCall(f);
    f(buf);
  "#;
  eval(scope, source).unwrap();
  assert_eq!("fast_buf", unsafe { WHO });
  let source = r#"
    %OptimizeFunctionOnNextCall(f);
    f(arr);
  "#;
  eval(scope, source).unwrap();
  assert_eq!("fast_array", unsafe { WHO });
}

#[test]
fn test_fast_calls_callback_options_fallback() {
  static mut WHO: &str = "none";
  fn fast_fn(
    _recv: v8::Local<v8::Object>,
    options: *mut fast_api::FastApiCallbackOptions,
  ) {
    if unsafe { WHO == "fast" } {
      let options = unsafe { &mut *options };
      options.fallback = true; // Go back to slow path.
    } else {
      unsafe { WHO = "fast" };
    }
  }

  const FAST_TEST: fast_api::FastFunction = fast_api::FastFunction::new(
    &[V8Value, CallbackOptions],
    CType::Void,
    fast_fn as _,
  );

  fn slow_fn(
    scope: &mut v8::HandleScope,
    _: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
  ) {
    unsafe { WHO = "slow" };
    rv.set(v8::Boolean::new(scope, false).into());
  }

  let _setup_guard = setup::parallel_test();
  let isolate = &mut v8::Isolate::new(Default::default());
  let scope = &mut v8::HandleScope::new(isolate);
  let context = v8::Context::new(scope);
  let scope = &mut v8::ContextScope::new(scope, context);

  let global = context.global(scope);

  let template = v8::FunctionTemplate::builder(slow_fn)
    .build_fast(scope, &FAST_TEST, None, None, None);

  let name = v8::String::new(scope, "func").unwrap();
  let value = template.get_function(scope).unwrap();
  global.set(scope, name.into(), value.into()).unwrap();
  let source = r#"
  function f() { return func(); }
  %PrepareFunctionForOptimization(f);
  f();
"#;
  eval(scope, source).unwrap();
  assert_eq!("slow", unsafe { WHO });

  let source = r#"
    %OptimizeFunctionOnNextCall(f);
    f();
  "#;
  eval(scope, source).unwrap();
  assert_eq!("fast", unsafe { WHO });
  let source = r#"
  f(); // Second call fallbacks back to slow path.
"#;
  eval(scope, source).unwrap();
  assert_eq!("slow", unsafe { WHO });
}

#[test]
fn test_fast_calls_callback_options_data() {
  static mut DATA: bool = false;
  unsafe fn fast_fn(
    _recv: v8::Local<v8::Object>,
    options: *mut fast_api::FastApiCallbackOptions,
  ) {
    let options = &mut *options;
    if !options.data.data.is_external() {
      options.fallback = true;
      return;
    }

    let data = v8::Local::<v8::External>::cast(options.data.data);
    let data = &mut *(data.value() as *mut bool);
    *data = true;
  }

  const FAST_TEST: fast_api::FastFunction = fast_api::FastFunction::new(
    &[V8Value, CallbackOptions],
    CType::Void,
    fast_fn as _,
  );

  fn slow_fn(
    scope: &mut v8::HandleScope,
    _: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
  ) {
    rv.set(v8::Boolean::new(scope, false).into());
  }

  let _setup_guard = setup::parallel_test();
  let isolate = &mut v8::Isolate::new(Default::default());
  let scope = &mut v8::HandleScope::new(isolate);
  let context = v8::Context::new(scope);
  let scope = &mut v8::ContextScope::new(scope, context);

  let global = context.global(scope);
  let external =
    v8::External::new(scope, unsafe { &mut DATA as *mut bool as *mut c_void });

  let template = v8::FunctionTemplate::builder(slow_fn)
    .data(external.into())
    .build_fast(scope, &FAST_TEST, None, None, None);

  let name = v8::String::new(scope, "func").unwrap();
  let value = template.get_function(scope).unwrap();
  global.set(scope, name.into(), value.into()).unwrap();
  let source = r#"
  function f() { return func(); }
  %PrepareFunctionForOptimization(f);
  f();
"#;
  eval(scope, source).unwrap();
  assert!(unsafe { !DATA });

  let source = r#"
    %OptimizeFunctionOnNextCall(f);
    f();
  "#;
  eval(scope, source).unwrap();
  assert!(unsafe { DATA });
}

#[test]
fn test_detach_key() {
  let _setup_guard = setup::parallel_test();
  let isolate = &mut v8::Isolate::new(Default::default());
  let scope = &mut v8::HandleScope::new(isolate);
  let context = v8::Context::new(scope);
  let scope = &mut v8::ContextScope::new(scope, context);

  // Object detach key
  {
    let detach_key = eval(scope, "({})").unwrap();
    assert!(detach_key.is_object());
    let buffer = v8::ArrayBuffer::new(scope, 1024);
    buffer.set_detach_key(detach_key);
    assert!(buffer.is_detachable());
    assert_eq!(buffer.detach(None), None);
    assert!(!buffer.was_detached());
    assert_eq!(buffer.detach(Some(detach_key)), Some(true));
    assert!(buffer.was_detached());
  }

  // External detach key
  {
    let mut rust_detach_key = Box::new(42usize);
    let v8_detach_key = v8::External::new(
      scope,
      &mut *rust_detach_key as *mut usize as *mut c_void,
    );
    let buffer = v8::ArrayBuffer::new(scope, 1024);
    buffer.set_detach_key(v8_detach_key.into());
    assert!(buffer.is_detachable());
    assert_eq!(buffer.detach(None), None);
    assert!(!buffer.was_detached());
    assert_eq!(buffer.detach(Some(v8_detach_key.into())), Some(true));
    assert!(buffer.was_detached());
  }

  // Undefined detach key
  {
    let buffer = v8::ArrayBuffer::new(scope, 1024);
    buffer.set_detach_key(v8::undefined(scope).into());
    assert!(buffer.is_detachable());
    assert_eq!(buffer.detach(Some(v8::undefined(scope).into())), Some(true));
    assert!(buffer.was_detached());
  }
}

#[test]
fn test_fast_calls_onebytestring() {
  static mut WHO: &str = "none";
  fn fast_fn(
    _recv: v8::Local<v8::Object>,
    data: *const fast_api::FastApiOneByteString,
  ) -> u32 {
    unsafe { WHO = "fast" };
    let data = unsafe { &*data }.as_bytes();
    assert_eq!(b"hello", data);
    data.len() as u32
  }

  const FAST_TEST: fast_api::FastFunction = fast_api::FastFunction::new(
    &[V8Value, SeqOneByteString],
    CType::Uint32,
    fast_fn as _,
  );

  fn slow_fn(
    _: &mut v8::HandleScope,
    _: v8::FunctionCallbackArguments,
    _: v8::ReturnValue,
  ) {
    unsafe { WHO = "slow" };
  }

  let _setup_guard = setup::parallel_test();
  let isolate = &mut v8::Isolate::new(Default::default());
  let scope = &mut v8::HandleScope::new(isolate);
  let context = v8::Context::new(scope);
  let scope = &mut v8::ContextScope::new(scope, context);

  let global = context.global(scope);

  let template = v8::FunctionTemplate::builder(slow_fn)
    .build_fast(scope, &FAST_TEST, None, None, None);

  let name = v8::String::new(scope, "func").unwrap();
  let value = template.get_function(scope).unwrap();
  global.set(scope, name.into(), value.into()).unwrap();
  let source = r#"
  function f(data) { return func(data); }
  %PrepareFunctionForOptimization(f);
  const str = "hello";
  f(str);
"#;
  eval(scope, source).unwrap();
  assert_eq!("slow", unsafe { WHO });

  let source = r#"
    %OptimizeFunctionOnNextCall(f);
    const result = f(str);
    if (result != 5) {
      throw new Error("wrong result");
    }
  "#;
  eval(scope, source).unwrap();
  assert_eq!("fast", unsafe { WHO });
}

#[test]
fn gc_callbacks() {
  let _setup_guard = setup::parallel_test();

  #[derive(Default)]
  struct GCCallbackState {
    mark_sweep_calls: u64,
    incremental_marking_calls: u64,
  }

  extern "C" fn callback(
    _isolate: *mut v8::Isolate,
    r#type: v8::GCType,
    _flags: v8::GCCallbackFlags,
    data: *mut c_void,
  ) {
    // We should get a mark-sweep GC here.
    assert_eq!(r#type, v8::GC_TYPE_MARK_SWEEP_COMPACT);
    let state = unsafe { &mut *(data as *mut GCCallbackState) };
    state.mark_sweep_calls += 1;
  }

  extern "C" fn callback2(
    _isolate: *mut v8::Isolate,
    r#type: v8::GCType,
    _flags: v8::GCCallbackFlags,
    data: *mut c_void,
  ) {
    // We should get a mark-sweep GC here.
    assert_eq!(r#type, v8::GC_TYPE_INCREMENTAL_MARKING);
    let state = unsafe { &mut *(data as *mut GCCallbackState) };
    state.incremental_marking_calls += 1;
  }

  let mut state = GCCallbackState::default();
  let state_ptr = &mut state as *mut _ as *mut c_void;
  let isolate = &mut v8::Isolate::new(Default::default());
  isolate.add_gc_prologue_callback(callback, state_ptr, v8::GC_TYPE_ALL);
  isolate.add_gc_prologue_callback(
    callback2,
    state_ptr,
    v8::GC_TYPE_INCREMENTAL_MARKING | v8::GC_TYPE_PROCESS_WEAK_CALLBACK,
  );

  {
    let scope = &mut v8::HandleScope::new(isolate);
    let context = v8::Context::new(scope);
    let scope = &mut v8::ContextScope::new(scope, context);

    scope
      .request_garbage_collection_for_testing(v8::GarbageCollectionType::Full);
    assert_eq!(state.mark_sweep_calls, 1);
    assert_eq!(state.incremental_marking_calls, 0);
  }

  isolate.remove_gc_prologue_callback(callback, state_ptr);
  isolate.remove_gc_prologue_callback(callback2, state_ptr);
  {
    let scope = &mut v8::HandleScope::new(isolate);
    let context = v8::Context::new(scope);
    let scope = &mut v8::ContextScope::new(scope, context);

    scope
      .request_garbage_collection_for_testing(v8::GarbageCollectionType::Full);
    // Assert callback was removed and not called again.
    assert_eq!(state.mark_sweep_calls, 1);
    assert_eq!(state.incremental_marking_calls, 0);
  }
}

#[test]
fn test_fast_calls_pointer() {
  static mut WHO: &str = "none";
  fn fast_fn(_recv: v8::Local<v8::Object>, data: *mut c_void) -> *mut c_void {
    // Assert before re-assigning WHO, as the reassignment will change the reference.
    assert!(std::ptr::eq(data, unsafe { WHO.as_ptr() as *mut c_void }));
    unsafe { WHO = "fast" };
    std::ptr::null_mut()
  }

  const FAST_TEST: fast_api::FastFunction = fast_api::FastFunction::new(
    &[V8Value, Pointer],
    fast_api::CType::Pointer,
    fast_fn as _,
  );

  fn slow_fn(
    scope: &mut v8::HandleScope,
    _: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
  ) {
    unsafe { WHO = "slow" };
    rv.set(
      v8::External::new(scope, unsafe { WHO.as_ptr() as *mut c_void }).into(),
    );
  }

  let _setup_guard = setup::parallel_test();
  let isolate = &mut v8::Isolate::new(Default::default());
  let scope = &mut v8::HandleScope::new(isolate);
  let context = v8::Context::new(scope);
  let scope = &mut v8::ContextScope::new(scope, context);

  let global = context.global(scope);

  let template = v8::FunctionTemplate::builder(slow_fn)
    .build_fast(scope, &FAST_TEST, None, None, None);

  let name = v8::String::new(scope, "func").unwrap();
  let value = template.get_function(scope).unwrap();
  global.set(scope, name.into(), value.into()).unwrap();
  let source = r#"
  function f(data) {
    return func(data);
  }
  %PrepareFunctionForOptimization(f);
  const external = f(null);
  if (
    typeof external !== "object" || external === null ||
    Object.keys(external).length > 0 || Object.getPrototypeOf(external) !== null
  ) {
    throw new Error(
      "External pointer object should be an empty object with no properties and no prototype",
    );
  }
"#;
  eval(scope, source).unwrap();
  assert_eq!("slow", unsafe { WHO });

  let source = r#"
    %OptimizeFunctionOnNextCall(f);
    const external_fast = f(external);
    if (external_fast !== null) {
      throw new Error("Null pointer external should be JS null");
    }
  "#;
  eval(scope, source).unwrap();
  assert_eq!("fast", unsafe { WHO });
}

#[test]
fn object_define_property() {
  let _setup_guard = setup::parallel_test();
  let isolate = &mut v8::Isolate::new(Default::default());
  {
    let scope = &mut v8::HandleScope::new(isolate);
    let context = v8::Context::new(scope);
    let scope = &mut v8::ContextScope::new(scope, context);

    let mut desc = v8::PropertyDescriptor::new();
    desc.set_configurable(true);
    desc.set_enumerable(false);

    let name = v8::String::new(scope, "g").unwrap();
    context
      .global(scope)
      .define_property(scope, name.into(), &desc);
    let source = r#"
      {
        const d = Object.getOwnPropertyDescriptor(globalThis, "g");
        [d.configurable, d.enumerable, d.writable].toString()
      }
    "#;
    let actual = eval(scope, source).unwrap();
    let expected = v8::String::new(scope, "true,false,false").unwrap();
    assert!(expected.strict_equals(actual));
  }

  {
    let scope = &mut v8::HandleScope::new(isolate);
    let context = v8::Context::new(scope);
    let scope = &mut v8::ContextScope::new(scope, context);

    let mut desc = v8::PropertyDescriptor::new_from_value_writable(
      v8::Integer::new(scope, 42).into(),
      true,
    );
    desc.set_configurable(true);
    desc.set_enumerable(false);

    let name = v8::String::new(scope, "g").unwrap();
    context
      .global(scope)
      .define_property(scope, name.into(), &desc);
    let source = r#"
      {
        const d = Object.getOwnPropertyDescriptor(globalThis, "g");
        [d.configurable, d.enumerable, d.writable].toString()
      }
    "#;
    let actual = eval(scope, source).unwrap();
    let expected = v8::String::new(scope, "true,false,true").unwrap();
    assert!(expected.strict_equals(actual));
  }

  {
    let scope = &mut v8::HandleScope::new(isolate);
    let context = v8::Context::new(scope);
    let scope = &mut v8::ContextScope::new(scope, context);

    let mut desc = v8::PropertyDescriptor::new_from_value(
      v8::Integer::new(scope, 42).into(),
    );
    desc.set_configurable(true);
    desc.set_enumerable(false);

    let name = v8::String::new(scope, "g").unwrap();
    context
      .global(scope)
      .define_property(scope, name.into(), &desc);
    let source = r#"
      {
        const d = Object.getOwnPropertyDescriptor(globalThis, "g");
        [d.configurable, d.enumerable, d.writable].toString()
      }
    "#;
    let actual = eval(scope, source).unwrap();
    let expected = v8::String::new(scope, "true,false,false").unwrap();
    assert!(expected.strict_equals(actual));
  }
}
