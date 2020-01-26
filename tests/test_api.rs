// Copyright 2019-2020 the Deno authors. All rights reserved. MIT license.

#[macro_use]
extern crate lazy_static;

use std::convert::{Into, TryFrom, TryInto};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

use rusty_v8 as v8;
// TODO(piscisaureus): Ideally there would be no need to import this trait.
use v8::MapFnTo;

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
  let isolate = v8::Isolate::new(params);
  let mut locker = v8::Locker::new(&isolate);
  let scope0 = locker.enter();
  {
    let mut hs = v8::HandleScope::new(scope0);
    let scope1 = hs.enter();
    {
      let mut hs = v8::HandleScope::new(scope1);
      let _scope2 = hs.enter();
    }
  }
}

#[test]
#[allow(clippy::float_cmp)]
fn handle_scope_numbers() {
  let _setup_guard = setup();
  let mut params = v8::Isolate::create_params();
  params.set_array_buffer_allocator(v8::new_default_allocator());
  let isolate = v8::Isolate::new(params);
  let mut locker = v8::Locker::new(&isolate);
  let scope0 = locker.enter();
  {
    let mut hs = v8::HandleScope::new(scope0);
    let scope1 = hs.enter();
    let l1 = v8::Integer::new(scope1, -123);
    let l2 = v8::Integer::new_from_unsigned(scope1, 456);
    {
      let mut hs = v8::HandleScope::new(scope1);
      let scope2 = hs.enter();
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
fn global_handles() {
  let _setup_guard = setup();
  let mut params = v8::Isolate::create_params();
  params.set_array_buffer_allocator(v8::new_default_allocator());
  let isolate = v8::Isolate::new(params);
  let mut locker = v8::Locker::new(&isolate);
  let scope = locker.enter();
  let mut g1 = v8::Global::<v8::String>::new();
  let mut g2 = v8::Global::<v8::Integer>::new();
  let mut g3 = v8::Global::<v8::Integer>::new();
  let mut _g4 = v8::Global::<v8::Integer>::new();
  let g5 = v8::Global::<v8::Script>::new();
  let mut g6 = v8::Global::<v8::Integer>::new();
  {
    let mut hs = v8::HandleScope::new(scope);
    let scope = hs.enter();
    let l1 = v8::String::new(scope, "bla").unwrap();
    let l2 = v8::Integer::new(scope, 123);
    g1.set(scope, l1);
    g2.set(scope, l2);
    g3.set(scope, &g2);
    _g4 = v8::Global::new_from(scope, l2);
    let l6 = v8::Integer::new(scope, 100);
    g6.set(scope, l6);
  }
  {
    let mut hs = v8::HandleScope::new(scope);
    let scope = hs.enter();
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
  }
  g1.reset(scope);
  assert!(g1.is_empty());
  g2.reset(scope);
  assert!(g2.is_empty());
  g3.reset(scope);
  assert!(g3.is_empty());
  _g4.reset(scope);
  assert!(_g4.is_empty());
  assert!(g5.is_empty());
}

#[test]
fn test_string() {
  let _setup_guard = setup();
  let mut params = v8::Isolate::create_params();
  params.set_array_buffer_allocator(v8::new_default_allocator());
  let isolate = v8::Isolate::new(params);
  let mut locker = v8::Locker::new(&isolate);
  let scope = locker.enter();
  {
    let mut hs = v8::HandleScope::new(scope);
    let scope = hs.enter();
    let reference = "Hello 🦕 world!";
    let local = v8::String::new(scope, reference).unwrap();
    assert_eq!(15, local.length());
    assert_eq!(17, local.utf8_length(scope));
    assert_eq!(reference, local.to_rust_string_lossy(scope));
  }
  {
    let mut hs = v8::HandleScope::new(scope);
    let scope = hs.enter();
    let local = v8::String::empty(scope);
    assert_eq!(0, local.length());
    assert_eq!(0, local.utf8_length(scope));
    assert_eq!("", local.to_rust_string_lossy(scope));
  }
  {
    let mut hs = v8::HandleScope::new(scope);
    let scope = hs.enter();
    let local =
      v8::String::new_from_utf8(scope, b"", v8::NewStringType::Normal).unwrap();
    assert_eq!(0, local.length());
    assert_eq!(0, local.utf8_length(scope));
    assert_eq!("", local.to_rust_string_lossy(scope));
  }
}

#[test]
#[allow(clippy::float_cmp)]
fn escapable_handle_scope() {
  let _setup_guard = setup();
  let mut params = v8::Isolate::create_params();
  params.set_array_buffer_allocator(v8::new_default_allocator());
  let isolate = v8::Isolate::new(params);
  let mut locker = v8::Locker::new(&isolate);
  let scope0 = locker.enter();
  {
    let mut hs = v8::HandleScope::new(scope0);
    let scope1 = hs.enter();
    // After dropping EscapableHandleScope, we should be able to
    // read escaped values.
    let number = {
      let mut hs = v8::EscapableHandleScope::new(scope1);
      let escapable_scope = hs.enter();
      let number = v8::Number::new(escapable_scope, 78.9);
      escapable_scope.escape(number)
    };
    assert_eq!(number.value(), 78.9);

    let string = {
      let mut hs = v8::EscapableHandleScope::new(scope1);
      let escapable_scope = hs.enter();
      let string = v8::String::new(escapable_scope, "Hello 🦕 world!").unwrap();
      escapable_scope.escape(string)
    };
    assert_eq!("Hello 🦕 world!", string.to_rust_string_lossy(scope1));

    let string = {
      let mut hs = v8::EscapableHandleScope::new(scope1);
      let escapable_scope = hs.enter();
      let nested_str_val = {
        let mut hs = v8::EscapableHandleScope::new(escapable_scope);
        let nested_escapable_scope = hs.enter();
        let string =
          v8::String::new(nested_escapable_scope, "Hello 🦕 world!").unwrap();
        nested_escapable_scope.escape(string)
      };
      escapable_scope.escape(nested_str_val)
    };
    assert_eq!("Hello 🦕 world!", string.to_rust_string_lossy(scope1));
  }
}

#[test]
fn microtasks() {
  let _setup_guard = setup();
  let mut params = v8::Isolate::create_params();
  params.set_array_buffer_allocator(v8::new_default_allocator());
  let isolate = v8::Isolate::new(params);
  let mut locker = v8::Locker::new(&isolate);
  let scope = locker.enter();

  scope.isolate().run_microtasks();

  {
    let mut hs = v8::HandleScope::new(scope);
    let scope = hs.enter();
    let context = v8::Context::new(scope);
    let mut cs = v8::ContextScope::new(scope, context);
    let scope = cs.enter();

    static CALL_COUNT: AtomicUsize = AtomicUsize::new(0);
    let function = v8::Function::new(
      scope,
      context,
      |_: v8::FunctionCallbackScope,
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
  }
}

#[test]
fn array_buffer() {
  let _setup_guard = setup();
  let mut params = v8::Isolate::create_params();
  params.set_array_buffer_allocator(v8::new_default_allocator());
  let isolate = v8::Isolate::new(params);
  let mut locker = v8::Locker::new(&isolate);
  let scope = locker.enter();
  {
    let mut hs = v8::HandleScope::new(scope);
    let scope = hs.enter();
    let context = v8::Context::new(scope);
    let mut cs = v8::ContextScope::new(scope, context);
    let scope = cs.enter();

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
  }
}

#[test]
fn array_buffer_with_shared_backing_store() {
  let _setup_guard = setup();
  let mut params = v8::Isolate::create_params();
  params.set_array_buffer_allocator(v8::new_default_allocator());
  let isolate = v8::Isolate::new(params);
  let mut locker = v8::Locker::new(&isolate);
  let scope = locker.enter();
  {
    let mut hs = v8::HandleScope::new(scope);
    let scope = hs.enter();

    let context = v8::Context::new(scope);
    let mut cs = v8::ContextScope::new(scope, context);
    let scope = cs.enter();

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
  }
}

fn v8_str<'sc>(
  scope: &mut impl v8::ToLocal<'sc>,
  s: &str,
) -> v8::Local<'sc, v8::String> {
  v8::String::new(scope, s).unwrap()
}

fn eval<'sc>(
  scope: &mut (impl v8::ToLocal<'sc> + v8::InContext),
  context: v8::Local<v8::Context>,
  code: &'static str,
) -> Option<v8::Local<'sc, v8::Value>> {
  let mut hs = v8::EscapableHandleScope::new(scope);
  let scope = hs.enter();
  let source = v8_str(scope, code);
  let mut script = v8::Script::compile(scope, context, source, None).unwrap();
  let r = script.run(scope, context);
  r.map(|v| scope.escape(v))
}

#[test]
fn try_catch() {
  let _setup_guard = setup();
  let mut params = v8::Isolate::create_params();
  params.set_array_buffer_allocator(v8::new_default_allocator());
  let isolate = v8::Isolate::new(params);
  let mut locker = v8::Locker::new(&isolate);
  let scope = locker.enter();
  {
    let mut hs = v8::HandleScope::new(scope);
    let scope = hs.enter();
    let context = v8::Context::new(scope);
    let mut cs = v8::ContextScope::new(scope, context);
    let scope = cs.enter();
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
  }
}

#[test]
fn throw_exception() {
  let _setup_guard = setup();
  let mut params = v8::Isolate::create_params();
  params.set_array_buffer_allocator(v8::new_default_allocator());
  let isolate = v8::Isolate::new(params);
  let mut locker = v8::Locker::new(&isolate);
  let scope = locker.enter();
  {
    let mut hs = v8::HandleScope::new(scope);
    let scope = hs.enter();
    let context = v8::Context::new(scope);
    let mut cs = v8::ContextScope::new(scope, context);
    let scope = cs.enter();
    {
      let mut try_catch = v8::TryCatch::new(scope);
      let tc = try_catch.enter();
      let exception = v8_str(scope, "boom");
      scope.isolate().throw_exception(exception.into());
      assert!(tc.has_caught());
      assert!(tc
        .exception()
        .unwrap()
        .strict_equals(v8_str(scope, "boom").into()));
    };
  }
}

#[test]
fn terminate_execution() {
  let _setup_guard = setup();
  let mut params = v8::Isolate::create_params();
  params.set_array_buffer_allocator(v8::new_default_allocator());
  let isolate = v8::Isolate::new(params);
  let mut locker = v8::Locker::new(&isolate);
  let scope = locker.enter();
  // Originally run fine.
  {
    let mut hs = v8::HandleScope::new(scope);
    let scope = hs.enter();
    let context = v8::Context::new(scope);
    let mut cs = v8::ContextScope::new(scope, context);
    let scope = cs.enter();
    let result = eval(scope, context, "true").unwrap();
    let true_val = v8::Boolean::new(scope, true).into();
    assert!(result.same_value(true_val));
  }
  // Terminate.
  isolate.terminate_execution();
  // Below run should fail with terminated knowledge.
  {
    let mut hs = v8::HandleScope::new(scope);
    let scope = hs.enter();
    let context = v8::Context::new(scope);
    let mut cs = v8::ContextScope::new(scope, context);
    let scope = cs.enter();
    let mut try_catch = v8::TryCatch::new(scope);
    let tc = try_catch.enter();
    let _ = eval(scope, context, "true");
    assert!(tc.has_caught());
    assert!(tc.has_terminated());
  }
  // Cancel termination.
  isolate.cancel_terminate_execution();
  // Works again.
  {
    let mut hs = v8::HandleScope::new(scope);
    let scope = hs.enter();
    let context = v8::Context::new(scope);
    let mut cs = v8::ContextScope::new(scope, context);
    let scope = cs.enter();
    let result = eval(scope, context, "true").unwrap();
    let true_val = v8::Boolean::new(scope, true).into();
    assert!(result.same_value(true_val));
  }
}

#[test]
fn request_interrupt_small_scripts() {
  let _setup_guard = setup();
  let mut params = v8::Isolate::create_params();
  params.set_array_buffer_allocator(v8::new_default_allocator());
  let isolate = v8::Isolate::new(params);
  let mut locker = v8::Locker::new(&isolate);
  let scope = locker.enter();
  {
    let mut hs = v8::HandleScope::new(scope);
    let scope = hs.enter();
    let context = v8::Context::new(scope);
    let mut cs = v8::ContextScope::new(scope, context);
    let scope = cs.enter();

    static CALL_COUNT: AtomicUsize = AtomicUsize::new(0);
    extern "C" fn callback(
      _isolate: &mut v8::Isolate,
      data: *mut std::ffi::c_void,
    ) {
      assert_eq!(data, std::ptr::null_mut());
      CALL_COUNT.fetch_add(1, Ordering::SeqCst);
    }
    isolate.request_interrupt(callback, std::ptr::null_mut());
    eval(scope, context, "(function(x){return x;})(1);");
    assert_eq!(CALL_COUNT.load(Ordering::SeqCst), 1);
  }
}

#[test]
fn add_message_listener() {
  let _setup_guard = setup();
  let mut params = v8::Isolate::create_params();
  params.set_array_buffer_allocator(v8::new_default_allocator());
  let mut isolate = v8::Isolate::new(params);
  isolate.set_capture_stack_trace_for_uncaught_exceptions(true, 32);

  static CALL_COUNT: AtomicUsize = AtomicUsize::new(0);

  extern "C" fn check_message_0(
    message: v8::Local<v8::Message>,
    _exception: v8::Local<v8::Value>,
  ) {
    let mut sc = v8::CallbackScope::new(message);
    let mut sc = v8::HandleScope::new(sc.enter());
    let scope = sc.enter();
    let context = scope.isolate().get_current_context();
    let message_str = message.get(scope);
    assert_eq!(message_str.to_rust_string_lossy(scope), "Uncaught foo");
    assert_eq!(Some(1), message.get_line_number(context));
    assert!(message.get_script_resource_name(scope).is_some());
    assert!(message.get_source_line(scope, context).is_some());
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
    assert_eq!(3, frame.get_script_id());
    assert!(frame.get_script_name(scope).is_none());
    assert!(frame.get_script_name_or_source_url(scope).is_none());
    assert!(frame.get_function_name(scope).is_none());
    assert_eq!(false, frame.is_eval());
    assert_eq!(false, frame.is_constructor());
    assert_eq!(false, frame.is_wasm());
    assert_eq!(true, frame.is_user_javascript());
    CALL_COUNT.fetch_add(1, Ordering::SeqCst);
  }
  isolate.add_message_listener(check_message_0);

  let mut locker = v8::Locker::new(&isolate);
  let scope = locker.enter();
  {
    let mut hs = v8::HandleScope::new(scope);
    let scope = hs.enter();
    let context = v8::Context::new(scope);
    let mut cs = v8::ContextScope::new(scope, context);
    let scope = cs.enter();
    let source = v8::String::new(scope, "throw 'foo'").unwrap();
    let mut script = v8::Script::compile(scope, context, source, None).unwrap();
    assert!(script.run(scope, context).is_none());
    assert_eq!(CALL_COUNT.load(Ordering::SeqCst), 1);
  }
}

fn unexpected_module_resolve_callback<'a>(
  _context: v8::Local<'a, v8::Context>,
  _specifier: v8::Local<'a, v8::String>,
  _referrer: v8::Local<'a, v8::Module>,
) -> Option<v8::Local<'a, v8::Module>> {
  unreachable!()
}

#[test]
fn set_host_initialize_import_meta_object_callback() {
  let _setup_guard = setup();
  let mut params = v8::Isolate::create_params();
  params.set_array_buffer_allocator(v8::new_default_allocator());
  let mut isolate = v8::Isolate::new(params);

  static CALL_COUNT: AtomicUsize = AtomicUsize::new(0);

  extern "C" fn callback(
    context: v8::Local<v8::Context>,
    _module: v8::Local<v8::Module>,
    meta: v8::Local<v8::Object>,
  ) {
    CALL_COUNT.fetch_add(1, Ordering::SeqCst);
    let mut cbs = v8::CallbackScope::new(context);
    let mut hs = v8::HandleScope::new(cbs.enter());
    let scope = hs.enter();
    let key = v8::String::new(scope, "foo").unwrap();
    let value = v8::String::new(scope, "bar").unwrap();
    meta.create_data_property(context, key.into(), value.into());
  }
  isolate.set_host_initialize_import_meta_object_callback(callback);

  let mut locker = v8::Locker::new(&isolate);
  let scope = locker.enter();
  {
    let mut hs = v8::HandleScope::new(scope);
    let scope = hs.enter();
    let context = v8::Context::new(scope);
    let mut cs = v8::ContextScope::new(scope, context);
    let scope = cs.enter();
    let source = mock_source(scope, "google.com", "import.meta;");
    let mut module =
      v8::script_compiler::compile_module(&isolate, source).unwrap();
    let result =
      module.instantiate_module(context, unexpected_module_resolve_callback);
    assert!(result.is_some());
    let meta = module.evaluate(scope, context).unwrap();
    assert!(meta.is_object());
    let meta = meta.to_object(scope).unwrap();
    let key = v8::String::new(scope, "foo").unwrap();
    let expected = v8::String::new(scope, "bar").unwrap();
    let actual = meta.get(scope, context, key.into()).unwrap();
    assert!(expected.strict_equals(actual));
    assert_eq!(CALL_COUNT.load(Ordering::SeqCst), 1);
  }
}

#[test]
fn script_compile_and_run() {
  let _setup_guard = setup();
  let mut params = v8::Isolate::create_params();
  params.set_array_buffer_allocator(v8::new_default_allocator());
  let isolate = v8::Isolate::new(params);
  let mut locker = v8::Locker::new(&isolate);
  let scope = locker.enter();
  {
    let mut hs = v8::HandleScope::new(scope);
    let scope = hs.enter();
    let context = v8::Context::new(scope);
    let mut cs = v8::ContextScope::new(scope, context);
    let scope = cs.enter();
    let source = v8::String::new(scope, "'Hello ' + 13 + 'th planet'").unwrap();
    let mut script = v8::Script::compile(scope, context, source, None).unwrap();
    source.to_rust_string_lossy(scope);
    let result = script.run(scope, context).unwrap();
    let result = result.to_string(scope).unwrap();
    assert_eq!(result.to_rust_string_lossy(scope), "Hello 13th planet");
  }
}

#[test]
fn script_origin() {
  let _setup_guard = setup();
  let mut params = v8::Isolate::create_params();
  params.set_array_buffer_allocator(v8::new_default_allocator());
  let isolate = v8::Isolate::new(params);
  let mut locker = v8::Locker::new(&isolate);
  let scope = locker.enter();

  {
    let mut hs = v8::HandleScope::new(scope);
    let scope = hs.enter();
    let context = v8::Context::new(scope);
    let mut cs = v8::ContextScope::new(scope, context);
    let scope = cs.enter();

    let resource_name = v8::String::new(scope, "foo.js").unwrap();
    let resource_line_offset = v8::Integer::new(scope, 4);
    let resource_column_offset = v8::Integer::new(scope, 5);
    let resource_is_shared_cross_origin = v8::Boolean::new(scope, true);
    let script_id = v8::Integer::new(scope, 123);
    let source_map_url = v8::String::new(scope, "source_map_url").unwrap();
    let resource_is_opaque = v8::Boolean::new(scope, true);
    let is_wasm = v8::Boolean::new(scope, false);
    let is_module = v8::Boolean::new(scope, false);

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

    let source = v8::String::new(scope, "1+2").unwrap();
    let mut script =
      v8::Script::compile(scope, context, source, Some(&script_origin))
        .unwrap();
    source.to_rust_string_lossy(scope);
    let _result = script.run(scope, context).unwrap();
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
  let _setup_guard = setup();
  let mut params = v8::Isolate::create_params();
  params.set_array_buffer_allocator(v8::new_default_allocator());
  let isolate = v8::Isolate::new(params);
  let mut locker = v8::Locker::new(&isolate);
  let scope = locker.enter();
  {
    let mut hs = v8::HandleScope::new(scope);
    let scope = hs.enter();
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
  let mut params = v8::Isolate::create_params();
  params.set_array_buffer_allocator(v8::new_default_allocator());
  let isolate = v8::Isolate::new(params);
  let mut locker = v8::Locker::new(&isolate);
  let scope = locker.enter();
  let mut hs = v8::HandleScope::new(scope);
  let scope = hs.enter();
  let context = v8::Context::new(scope);
  let mut cs = v8::ContextScope::new(scope, context);
  let scope = cs.enter();

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
  let mut params = v8::Isolate::create_params();
  params.set_array_buffer_allocator(v8::new_default_allocator());
  let isolate = v8::Isolate::new(params);
  let mut locker = v8::Locker::new(&isolate);
  let scope = locker.enter();
  let mut hs = v8::HandleScope::new(scope);
  let scope = hs.enter();
  let context = v8::Context::new(scope);
  let mut cs = v8::ContextScope::new(scope, context);
  let scope = cs.enter();

  {
    let create_message = v8::Function::new(
      scope,
      context,
      |scope: v8::FunctionCallbackScope,
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
      .call(scope, context, receiver.into(), &[exception])
      .unwrap();
    let expected =
      v8::String::new(scope, "Uncaught TypeError: mishap").unwrap();
    assert!(actual.strict_equals(expected.into()));
  }
}

#[test]
fn json() {
  let _setup_guard = setup();
  let mut params = v8::Isolate::create_params();
  params.set_array_buffer_allocator(v8::new_default_allocator());
  let isolate = v8::Isolate::new(params);
  let mut locker = v8::Locker::new(&isolate);
  let scope = locker.enter();
  {
    let mut hs = v8::HandleScope::new(scope);
    let scope = hs.enter();
    let context = v8::Context::new(scope);
    let mut cs = v8::ContextScope::new(scope, context);
    let scope = cs.enter();
    let json_string = v8_str(scope, "{\"a\": 1, \"b\": 2}");
    let maybe_value = v8::json::parse(context, json_string);
    assert!(maybe_value.is_some());
    let value = maybe_value.unwrap();
    let maybe_stringified = v8::json::stringify(context, value);
    assert!(maybe_stringified.is_some());
    let stringified = maybe_stringified.unwrap();
    let rust_str = stringified.to_rust_string_lossy(scope);
    assert_eq!("{\"a\":1,\"b\":2}".to_string(), rust_str);
  }
}

#[test]
fn object_template() {
  let _setup_guard = setup();
  let mut params = v8::Isolate::create_params();
  params.set_array_buffer_allocator(v8::new_default_allocator());
  let isolate = v8::Isolate::new(params);
  let mut locker = v8::Locker::new(&isolate);
  let scope = locker.enter();
  {
    let mut hs = v8::HandleScope::new(scope);
    let scope = hs.enter();
    let object_templ = v8::ObjectTemplate::new(scope);
    let function_templ = v8::FunctionTemplate::new(scope, fortytwo_callback);
    let name = v8_str(scope, "f");
    let attr = v8::READ_ONLY + v8::DONT_ENUM + v8::DONT_DELETE;
    object_templ.set_with_attr(name.into(), function_templ.into(), attr);
    let context = v8::Context::new(scope);
    let mut cs = v8::ContextScope::new(scope, context);
    let scope = cs.enter();
    let object = object_templ.new_instance(scope, context).unwrap();
    assert!(!object.is_null_or_undefined());
    let name = v8_str(scope, "g");
    context.global(scope).define_own_property(
      context,
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
    let actual = eval(scope, context, source).unwrap();
    let expected = v8_str(scope, "true,false,true");
    assert!(expected.strict_equals(actual));
    let actual = eval(scope, context, "g.f()").unwrap();
    let expected = v8::Integer::new(scope, 42);
    assert!(expected.strict_equals(actual));
    let source = r#"
      {
        const d = Object.getOwnPropertyDescriptor(g, "f");
        [d.configurable, d.enumerable, d.writable].toString()
      }
    "#;
    let actual = eval(scope, context, source).unwrap();
    let expected = v8_str(scope, "false,false,false");
    assert!(expected.strict_equals(actual));
  }
}

#[test]
fn object_template_from_function_template() {
  let _setup_guard = setup();
  let mut params = v8::Isolate::create_params();
  params.set_array_buffer_allocator(v8::new_default_allocator());
  let isolate = v8::Isolate::new(params);
  let mut locker = v8::Locker::new(&isolate);
  let scope = locker.enter();
  {
    let mut hs = v8::HandleScope::new(scope);
    let scope = hs.enter();
    let mut function_templ =
      v8::FunctionTemplate::new(scope, fortytwo_callback);
    let expected_class_name = v8_str(scope, "fortytwo");
    function_templ.set_class_name(expected_class_name);
    let object_templ =
      v8::ObjectTemplate::new_from_template(scope, function_templ);
    let context = v8::Context::new(scope);
    let mut cs = v8::ContextScope::new(scope, context);
    let scope = cs.enter();
    let object = object_templ.new_instance(scope, context).unwrap();
    assert!(!object.is_null_or_undefined());
    let name = v8_str(scope, "g");
    context
      .global(scope)
      .set(context, name.into(), object.into());
    let actual_class_name = eval(scope, context, "g.constructor.name").unwrap();
    assert!(expected_class_name.strict_equals(actual_class_name));
  }
}

#[test]
fn object() {
  let _setup_guard = setup();
  let mut params = v8::Isolate::create_params();
  params.set_array_buffer_allocator(v8::new_default_allocator());
  let isolate = v8::Isolate::new(params);
  let mut locker = v8::Locker::new(&isolate);
  let scope = locker.enter();
  {
    let mut hs = v8::HandleScope::new(scope);
    let scope = hs.enter();
    let context = v8::Context::new(scope);
    let mut cs = v8::ContextScope::new(scope, context);
    let scope = cs.enter();
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
  }
}

#[test]
fn array() {
  let _setup_guard = setup();
  let mut params = v8::Isolate::create_params();
  params.set_array_buffer_allocator(v8::new_default_allocator());
  let isolate = v8::Isolate::new(params);
  let mut locker = v8::Locker::new(&isolate);
  let scope = locker.enter();
  {
    let mut hs = v8::HandleScope::new(scope);
    let scope = hs.enter();
    let context = v8::Context::new(scope);
    let mut cs = v8::ContextScope::new(scope, context);
    let scope = cs.enter();
    let s1 = v8::String::new(scope, "a").unwrap();
    let index1 = v8::Integer::new(scope, 0);
    let s2 = v8::String::new(scope, "b").unwrap();
    let index2 = v8::Integer::new(scope, 1);
    let array = v8::Array::new(scope, 2);
    let lhs = array.creation_context(scope).global(scope);
    let rhs = context.global(scope);
    assert!(lhs.strict_equals(rhs.into()));
    array.set(context, index1.into(), s1.into());
    array.set(context, index2.into(), s2.into());

    let maybe_v1 = array.get(scope, context, index1.into());
    assert!(maybe_v1.is_some());
    assert!(maybe_v1.unwrap().same_value(s1.into()));
    let maybe_v2 = array.get(scope, context, index2.into());
    assert!(maybe_v2.is_some());
    assert!(maybe_v2.unwrap().same_value(s2.into()));
  }
}

#[test]
fn create_data_property() {
  let _setup_guard = setup();
  let mut params = v8::Isolate::create_params();
  params.set_array_buffer_allocator(v8::new_default_allocator());
  let isolate = v8::Isolate::new(params);
  let mut locker = v8::Locker::new(&isolate);
  let scope = locker.enter();
  {
    let mut hs = v8::HandleScope::new(scope);
    let scope = hs.enter();
    let context = v8::Context::new(scope);
    let mut cs = v8::ContextScope::new(scope, context);
    let scope = cs.enter();

    eval(scope, context, "var a = {};");

    let key = v8_str(scope, "a");
    let obj = context
      .global(scope)
      .get(scope, context, key.into())
      .unwrap();
    assert!(obj.is_object());
    let obj = obj.to_object(scope).unwrap();
    let key = v8_str(scope, "foo");
    let value = v8_str(scope, "bar");
    assert!(obj
      .create_data_property(context, key.into(), value.into())
      .unwrap());
    let actual = obj.get(scope, context, key.into()).unwrap();
    assert!(value.strict_equals(actual));

    let key2 = v8_str(scope, "foo2");
    assert!(obj.set(context, key2.into(), value.into()).unwrap());
    let actual = obj.get(scope, context, key2.into()).unwrap();
    assert!(value.strict_equals(actual));
  }
}

#[test]
fn object_set_accessor() {
  let _setup_guard = setup();
  let mut params = v8::Isolate::create_params();
  params.set_array_buffer_allocator(v8::new_default_allocator());
  let isolate = v8::Isolate::new(params);
  let mut locker = v8::Locker::new(&isolate);
  let scope = locker.enter();
  let mut hs = v8::HandleScope::new(scope);
  let scope = hs.enter();
  let context = v8::Context::new(scope);
  let mut cs = v8::ContextScope::new(scope, context);
  let scope = cs.enter();

  {
    static CALL_COUNT: AtomicUsize = AtomicUsize::new(0);

    let getter = |scope: v8::PropertyCallbackScope,
                  key: v8::Local<v8::Name>,
                  args: v8::PropertyCallbackArguments,
                  mut rv: v8::ReturnValue| {
      let context = scope.isolate().get_current_context();
      let this = args.this();

      let expected_key = v8::String::new(scope, "getter_key").unwrap();
      assert!(key.strict_equals(expected_key.into()));

      let int_key = v8::String::new(scope, "int_key").unwrap();
      let int_value = this.get(scope, context, int_key.into()).unwrap();
      let int_value = v8::Local::<v8::Integer>::try_from(int_value).unwrap();
      assert_eq!(int_value.value(), 42);

      let s = v8::String::new(scope, "hello").unwrap();
      assert!(rv.get(scope).is_undefined());
      rv.set(s.into());

      CALL_COUNT.fetch_add(1, Ordering::SeqCst);
    };

    let mut obj = v8::Object::new(scope);

    let getter_key = v8::String::new(scope, "getter_key").unwrap();
    obj.set_accessor(context, getter_key.into(), getter);

    let int_key = v8::String::new(scope, "int_key").unwrap();
    obj.set(context, int_key.into(), v8::Integer::new(scope, 42).into());

    let obj_name = v8::String::new(scope, "obj").unwrap();
    context
      .global(scope)
      .set(context, obj_name.into(), obj.into());

    let actual = eval(scope, context, "obj.getter_key").unwrap();
    let expected = v8::String::new(scope, "hello").unwrap();
    assert!(actual.strict_equals(expected.into()));

    assert_eq!(CALL_COUNT.load(Ordering::SeqCst), 1);
  }
}

#[test]
fn promise_resolved() {
  let _setup_guard = setup();
  let mut params = v8::Isolate::create_params();
  params.set_array_buffer_allocator(v8::new_default_allocator());
  let isolate = v8::Isolate::new(params);
  let mut locker = v8::Locker::new(&isolate);
  let scope = locker.enter();
  {
    let mut hs = v8::HandleScope::new(scope);
    let scope = hs.enter();
    let context = v8::Context::new(scope);
    let mut cs = v8::ContextScope::new(scope, context);
    let scope = cs.enter();
    let maybe_resolver = v8::PromiseResolver::new(scope, context);
    assert!(maybe_resolver.is_some());
    let mut resolver = maybe_resolver.unwrap();
    let mut promise = resolver.get_promise(scope);
    assert!(!promise.has_handler());
    assert_eq!(promise.state(), v8::PromiseState::Pending);
    let value = v8::String::new(scope, "test").unwrap();
    resolver.resolve(context, value.into());
    assert_eq!(promise.state(), v8::PromiseState::Fulfilled);
    let result = promise.result(scope);
    let result_str = result.to_string(scope).unwrap();
    assert_eq!(result_str.to_rust_string_lossy(scope), "test".to_string());
    // Resolve again with different value, since promise is already in `Fulfilled` state
    // it should be ignored.
    let value = v8::String::new(scope, "test2").unwrap();
    resolver.resolve(context, value.into());
    let result = promise.result(scope);
    let result_str = result.to_string(scope).unwrap();
    assert_eq!(result_str.to_rust_string_lossy(scope), "test".to_string());
  }
}

#[test]
fn promise_rejected() {
  let _setup_guard = setup();
  let mut params = v8::Isolate::create_params();
  params.set_array_buffer_allocator(v8::new_default_allocator());
  let isolate = v8::Isolate::new(params);
  let mut locker = v8::Locker::new(&isolate);
  let scope = locker.enter();
  {
    let mut hs = v8::HandleScope::new(scope);
    let scope = hs.enter();
    let context = v8::Context::new(scope);
    let mut cs = v8::ContextScope::new(scope, context);
    let scope = cs.enter();
    let maybe_resolver = v8::PromiseResolver::new(scope, context);
    assert!(maybe_resolver.is_some());
    let mut resolver = maybe_resolver.unwrap();
    let mut promise = resolver.get_promise(scope);
    assert!(!promise.has_handler());
    assert_eq!(promise.state(), v8::PromiseState::Pending);
    let value = v8::String::new(scope, "test").unwrap();
    let rejected = resolver.reject(context, value.into());
    assert!(rejected.unwrap());
    assert_eq!(promise.state(), v8::PromiseState::Rejected);
    let result = promise.result(scope);
    let result_str = result.to_string(scope).unwrap();
    assert_eq!(result_str.to_rust_string_lossy(scope), "test".to_string());
    // Reject again with different value, since promise is already in `Rejected` state
    // it should be ignored.
    let value = v8::String::new(scope, "test2").unwrap();
    resolver.reject(context, value.into());
    let result = promise.result(scope);
    let result_str = result.to_string(scope).unwrap();
    assert_eq!(result_str.to_rust_string_lossy(scope), "test".to_string());
  }
}

fn fn_callback(
  scope: v8::FunctionCallbackScope,
  args: v8::FunctionCallbackArguments,
  mut rv: v8::ReturnValue,
) {
  assert_eq!(args.length(), 0);
  let s = v8::String::new(scope, "Hello callback!").unwrap();
  assert!(rv.get(scope).is_undefined());
  rv.set(s.into());
}

fn fn_callback2(
  scope: v8::FunctionCallbackScope,
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
  scope: v8::FunctionCallbackScope,
  _: v8::FunctionCallbackArguments,
  mut rv: v8::ReturnValue,
) {
  rv.set(v8::Integer::new(scope, 42).into());
}

#[test]
fn function() {
  let _setup_guard = setup();
  let mut params = v8::Isolate::create_params();
  params.set_array_buffer_allocator(v8::new_default_allocator());
  let isolate = v8::Isolate::new(params);
  let mut locker = v8::Locker::new(&isolate);
  let scope = locker.enter();

  {
    let mut hs = v8::HandleScope::new(scope);
    let scope = hs.enter();
    let context = v8::Context::new(scope);
    let mut cs = v8::ContextScope::new(scope, context);
    let scope = cs.enter();
    let global = context.global(scope);
    let recv: v8::Local<v8::Value> = global.into();
    // create function using template
    let mut fn_template = v8::FunctionTemplate::new(scope, fn_callback);
    let function = fn_template
      .get_function(scope, context)
      .expect("Unable to create function");
    let lhs = function.creation_context(scope).global(scope);
    let rhs = context.global(scope);
    assert!(lhs.strict_equals(rhs.into()));
    function
      .call(scope, context, recv, &[])
      .expect("Function call failed");
    // create function without a template
    let function = v8::Function::new(scope, context, fn_callback2)
      .expect("Unable to create function");
    let arg1 = v8::String::new(scope, "arg1").unwrap();
    let arg2 = v8::Integer::new(scope, 2);
    let value = function
      .call(scope, context, recv, &[arg1.into(), arg2.into()])
      .unwrap();
    let value_str = value.to_string(scope).unwrap();
    let rust_str = value_str.to_rust_string_lossy(scope);
    assert_eq!(rust_str, "Hello callback!".to_string());
  }
}

extern "C" fn promise_reject_callback(msg: v8::PromiseRejectMessage) {
  let mut scope = v8::CallbackScope::new(&msg);
  let scope = scope.enter();
  let event = msg.get_event();
  assert_eq!(event, v8::PromiseRejectEvent::PromiseRejectWithNoHandler);
  let mut promise = msg.get_promise();
  assert_eq!(promise.state(), v8::PromiseState::Rejected);
  let value = msg.get_value();
  {
    let mut hs = v8::HandleScope::new(scope);
    let scope = hs.enter();
    let value_str = value.to_string(scope).unwrap();
    let rust_str = value_str.to_rust_string_lossy(scope);
    assert_eq!(rust_str, "promise rejected".to_string());
  }
}

#[test]
fn set_promise_reject_callback() {
  let _setup_guard = setup();
  let mut params = v8::Isolate::create_params();
  params.set_array_buffer_allocator(v8::new_default_allocator());
  let mut isolate = v8::Isolate::new(params);
  isolate.set_promise_reject_callback(promise_reject_callback);
  let mut locker = v8::Locker::new(&isolate);
  let scope = locker.enter();
  {
    let mut hs = v8::HandleScope::new(scope);
    let scope = hs.enter();
    let context = v8::Context::new(scope);
    let mut cs = v8::ContextScope::new(scope, context);
    let scope = cs.enter();
    let mut resolver = v8::PromiseResolver::new(scope, context).unwrap();
    let value = v8::String::new(scope, "promise rejected").unwrap();
    resolver.reject(context, value.into());
  }
}

fn mock_script_origin<'sc>(
  scope: &mut impl v8::ToLocal<'sc>,
  resource_name_: &str,
) -> v8::ScriptOrigin<'sc> {
  let resource_name = v8_str(scope, resource_name_);
  let resource_line_offset = v8::Integer::new(scope, 0);
  let resource_column_offset = v8::Integer::new(scope, 0);
  let resource_is_shared_cross_origin = v8::Boolean::new(scope, true);
  let script_id = v8::Integer::new(scope, 123);
  let source_map_url = v8_str(scope, "source_map_url");
  let resource_is_opaque = v8::Boolean::new(scope, true);
  let is_wasm = v8::Boolean::new(scope, false);
  let is_module = v8::Boolean::new(scope, true);
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

fn mock_source<'sc>(
  scope: &mut impl v8::ToLocal<'sc>,
  resource_name: &str,
  source: &str,
) -> v8::script_compiler::Source {
  let source_str = v8_str(scope, source);
  let script_origin = mock_script_origin(scope, resource_name);
  v8::script_compiler::Source::new(source_str, &script_origin)
}

#[test]
fn script_compiler_source() {
  let _setup_guard = setup();
  let mut params = v8::Isolate::create_params();
  params.set_array_buffer_allocator(v8::new_default_allocator());
  let mut isolate = v8::Isolate::new(params);
  isolate.set_promise_reject_callback(promise_reject_callback);
  let mut locker = v8::Locker::new(&isolate);
  let scope = locker.enter();
  {
    let mut hs = v8::HandleScope::new(scope);
    let scope = hs.enter();
    let context = v8::Context::new(scope);
    let mut cs = v8::ContextScope::new(scope, context);
    let scope = cs.enter();

    let source = "1+2";
    let script_origin = mock_script_origin(scope, "foo.js");
    let source =
      v8::script_compiler::Source::new(v8_str(scope, source), &script_origin);

    let result = v8::script_compiler::compile_module(&isolate, source);
    assert!(result.is_some());
  }
}

#[test]
fn module_instantiation_failures1() {
  let _setup_guard = setup();
  let mut params = v8::Isolate::create_params();
  params.set_array_buffer_allocator(v8::new_default_allocator());
  let isolate = v8::Isolate::new(params);
  let mut locker = v8::Locker::new(&isolate);
  let scope = locker.enter();
  {
    let mut hs = v8::HandleScope::new(scope);
    let scope = hs.enter();
    let context = v8::Context::new(scope);
    let mut cs = v8::ContextScope::new(scope, context);
    let scope = cs.enter();

    let source_text = v8_str(
      scope,
      "import './foo.js';\n\
       export {} from './bar.js';",
    );
    let origin = mock_script_origin(scope, "foo.js");
    let source = v8::script_compiler::Source::new(source_text, &origin);

    let mut module =
      v8::script_compiler::compile_module(&isolate, source).unwrap();
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

    // Instantiation should fail.
    {
      let mut try_catch = v8::TryCatch::new(scope);
      let tc = try_catch.enter();
      fn resolve_callback<'a>(
        context: v8::Local<'a, v8::Context>,
        _specifier: v8::Local<'a, v8::String>,
        _referrer: v8::Local<'a, v8::Module>,
      ) -> Option<v8::Local<'a, v8::Module>> {
        let mut cbs = v8::CallbackScope::new(context);
        let mut hs = v8::HandleScope::new(cbs.enter());
        let scope = hs.enter();
        let e = v8_str(scope, "boom");
        scope.isolate().throw_exception(e.into());
        None
      }
      let result = module.instantiate_module(context, resolve_callback);
      assert!(result.is_none());
      assert!(tc.has_caught());
      assert!(tc
        .exception()
        .unwrap()
        .strict_equals(v8_str(scope, "boom").into()));
      assert_eq!(v8::ModuleStatus::Uninstantiated, module.get_status());
    }
  }
}

fn compile_specifier_as_module_resolve_callback<'a>(
  context: v8::Local<'a, v8::Context>,
  specifier: v8::Local<'a, v8::String>,
  _referrer: v8::Local<'a, v8::Module>,
) -> Option<v8::Local<'a, v8::Module>> {
  let mut cbs = v8::CallbackScope::new_escapable(context);
  let mut hs = v8::EscapableHandleScope::new(cbs.enter());
  let scope = hs.enter();
  let origin = mock_script_origin(scope, "module.js");
  let source = v8::script_compiler::Source::new(specifier, &origin);
  let module =
    v8::script_compiler::compile_module(scope.isolate(), source).unwrap();
  Some(scope.escape(module))
}

#[test]
fn module_evaluation() {
  let _setup_guard = setup();
  let mut params = v8::Isolate::create_params();
  params.set_array_buffer_allocator(v8::new_default_allocator());
  let isolate = v8::Isolate::new(params);
  let mut locker = v8::Locker::new(&isolate);
  let scope = locker.enter();
  {
    let mut hs = v8::HandleScope::new(scope);
    let scope = hs.enter();
    let context = v8::Context::new(scope);
    let mut cs = v8::ContextScope::new(scope, context);
    let scope = cs.enter();

    let source_text = v8_str(
      scope,
      "import 'Object.expando = 5';\n\
       import 'Object.expando *= 2';",
    );
    let origin = mock_script_origin(scope, "foo.js");
    let source = v8::script_compiler::Source::new(source_text, &origin);

    let mut module =
      v8::script_compiler::compile_module(&isolate, source).unwrap();
    assert_eq!(v8::ModuleStatus::Uninstantiated, module.get_status());

    let result = module.instantiate_module(
      context,
      compile_specifier_as_module_resolve_callback,
    );
    assert!(result.unwrap());
    assert_eq!(v8::ModuleStatus::Instantiated, module.get_status());

    let result = module.evaluate(scope, context);
    assert!(result.is_some());
    assert_eq!(v8::ModuleStatus::Evaluated, module.get_status());

    let result = eval(scope, context, "Object.expando").unwrap();
    assert!(result.is_number());
    let expected = v8::Number::new(scope, 10.);
    assert!(result.strict_equals(expected.into()));
  }
}

#[test]
fn primitive_array() {
  let _setup_guard = setup();
  let mut params = v8::Isolate::create_params();
  params.set_array_buffer_allocator(v8::new_default_allocator());
  let isolate = v8::Isolate::new(params);
  let mut locker = v8::Locker::new(&isolate);
  let scope = locker.enter();
  {
    let mut hs = v8::HandleScope::new(scope);
    let scope = hs.enter();
    let context = v8::Context::new(scope);
    let mut cs = v8::ContextScope::new(scope, context);
    let scope = cs.enter();

    let length = 3;
    let array = v8::PrimitiveArray::new(scope, length);
    assert_eq!(length, array.length());

    for i in 0..length {
      let item = array.get(scope, i);
      assert!(item.is_undefined());
    }

    let string = v8_str(scope, "test");
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
  let mut params = v8::Isolate::create_params();
  params.set_array_buffer_allocator(v8::new_default_allocator());
  let isolate = v8::Isolate::new(params);
  let mut locker = v8::Locker::new(&isolate);
  let scope = locker.enter();
  {
    let mut hs = v8::HandleScope::new(scope);
    let scope = hs.enter();
    let context = v8::Context::new(scope);
    let mut cs = v8::ContextScope::new(scope, context);
    let scope = cs.enter();

    assert!(v8_str(scope, "a").strict_equals(v8_str(scope, "a").into()));
    assert!(!v8_str(scope, "a").strict_equals(v8_str(scope, "b").into()));

    assert!(v8_str(scope, "a").same_value(v8_str(scope, "a").into()));
    assert!(!v8_str(scope, "a").same_value(v8_str(scope, "b").into()));
  }
}

#[test]
fn array_buffer_view() {
  let _setup_guard = setup();
  let mut params = v8::Isolate::create_params();
  params.set_array_buffer_allocator(v8::new_default_allocator());
  let isolate = v8::Isolate::new(params);
  let mut locker = v8::Locker::new(&isolate);
  let scope = locker.enter();
  {
    let mut hs = v8::HandleScope::new(scope);
    let scope = hs.enter();
    let context = v8::Context::new(scope);
    let mut cs = v8::ContextScope::new(scope, context);
    let scope = cs.enter();
    let source =
      v8::String::new(scope, "new Uint8Array([23,23,23,23])").unwrap();
    let mut script = v8::Script::compile(scope, context, source, None).unwrap();
    source.to_rust_string_lossy(scope);
    let result: v8::Local<v8::ArrayBufferView> =
      script.run(scope, context).unwrap().try_into().unwrap();
    assert_eq!(result.byte_length(), 4);
    assert_eq!(result.byte_offset(), 0);
    let mut dest = [0; 4];
    let copy_bytes = result.copy_contents(&mut dest);
    assert_eq!(copy_bytes, 4);
    assert_eq!(dest, [23, 23, 23, 23]);
    let maybe_ab = result.buffer();
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
  let startup_data = {
    let mut snapshot_creator = v8::SnapshotCreator::new(None);
    let isolate = snapshot_creator.get_isolate();
    let mut locker = v8::Locker::new(&isolate);
    let scope = locker.enter();
    {
      let mut hs = v8::HandleScope::new(scope);
      let scope = hs.enter();
      let context = v8::Context::new(scope);
      let mut cs = v8::ContextScope::new(scope, context);
      let scope = cs.enter();

      let source = v8::String::new(scope, "a = 1 + 2").unwrap();
      let mut script =
        v8::Script::compile(scope, context, source, None).unwrap();
      script.run(scope, context).unwrap();

      snapshot_creator.set_default_context(context);
    }

    snapshot_creator
      .create_blob(v8::FunctionCodeHandling::Clear)
      .unwrap()
  };
  assert!(startup_data.len() > 0);
  // Now we try to load up the snapshot and check that 'a' has the correct
  // value.
  {
    let mut params = v8::Isolate::create_params();
    params.set_array_buffer_allocator(v8::new_default_allocator());
    params.set_snapshot_blob(&startup_data);
    let isolate = v8::Isolate::new(params);
    let mut locker = v8::Locker::new(&isolate);
    let scope = locker.enter();
    {
      let mut hs = v8::HandleScope::new(scope);
      let scope = hs.enter();
      let context = v8::Context::new(scope);
      let mut cs = v8::ContextScope::new(scope, context);
      let scope = cs.enter();
      let source = v8::String::new(scope, "a === 3").unwrap();
      let mut script =
        v8::Script::compile(scope, context, source, None).unwrap();
      let result = script.run(scope, context).unwrap();
      let true_val = v8::Boolean::new(scope, true).into();
      assert!(result.same_value(true_val));
    }
    // TODO(ry) WARNING! startup_data needs to be kept alive as long the isolate
    // using it. See note in CreateParams::set_snapshot_blob
    drop(startup_data);
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
    let isolate = snapshot_creator.get_isolate();
    let mut locker = v8::Locker::new(&isolate);
    let scope = locker.enter();
    {
      let mut hs = v8::HandleScope::new(scope);
      let scope = hs.enter();
      let context = v8::Context::new(scope);
      let mut cs = v8::ContextScope::new(scope, context);
      let scope = cs.enter();

      // create function using template
      let mut fn_template = v8::FunctionTemplate::new(scope, fn_callback);
      let function = fn_template
        .get_function(scope, context)
        .expect("Unable to create function");

      let global = context.global(scope);
      global.set(context, v8_str(scope, "F").into(), function.into());

      snapshot_creator.set_default_context(context);
    }

    snapshot_creator
      .create_blob(v8::FunctionCodeHandling::Clear)
      .unwrap()
  };
  assert!(startup_data.len() > 0);
  // Now we try to load up the snapshot and check that 'a' has the correct
  // value.
  {
    let mut params = v8::Isolate::create_params();
    params.set_array_buffer_allocator(v8::new_default_allocator());
    params.set_snapshot_blob(&startup_data);
    params.set_external_references(&EXTERNAL_REFERENCES);
    let isolate = v8::Isolate::new(params);
    let mut locker = v8::Locker::new(&isolate);
    let scope = locker.enter();
    {
      let mut hs = v8::HandleScope::new(scope);
      let scope = hs.enter();
      let context = v8::Context::new(scope);
      let mut cs = v8::ContextScope::new(scope, context);
      let scope = cs.enter();

      let result =
        eval(scope, context, "if(F() != 'wrong answer') throw 'boom1'");
      assert!(result.is_none());

      let result =
        eval(scope, context, "if(F() != 'Hello callback!') throw 'boom2'");
      assert!(result.is_some());
    }
    // TODO(ry) WARNING! startup_data needs to be kept alive as long the isolate
    // using it. See note in CreateParams::set_snapshot_blob
    drop(startup_data);
  }
}

#[test]
fn startup_data() {
  let data1 = b"abcd";
  let sd1 = v8::StartupData::new(data1);
  assert_eq!(&*sd1, data1);

  let data2 = b"defg";
  let vec2 = Vec::from(&data2[..]);
  let sd2 = v8::StartupData::new(&vec2);
  assert_eq!(&*sd2, data2);

  let data3 = b"hijk";
  let sd3 = Box::new(v8::StartupData::new(data3));
  assert_eq!(&**sd3, data3);
}

#[test]
fn uint8_array() {
  let _setup_guard = setup();
  let mut params = v8::Isolate::create_params();
  params.set_array_buffer_allocator(v8::new_default_allocator());
  let isolate = v8::Isolate::new(params);
  let mut locker = v8::Locker::new(&isolate);
  let scope = locker.enter();
  {
    let mut hs = v8::HandleScope::new(scope);
    let scope = hs.enter();
    let context = v8::Context::new(scope);
    let mut cs = v8::ContextScope::new(scope, context);
    let scope = cs.enter();
    let source =
      v8::String::new(scope, "new Uint8Array([23,23,23,23])").unwrap();
    let mut script = v8::Script::compile(scope, context, source, None).unwrap();
    source.to_rust_string_lossy(scope);
    let result: v8::Local<v8::ArrayBufferView> =
      script.run(scope, context).unwrap().try_into().unwrap();
    assert_eq!(result.byte_length(), 4);
    assert_eq!(result.byte_offset(), 0);
    let mut dest = [0; 4];
    let copy_bytes = result.copy_contents(&mut dest);
    assert_eq!(copy_bytes, 4);
    assert_eq!(dest, [23, 23, 23, 23]);
    let maybe_ab = result.buffer();
    assert!(maybe_ab.is_some());
    let ab = maybe_ab.unwrap();
    let uint8_array = v8::Uint8Array::new(ab, 0, 0);
    assert!(uint8_array.is_some());
  }
}

#[test]
fn dynamic_import() {
  let _setup_guard = setup();
  let mut params = v8::Isolate::create_params();
  params.set_array_buffer_allocator(v8::new_default_allocator());
  let mut isolate = v8::Isolate::new(params);

  static CALL_COUNT: AtomicUsize = AtomicUsize::new(0);

  extern "C" fn dynamic_import_cb(
    context: v8::Local<v8::Context>,
    _referrer: v8::Local<v8::ScriptOrModule>,
    specifier: v8::Local<v8::String>,
  ) -> *mut v8::Promise {
    let mut cbs = v8::CallbackScope::new(context);
    let mut hs = v8::HandleScope::new(cbs.enter());
    let scope = hs.enter();
    assert!(specifier.strict_equals(v8_str(scope, "bar.js").into()));
    let e = v8_str(scope, "boom");
    scope.isolate().throw_exception(e.into());
    CALL_COUNT.fetch_add(1, Ordering::SeqCst);
    std::ptr::null_mut()
  }
  isolate.set_host_import_module_dynamically_callback(dynamic_import_cb);

  let mut locker = v8::Locker::new(&isolate);
  let scope = locker.enter();
  {
    let mut hs = v8::HandleScope::new(scope);
    let scope = hs.enter();
    let context = v8::Context::new(scope);
    let mut cs = v8::ContextScope::new(scope, context);
    let scope = cs.enter();

    let result = eval(
      scope,
      context,
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
  let mut params = v8::Isolate::create_params();
  params.set_array_buffer_allocator(v8::new_default_allocator());
  let isolate = v8::Isolate::new(params);
  let mut locker = v8::Locker::new(&isolate);
  let scope = locker.enter();
  {
    let mut hs = v8::HandleScope::new(scope);
    let scope = hs.enter();
    let context = v8::Context::new(scope);
    let mut cs = v8::ContextScope::new(scope, context);
    let scope = cs.enter();

    let sab = v8::SharedArrayBuffer::new(scope, 16).unwrap();
    let shared_bs_1 = sab.get_backing_store();
    {
      let bs = unsafe { &mut *shared_bs_1.get() };
      bs[5] = 12;
      bs[12] = 52;
    }

    let global = context.global(scope);
    let r = global
      .create_data_property(context, v8_str(scope, "shared").into(), sab.into())
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
    let mut script = v8::Script::compile(scope, context, source, None).unwrap();

    let result: v8::Local<v8::Integer> =
      script.run(scope, context).unwrap().try_into().unwrap();
    assert_eq!(result.value(), 64);
    {
      let bs = unsafe { &*shared_bs_1.get() };
      assert_eq!(bs[2], 16);
      assert_eq!(bs[14], 62);
    }

    let data: Box<[u8]> = vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9].into_boxed_slice();
    let bs = v8::SharedArrayBuffer::new_backing_store_from_boxed_slice(data);
    assert_eq!(bs.byte_length(), 10);
    assert_eq!(bs.is_shared(), true);

    let mut shared_bs_2 = bs.make_shared();
    {
      let bs = unsafe { &*shared_bs_2.get() };
      assert_eq!(bs.byte_length(), 10);
      assert_eq!(bs.is_shared(), true);
    }

    let ab = v8::SharedArrayBuffer::with_backing_store(scope, &mut shared_bs_2);
    let shared_bs_3 = ab.get_backing_store();
    {
      let bs = unsafe { &*shared_bs_3.get() };
      assert_eq!(bs.byte_length(), 10);
      assert_eq!(bs[0], 0);
      assert_eq!(bs[9], 9);
    }
  }
}

#[test]
#[allow(clippy::cognitive_complexity)]
fn value_checker() {
  let _setup_guard = setup();
  let mut params = v8::Isolate::create_params();
  params.set_array_buffer_allocator(v8::new_default_allocator());
  let isolate = v8::Isolate::new(params);
  let mut locker = v8::Locker::new(&isolate);
  let scope = locker.enter();
  {
    let mut hs = v8::HandleScope::new(scope);
    let scope = hs.enter();
    let context = v8::Context::new(scope);
    let mut cs = v8::ContextScope::new(scope, context);
    let scope = cs.enter();

    let value = eval(scope, context, "undefined").unwrap();
    assert!(value.is_undefined());
    assert!(value.is_null_or_undefined());

    let value = eval(scope, context, "null").unwrap();
    assert!(value.is_null());
    assert!(value.is_null_or_undefined());

    let value = eval(scope, context, "true").unwrap();
    assert!(value.is_boolean());
    assert!(value.is_true());
    assert!(!value.is_false());

    let value = eval(scope, context, "false").unwrap();
    assert!(value.is_boolean());
    assert!(!value.is_true());
    assert!(value.is_false());

    let value = eval(scope, context, "'name'").unwrap();
    assert!(value.is_name());
    assert!(value.is_string());

    let value = eval(scope, context, "Symbol()").unwrap();
    assert!(value.is_name());
    assert!(value.is_symbol());

    let value = eval(scope, context, "() => 0").unwrap();
    assert!(value.is_function());

    let value = eval(scope, context, "async () => 0").unwrap();
    assert!(value.is_async_function());

    let value = eval(scope, context, "[]").unwrap();
    assert!(value.is_array());

    let value = eval(scope, context, "BigInt('9007199254740995')").unwrap();
    assert!(value.is_big_int());
    assert!(value.to_big_int(scope).is_some());
    let detail_string = value.to_detail_string(scope).unwrap();
    let detail_string = detail_string.to_rust_string_lossy(scope);
    assert_eq!("9007199254740995", detail_string);

    let value = eval(scope, context, "123").unwrap();
    assert!(value.is_number());
    assert!(value.is_int32());
    assert!(value.is_uint32());
    assert_eq!(123, value.to_uint32(scope).unwrap().value());
    assert_eq!(123, value.to_int32(scope).unwrap().value());
    assert_eq!(123, value.to_integer(scope).unwrap().value());
    assert_eq!(123, value.integer_value(scope).unwrap());
    assert_eq!(123, value.uint32_value(scope).unwrap());
    assert_eq!(123, value.int32_value(scope).unwrap());

    let value = eval(scope, context, "12.3").unwrap();
    assert!(12.3 - value.number_value(scope).unwrap() < 0.00001);

    let value = eval(scope, context, "-123").unwrap();
    assert!(value.is_number());
    assert!(!value.is_uint32());

    let value = eval(scope, context, "new Date()").unwrap();
    assert!(value.is_date());

    let value =
      eval(scope, context, "(function(){return arguments})()").unwrap();
    assert!(value.is_arguments_object());

    let value = eval(scope, context, "new Promise(function(){})").unwrap();
    assert!(value.is_promise());

    let value = eval(scope, context, "new Map()").unwrap();
    assert!(value.is_map());

    let value = eval(scope, context, "new Set").unwrap();
    assert!(value.is_set());

    let value = eval(scope, context, "new Map().entries()").unwrap();
    assert!(value.is_map_iterator());

    let value = eval(scope, context, "new Set().entries()").unwrap();
    assert!(value.is_set_iterator());

    let value = eval(scope, context, "new WeakMap()").unwrap();
    assert!(value.is_weak_map());

    let value = eval(scope, context, "new WeakSet()").unwrap();
    assert!(value.is_weak_set());

    let value = eval(scope, context, "new ArrayBuffer(8)").unwrap();
    assert!(value.is_array_buffer());

    let value = eval(scope, context, "new Uint8Array([])").unwrap();
    assert!(value.is_uint8_array());
    assert!(value.is_array_buffer_view());
    assert!(value.is_typed_array());

    let value = eval(scope, context, "new Uint8ClampedArray([])").unwrap();
    assert!(value.is_uint8_clamped_array());
    assert!(value.is_array_buffer_view());
    assert!(value.is_typed_array());

    let value = eval(scope, context, "new Int8Array([])").unwrap();
    assert!(value.is_int8_array());
    assert!(value.is_array_buffer_view());
    assert!(value.is_typed_array());

    let value = eval(scope, context, "new Uint16Array([])").unwrap();
    assert!(value.is_uint16_array());
    assert!(value.is_array_buffer_view());
    assert!(value.is_typed_array());

    let value = eval(scope, context, "new Int16Array([])").unwrap();
    assert!(value.is_int16_array());
    assert!(value.is_array_buffer_view());
    assert!(value.is_typed_array());

    let value = eval(scope, context, "new Uint32Array([])").unwrap();
    assert!(value.is_uint32_array());
    assert!(value.is_array_buffer_view());
    assert!(value.is_typed_array());

    let value = eval(scope, context, "new Int32Array([])").unwrap();
    assert!(value.is_int32_array());
    assert!(value.is_array_buffer_view());
    assert!(value.is_typed_array());

    let value = eval(scope, context, "new Float32Array([])").unwrap();
    assert!(value.is_float32_array());
    assert!(value.is_array_buffer_view());
    assert!(value.is_typed_array());

    let value = eval(scope, context, "new Float64Array([])").unwrap();
    assert!(value.is_float64_array());
    assert!(value.is_array_buffer_view());
    assert!(value.is_typed_array());

    let value = eval(scope, context, "new BigInt64Array([])").unwrap();
    assert!(value.is_big_int64_array());
    assert!(value.is_array_buffer_view());
    assert!(value.is_typed_array());

    let value = eval(scope, context, "new BigUint64Array([])").unwrap();
    assert!(value.is_big_uint64_array());
    assert!(value.is_array_buffer_view());
    assert!(value.is_typed_array());

    let value = eval(scope, context, "new SharedArrayBuffer(64)").unwrap();
    assert!(value.is_shared_array_buffer());

    let value = eval(scope, context, "new Proxy({},{})").unwrap();
    assert!(value.is_proxy());

    // Other checker, Just check if it can be called
    value.is_external();
    value.is_web_assembly_compiled_module();
    value.is_module_namespace_object();
  }
}

#[test]
fn try_from_local() {
  let _setup_guard = setup();
  let mut params = v8::Isolate::create_params();
  params.set_array_buffer_allocator(v8::new_default_allocator());
  let isolate = v8::Isolate::new(params);
  let mut locker = v8::Locker::new(&isolate);
  let scope = locker.enter();
  {
    let mut hs = v8::HandleScope::new(scope);
    let scope = hs.enter();
    let context = v8::Context::new(scope);
    let mut cs = v8::ContextScope::new(scope, context);
    let scope = cs.enter();

    {
      let value: v8::Local<v8::Value> = v8::undefined(scope).into();
      let _primitive = v8::Local::<v8::Primitive>::try_from(value).unwrap();
      assert_eq!(
        v8::Local::<v8::Object>::try_from(value)
          .err()
          .unwrap()
          .to_string(),
        "Object expected"
      );
      assert_eq!(
        v8::Local::<v8::Int32>::try_from(value)
          .err()
          .unwrap()
          .to_string(),
        "Int32 expected"
      );
    }

    {
      let value: v8::Local<v8::Value> = v8::Boolean::new(scope, true).into();
      let primitive = v8::Local::<v8::Primitive>::try_from(value).unwrap();
      let _boolean = v8::Local::<v8::Boolean>::try_from(value).unwrap();
      let _boolean = v8::Local::<v8::Boolean>::try_from(primitive).unwrap();
      assert_eq!(
        v8::Local::<v8::String>::try_from(value)
          .err()
          .unwrap()
          .to_string(),
        "String expected"
      );
      assert_eq!(
        v8::Local::<v8::Number>::try_from(primitive)
          .err()
          .unwrap()
          .to_string(),
        "Number expected"
      );
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
      assert_eq!(
        v8::Local::<v8::String>::try_from(value)
          .err()
          .unwrap()
          .to_string(),
        "String expected"
      );
      assert_eq!(
        v8::Local::<v8::Boolean>::try_from(primitive)
          .err()
          .unwrap()
          .to_string(),
        "Boolean expected"
      );
      assert_eq!(
        v8::Local::<v8::Uint32>::try_from(integer)
          .err()
          .unwrap()
          .to_string(),
        "Uint32 expected"
      );
    }

    {
      let value: v8::Local<v8::Value> =
        eval(scope, context, "(() => {})").unwrap();
      let object = v8::Local::<v8::Object>::try_from(value).unwrap();
      let _function = v8::Local::<v8::Function>::try_from(value).unwrap();
      let _function = v8::Local::<v8::Function>::try_from(object).unwrap();
      assert_eq!(
        v8::Local::<v8::Primitive>::try_from(value)
          .err()
          .unwrap()
          .to_string(),
        "Primitive expected"
      );
      assert_eq!(
        v8::Local::<v8::BigInt>::try_from(value)
          .err()
          .unwrap()
          .to_string(),
        "BigInt expected"
      );
      assert_eq!(
        v8::Local::<v8::NumberObject>::try_from(value)
          .err()
          .unwrap()
          .to_string(),
        "NumberObject expected"
      );
      assert_eq!(
        v8::Local::<v8::NumberObject>::try_from(object)
          .err()
          .unwrap()
          .to_string(),
        "NumberObject expected"
      );
      assert_eq!(
        v8::Local::<v8::Set>::try_from(value)
          .err()
          .unwrap()
          .to_string(),
        "Set expected"
      );
      assert_eq!(
        v8::Local::<v8::Set>::try_from(object)
          .err()
          .unwrap()
          .to_string(),
        "Set expected"
      );
    }
  }
}

#[test]
fn inspector_dispatch_protocol_message() {
  let _setup_guard = setup();
  let mut params = v8::Isolate::create_params();
  params.set_array_buffer_allocator(v8::new_default_allocator());
  let mut isolate = v8::Isolate::new(params);
  let mut locker = v8::Locker::new(&isolate);
  let scope = locker.enter();

  use v8::inspector::*;

  struct Client {
    base: V8InspectorClientBase,
  }

  impl Client {
    fn new() -> Self {
      Self {
        base: V8InspectorClientBase::new::<Self>(),
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
  }

  struct TestChannel {
    base: ChannelBase,
    send_response_count: usize,
    send_notification_count: usize,
    flush_protocol_notifications_count: usize,
  }

  impl TestChannel {
    pub fn new() -> Self {
      Self {
        base: ChannelBase::new::<Self>(),
        send_response_count: 0,
        send_notification_count: 0,
        flush_protocol_notifications_count: 0,
      }
    }
  }

  impl ChannelImpl for TestChannel {
    fn base(&self) -> &ChannelBase {
      &self.base
    }
    fn base_mut(&mut self) -> &mut ChannelBase {
      &mut self.base
    }
    fn send_response(
      &mut self,
      _call_id: i32,
      _message: v8::UniquePtr<StringBuffer>,
    ) {
      self.send_response_count += 1;
    }
    fn send_notification(&mut self, _message: v8::UniquePtr<StringBuffer>) {
      self.send_notification_count += 1;
    }
    fn flush_protocol_notifications(&mut self) {
      self.flush_protocol_notifications_count += 1;
    }
  }

  let mut hs = v8::HandleScope::new(scope);
  let scope = hs.enter();
  let context = v8::Context::new(scope);
  let mut cs = v8::ContextScope::new(scope, context);
  let _scope = cs.enter();

  let mut default_client = Client::new();
  let mut inspector = V8Inspector::create(&mut isolate, &mut default_client);
  let name = b"";
  let name_view = StringView::from(&name[..]);
  inspector.context_created(context, 1, &name_view);
  let mut channel = TestChannel::new();
  let state = b"{}";
  let state_view = StringView::from(&state[..]);
  let mut session = inspector.connect(1, &mut channel, &state_view);
  let message = String::from(
    r#"{"id":1,"method":"Network.enable","params":{"maxPostDataSize":65536}}"#,
  );
  let message = &message.into_bytes()[..];
  let string_view = StringView::from(message);
  session.dispatch_protocol_message(&string_view);
  assert_eq!(channel.send_response_count, 1);
  assert_eq!(channel.send_notification_count, 0);
  assert_eq!(channel.flush_protocol_notifications_count, 0);
}

#[test]
fn inspector_console_api_message() {
  let _setup_guard = setup();
  let mut params = v8::Isolate::create_params();
  params.set_array_buffer_allocator(v8::new_default_allocator());
  let mut isolate = v8::Isolate::new(params);
  let mut locker = v8::Locker::new(&isolate);
  let scope = locker.enter();

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

  let mut hs = v8::HandleScope::new(scope);
  let scope = hs.enter();
  let context = v8::Context::new(scope);
  let mut cs = v8::ContextScope::new(scope, context);
  let scope = cs.enter();

  let mut client = Client::new();
  let mut inspector = V8Inspector::create(&mut isolate, &mut client);
  let name = b"";
  let name_view = StringView::from(&name[..]);
  inspector.context_created(context, 1, &name_view);

  let source = r#"
    console.log("one");
    console.error("two");
    console.trace("three");
  "#;
  let _ = eval(scope, context, source).unwrap();
  assert_eq!(client.messages, vec!["one", "two", "three"]);
}

#[test]
fn context_from_object_template() {
  let _setup_guard = setup();
  let mut params = v8::Isolate::create_params();
  params.set_array_buffer_allocator(v8::new_default_allocator());
  let isolate = v8::Isolate::new(params);
  let mut locker = v8::Locker::new(&isolate);
  let scope = locker.enter();
  {
    let mut hs = v8::HandleScope::new(scope);
    let scope = hs.enter();
    let object_templ = v8::ObjectTemplate::new(scope);
    let function_templ = v8::FunctionTemplate::new(scope, fortytwo_callback);
    let name = v8_str(scope, "f");
    object_templ.set(name.into(), function_templ.into());
    let context = v8::Context::new_from_template(scope, object_templ);
    let mut cs = v8::ContextScope::new(scope, context);
    let scope = cs.enter();
    let actual = eval(scope, context, "f()").unwrap();
    let expected = v8::Integer::new(scope, 42);
    assert!(expected.strict_equals(actual));
  }
}
