// Copyright 2019-2020 the Deno authors. All rights reserved. MIT license.

#[macro_use]
extern crate lazy_static;

use rusty_v8 as v8;
use rusty_v8::{new_null, FunctionCallbackInfo, InIsolate, Local, ToLocal};
use std::convert::Into;
use std::sync::atomic::{AtomicUsize, Ordering};
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
  params.set_array_buffer_allocator(v8::new_default_allocator());
  let isolate = v8::Isolate::new(params);
  let mut locker = v8::Locker::new(&isolate);
  {
    let mut hs = v8::HandleScope::new(&mut locker);
    let scope1 = hs.enter();
    {
      let mut hs = v8::HandleScope::new(scope1);
      let _scope2 = hs.enter();
    }
  }
  drop(locker);
  drop(g);
}

#[test]
#[allow(clippy::float_cmp)]
fn handle_scope_numbers() {
  let g = setup();
  let mut params = v8::Isolate::create_params();
  params.set_array_buffer_allocator(v8::new_default_allocator());
  let isolate = v8::Isolate::new(params);
  let mut locker = v8::Locker::new(&isolate);
  {
    let mut hs = v8::HandleScope::new(&mut locker);
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
  drop(locker);
  drop(g);
}

#[test]
fn global_handles() {
  let _g = setup();
  let mut params = v8::Isolate::create_params();
  params.set_array_buffer_allocator(v8::new_default_allocator());
  let isolate = v8::Isolate::new(params);
  let mut locker = v8::Locker::new(&isolate);
  let mut g1 = v8::Global::<v8::String>::new();
  let mut g2 = v8::Global::<v8::Integer>::new();
  let mut g3 = v8::Global::<v8::Integer>::new();
  let mut _g4 = v8::Global::<v8::Integer>::new();
  let g5 = v8::Global::<v8::Script>::new();
  {
    let mut hs = v8::HandleScope::new(&mut locker);
    let scope = hs.enter();
    let l1 = v8::String::new(scope, "bla").unwrap();
    let l2 = v8::Integer::new(scope, 123);
    g1.set(scope, l1);
    g2.set(scope, l2);
    g3.set(scope, &g2);
    _g4 = v8::Global::new_from(scope, l2);
  }
  {
    let mut hs = v8::HandleScope::new(&mut locker);
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
  }
  g1.reset(&mut locker);
  assert!(g1.is_empty());
  g2.reset(&mut locker);
  assert!(g2.is_empty());
  g3.reset(&mut locker);
  assert!(g3.is_empty());
  _g4.reset(&mut locker);
  assert!(_g4.is_empty());
  assert!(g5.is_empty());
}

#[test]
fn test_string() {
  setup();
  let mut params = v8::Isolate::create_params();
  params.set_array_buffer_allocator(v8::new_default_allocator());
  let isolate = v8::Isolate::new(params);
  let mut locker = v8::Locker::new(&isolate);
  {
    let mut hs = v8::HandleScope::new(&mut locker);
    let scope = hs.enter();
    let reference = "Hello 🦕 world!";
    let local = v8::String::new(scope, reference).unwrap();
    assert_eq!(15, local.length());
    assert_eq!(17, local.utf8_length(scope));
    assert_eq!(reference, local.to_rust_string_lossy(scope));
  }
  drop(locker);
}

#[test]
#[allow(clippy::float_cmp)]
fn escapable_handle_scope() {
  let g = setup();
  let mut params = v8::Isolate::create_params();
  params.set_array_buffer_allocator(v8::new_default_allocator());
  let mut isolate = v8::Isolate::new(params);
  let mut locker = v8::Locker::new(&isolate);
  isolate.enter();
  {
    let mut hs = v8::HandleScope::new(&mut locker);
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
  drop(locker);
  isolate.exit();
  drop(g);
}

#[test]
fn microtasks() {
  setup();
  let mut params = v8::Isolate::create_params();
  params.set_array_buffer_allocator(v8::new_default_allocator());
  let isolate = v8::Isolate::new(params);

  isolate.run_microtasks();

  let mut locker = v8::Locker::new(&isolate);
  {
    let mut hs = v8::HandleScope::new(&mut locker);
    let scope = hs.enter();
    let mut context = v8::Context::new(scope);
    context.enter();

    static CALL_COUNT: AtomicUsize = AtomicUsize::new(0);
    extern "C" fn cb(_info: &FunctionCallbackInfo) {
      CALL_COUNT.fetch_add(1, Ordering::SeqCst);
    }
    let function = v8::Function::new(scope, context, cb).unwrap();

    assert_eq!(CALL_COUNT.load(Ordering::SeqCst), 0);

    isolate.enqueue_microtask(function);
    isolate.run_microtasks();

    assert_eq!(CALL_COUNT.load(Ordering::SeqCst), 1);
    context.exit();
  }
}

#[test]
fn array_buffer() {
  setup();
  let mut params = v8::Isolate::create_params();
  params.set_array_buffer_allocator(v8::new_default_allocator());
  let isolate = v8::Isolate::new(params);
  let mut locker = v8::Locker::new(&isolate);
  {
    let mut hs = v8::HandleScope::new(&mut locker);
    let scope = hs.enter();
    let mut context = v8::Context::new(scope);
    context.enter();

    let ab = v8::ArrayBuffer::new(scope, 42);
    assert_eq!(42, ab.byte_length());

    let bs = v8::ArrayBuffer::new_backing_store(scope, 84);
    assert_eq!(84, bs.byte_length());
    assert_eq!(false, bs.is_shared());

    context.exit();
  }
  drop(locker);
}

#[test]
fn array_buffer_with_shared_backing_store() {
  setup();
  let mut params = v8::Isolate::create_params();
  params.set_array_buffer_allocator(v8::new_default_allocator());
  let isolate = v8::Isolate::new(params);
  let mut locker = v8::Locker::new(&isolate);
  {
    let mut hs = v8::HandleScope::new(&mut locker);
    let scope = hs.enter();

    let mut context = v8::Context::new(scope);
    context.enter();

    let ab1 = v8::ArrayBuffer::new(scope, 42);
    assert_eq!(42, ab1.byte_length());

    let bs1 = ab1.get_backing_store();
    assert_eq!(ab1.byte_length(), bs1.byte_length());
    assert_eq!(2, v8::SharedRef::use_count(&bs1));

    let bs2 = ab1.get_backing_store();
    assert_eq!(ab1.byte_length(), bs2.byte_length());
    assert_eq!(3, v8::SharedRef::use_count(&bs1));
    assert_eq!(3, v8::SharedRef::use_count(&bs2));

    let mut bs3 = ab1.get_backing_store();
    assert_eq!(ab1.byte_length(), bs3.byte_length());
    assert_eq!(4, v8::SharedRef::use_count(&bs1));
    assert_eq!(4, v8::SharedRef::use_count(&bs2));
    assert_eq!(4, v8::SharedRef::use_count(&bs3));

    drop(bs2);
    assert_eq!(3, v8::SharedRef::use_count(&bs1));
    assert_eq!(3, v8::SharedRef::use_count(&bs3));

    drop(bs1);
    assert_eq!(2, v8::SharedRef::use_count(&bs3));

    let ab2 = v8::ArrayBuffer::new_with_backing_store(scope, &mut bs3);
    assert_eq!(ab1.byte_length(), ab2.byte_length());
    assert_eq!(3, v8::SharedRef::use_count(&bs3));

    let bs4 = ab2.get_backing_store();
    assert_eq!(ab2.byte_length(), bs4.byte_length());
    assert_eq!(4, v8::SharedRef::use_count(&bs4));
    assert_eq!(4, v8::SharedRef::use_count(&bs3));

    drop(bs3);
    assert_eq!(3, v8::SharedRef::use_count(&bs4));

    context.exit();
  }
}

fn v8_str<'sc>(
  scope: &mut impl v8::ToLocal<'sc>,
  s: &str,
) -> v8::Local<'sc, v8::String> {
  v8::String::new(scope, s).unwrap()
}

fn eval<'sc>(
  scope: &mut impl v8::InIsolate,
  context: Local<v8::Context>,
  code: &'static str,
) -> Option<Local<'sc, v8::Value>> {
  let mut hs = v8::EscapableHandleScope::new(scope);
  let scope = hs.enter();
  let source = v8_str(scope, code);
  let mut script =
    v8::Script::compile(&mut *scope, context, source, None).unwrap();
  let r = script.run(scope, context);
  r.map(|v| scope.escape(v))
}

#[test]
fn try_catch() {
  let _g = setup();
  let mut params = v8::Isolate::create_params();
  params.set_array_buffer_allocator(v8::new_default_allocator());
  let isolate = v8::Isolate::new(params);
  let mut locker = v8::Locker::new(&isolate);
  {
    let mut hs = v8::HandleScope::new(&mut locker);
    let scope = hs.enter();
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
  }
}

#[test]
fn throw_exception() {
  let _g = setup();
  let mut params = v8::Isolate::create_params();
  params.set_array_buffer_allocator(v8::new_default_allocator());
  let isolate = v8::Isolate::new(params);
  let mut locker = v8::Locker::new(&isolate);
  {
    let mut hs = v8::HandleScope::new(&mut locker);
    let scope = hs.enter();
    let mut context = v8::Context::new(scope);
    context.enter();
    {
      let mut try_catch = v8::TryCatch::new(scope);
      let tc = try_catch.enter();
      isolate.throw_exception(v8_str(scope, "boom").into());
      assert!(tc.has_caught());
      assert!(tc
        .exception()
        .unwrap()
        .strict_equals(v8_str(scope, "boom").into()));
    };
    context.exit();
  }
}

#[test]
fn terminate_execution() {
  let g = setup();
  let mut params = v8::Isolate::create_params();
  params.set_array_buffer_allocator(v8::new_default_allocator());
  let isolate = v8::Isolate::new(params);
  let mut locker = v8::Locker::new(&isolate);
  // Originally run fine.
  {
    let mut hs = v8::HandleScope::new(&mut locker);
    let scope = hs.enter();
    let mut context = v8::Context::new(scope);
    context.enter();
    let result = eval(scope, context, "true").unwrap();
    let true_val = v8::new_true(scope).into();
    assert!(result.same_value(true_val));
    context.exit();
  }
  // Terminate.
  isolate.terminate_execution();
  // Below run should fail with terminated knowledge.
  {
    let mut hs = v8::HandleScope::new(&mut locker);
    let scope = hs.enter();
    let mut context = v8::Context::new(scope);
    context.enter();
    let mut try_catch = v8::TryCatch::new(scope);
    let tc = try_catch.enter();
    let _ = eval(scope, context, "true");
    assert!(tc.has_caught());
    assert!(tc.has_terminated());
    context.exit();
  }
  // Cancel termination.
  isolate.cancel_terminate_execution();
  // Works again.
  {
    let mut hs = v8::HandleScope::new(&mut locker);
    let scope = hs.enter();
    let mut context = v8::Context::new(scope);
    context.enter();
    let result = eval(scope, context, "true").unwrap();
    let true_val = v8::new_true(scope).into();
    assert!(result.same_value(true_val));
    context.exit();
  }
  drop(locker);
  drop(g);
}

#[test]
fn add_message_listener() {
  let g = setup();
  let mut params = v8::Isolate::create_params();
  params.set_array_buffer_allocator(v8::new_default_allocator());
  let mut isolate = v8::Isolate::new(params);
  isolate.set_capture_stack_trace_for_uncaught_exceptions(true, 32);

  static CALL_COUNT: AtomicUsize = AtomicUsize::new(0);

  extern "C" fn check_message_0(
    mut message: Local<v8::Message>,
    _exception: Local<v8::Value>,
  ) {
    let isolate = message.get_isolate();
    let context = isolate.get_current_context();
    let mut cbs = v8::CallbackScope::new(message);
    let scope = cbs.enter();
    let mut hs = v8::HandleScope::new(scope);
    let scope = hs.enter();
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
  {
    let mut hs = v8::HandleScope::new(&mut locker);
    let s = hs.enter();
    let mut context = v8::Context::new(s);
    context.enter();
    let source = v8::String::new(s, "throw 'foo'").unwrap();
    let mut script = v8::Script::compile(s, context, source, None).unwrap();
    assert!(script.run(s, context).is_none());
    assert_eq!(CALL_COUNT.load(Ordering::SeqCst), 1);
    context.exit();
  }
  drop(locker);
  drop(g);
}

fn unexpected_module_resolve_callback(
  _context: v8::Local<v8::Context>,
  _specifier: v8::Local<v8::String>,
  _referrer: v8::Local<v8::Module>,
) -> *mut v8::Module {
  unreachable!()
}

#[test]
fn set_host_initialize_import_meta_object_callback() {
  let g = setup();
  let mut params = v8::Isolate::create_params();
  params.set_array_buffer_allocator(v8::new_default_allocator());
  let mut isolate = v8::Isolate::new(params);

  static CALL_COUNT: AtomicUsize = AtomicUsize::new(0);

  extern "C" fn callback(
    context: Local<v8::Context>,
    _module: Local<v8::Module>,
    meta: Local<v8::Object>,
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
  {
    let mut hs = v8::HandleScope::new(&mut locker);
    let s = hs.enter();
    let mut context = v8::Context::new(s);
    context.enter();
    let source = mock_source(s, "google.com", "import.meta;");
    let mut module =
      v8::script_compiler::compile_module(&isolate, source).unwrap();
    let result =
      module.instantiate_module(context, unexpected_module_resolve_callback);
    assert!(result.is_some());
    let meta = module.evaluate(s, context).unwrap();
    assert!(meta.is_object());
    let meta = meta.to_object(s).unwrap();
    let key = v8::String::new(s, "foo").unwrap();
    let expected = v8::String::new(s, "bar").unwrap();
    let actual = meta.get(s, context, key.into()).unwrap();
    assert!(expected.strict_equals(actual));
    assert_eq!(CALL_COUNT.load(Ordering::SeqCst), 1);

    context.exit();
  }
  drop(locker);
  drop(g);
}

#[test]
fn script_compile_and_run() {
  setup();
  let mut params = v8::Isolate::create_params();
  params.set_array_buffer_allocator(v8::new_default_allocator());
  let isolate = v8::Isolate::new(params);
  let mut locker = v8::Locker::new(&isolate);
  {
    let mut hs = v8::HandleScope::new(&mut locker);
    let s = hs.enter();
    let mut context = v8::Context::new(s);
    context.enter();
    let source = v8::String::new(s, "'Hello ' + 13 + 'th planet'").unwrap();
    let mut script = v8::Script::compile(s, context, source, None).unwrap();
    source.to_rust_string_lossy(s);
    let result = script.run(s, context).unwrap();
    let result = result.to_string(s).unwrap();
    assert_eq!(result.to_rust_string_lossy(s), "Hello 13th planet");
    context.exit();
  }
  drop(locker);
}

#[test]
fn script_origin() {
  setup();
  let mut params = v8::Isolate::create_params();
  params.set_array_buffer_allocator(v8::new_default_allocator());
  let isolate = v8::Isolate::new(params);
  let mut locker = v8::Locker::new(&isolate);

  {
    let mut hs = v8::HandleScope::new(&mut locker);
    let s = hs.enter();
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
  }
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
  params.set_array_buffer_allocator(v8::new_default_allocator());
  let isolate = v8::Isolate::new(params);
  let mut locker = v8::Locker::new(&isolate);
  {
    let mut hs = v8::HandleScope::new(&mut locker);
    let scope = hs.enter();
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
  }
  drop(locker);
}

#[test]
fn exception() {
  setup();
  let mut params = v8::Isolate::create_params();
  params.set_array_buffer_allocator(v8::new_default_allocator());
  let mut isolate = v8::Isolate::new(params);
  let mut locker = v8::Locker::new(&isolate);
  isolate.enter();
  {
    let mut hs = v8::HandleScope::new(&mut locker);
    let scope = hs.enter();
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
  }
  drop(locker);
  isolate.exit();
}

#[test]
fn json() {
  setup();
  let mut params = v8::Isolate::create_params();
  params.set_array_buffer_allocator(v8::new_default_allocator());
  let isolate = v8::Isolate::new(params);
  let mut locker = v8::Locker::new(&isolate);
  {
    let mut hs = v8::HandleScope::new(&mut locker);
    let s = hs.enter();
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
  }
  drop(locker);
}

#[test]
fn object() {
  setup();
  let mut params = v8::Isolate::create_params();
  params.set_array_buffer_allocator(v8::new_default_allocator());
  let isolate = v8::Isolate::new(params);
  let mut locker = v8::Locker::new(&isolate);
  {
    let mut hs = v8::HandleScope::new(&mut locker);
    let scope = hs.enter();
    let mut context = v8::Context::new(scope);
    context.enter();
    let null: v8::Local<v8::Value> = new_null(scope).into();
    let s1 = v8::String::new(scope, "a").unwrap();
    let s2 = v8::String::new(scope, "b").unwrap();
    let name1 = s1.into();
    let name2 = s2.into();
    let names = vec![name1, name2];
    let v1: v8::Local<v8::Value> = v8::Number::new(scope, 1.0).into();
    let v2: v8::Local<v8::Value> = v8::Number::new(scope, 2.0).into();
    let values = vec![v1, v2];
    let object = v8::Object::new2(scope, null, names, values);
    assert!(!object.is_null_or_undefined());

    let object_ = v8::Object::new(scope);
    assert!(!object_.is_null_or_undefined());
    let id = object_.get_identity_hash();
    assert_ne!(id, 0);
    context.exit();
  }
  drop(locker);
}

#[test]
fn array() {
  setup();
  let mut params = v8::Isolate::create_params();
  params.set_array_buffer_allocator(v8::new_default_allocator());
  let isolate = v8::Isolate::new(params);
  let mut locker = v8::Locker::new(&isolate);
  {
    let mut hs = v8::HandleScope::new(&mut locker);
    let s = hs.enter();
    let mut context = v8::Context::new(s);
    context.enter();
    let s1 = v8::String::new(s, "a").unwrap();
    let index1 = v8::Integer::new(s, 0);
    let s2 = v8::String::new(s, "b").unwrap();
    let index2 = v8::Integer::new(s, 1);
    let array = v8::Array::new(s, 2);
    array.set(context, index1.into(), s1.into());
    array.set(context, index2.into(), s2.into());

    let maybe_v1 = array.get(s, context, index1.into());
    assert!(maybe_v1.is_some());
    assert!(maybe_v1.unwrap().same_value(s1.into()));
    let maybe_v2 = array.get(s, context, index2.into());
    assert!(maybe_v2.is_some());
    assert!(maybe_v2.unwrap().same_value(s2.into()));

    context.exit();
  }
  drop(locker);
}

#[test]
fn create_data_property() {
  setup();
  let mut params = v8::Isolate::create_params();
  params.set_array_buffer_allocator(v8::new_default_allocator());
  let isolate = v8::Isolate::new(params);
  let mut locker = v8::Locker::new(&isolate);
  {
    let mut hs = v8::HandleScope::new(&mut locker);
    let scope = hs.enter();
    let mut context = v8::Context::new(scope);
    context.enter();

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
    assert_eq!(
      obj.create_data_property(context, key.into(), value.into()),
      v8::MaybeBool::JustTrue
    );
    let actual = obj.get(scope, context, key.into()).unwrap();
    assert!(value.strict_equals(actual));

    let key2 = v8_str(scope, "foo2");
    assert_eq!(
      obj.set(context, key2.into(), value.into()),
      v8::MaybeBool::JustTrue
    );
    let actual = obj.get(scope, context, key2.into()).unwrap();
    assert!(value.strict_equals(actual));

    context.exit();
  }
  drop(locker);
}

#[test]
fn object_set_accessor() {
  setup();
  let mut params = v8::Isolate::create_params();
  params.set_array_buffer_allocator(v8::new_default_allocator());
  let isolate = v8::Isolate::new(params);
  let mut locker = v8::Locker::new(&isolate);
  {
    let mut hs = v8::HandleScope::new(&mut locker);
    let scope = hs.enter();
    let mut context = v8::Context::new(scope);
    context.enter();
    let mut obj = v8::Object::new(scope);
    let key = v8_str(scope, "key");
    static CALL_COUNT: AtomicUsize = AtomicUsize::new(0);
    extern "C" fn getter(
      name: v8::Local<v8::Name>,
      info: &v8::PropertyCallbackInfo,
    ) {
      let rv = &mut info.get_return_value();
      // TODO fix callback mutability.
      #[allow(mutable_transmutes)]
      #[allow(clippy::transmute_ptr_to_ptr)]
      let info: &mut v8::PropertyCallbackInfo =
        unsafe { std::mem::transmute(info) };
      {
        let mut hs = v8::HandleScope::new(info);
        let scope = hs.enter();
        let name_str =
          name.to_string(scope).unwrap().to_rust_string_lossy(scope);
        assert_eq!(name_str, "key");
        let s = v8::String::new(scope, "hello").unwrap();
        rv.set(s.into());
      }
      CALL_COUNT.fetch_add(1, Ordering::SeqCst);
    }
    obj.set_accessor(context, key.into(), getter);
    let global = context.global(scope);
    let obj_name = v8_str(scope, "obj");
    global.set(context, obj_name.into(), obj.into());
    let actual = eval(scope, context, "obj.key;").unwrap();
    let expected = v8_str(scope, "hello");
    assert!(actual.strict_equals(expected.into()));
    assert_eq!(CALL_COUNT.load(Ordering::SeqCst), 1);
    context.exit();
  }
  drop(locker);
}

#[test]
fn promise_resolved() {
  setup();
  let mut params = v8::Isolate::create_params();
  params.set_array_buffer_allocator(v8::new_default_allocator());
  let isolate = v8::Isolate::new(params);
  let mut locker = v8::Locker::new(&isolate);
  {
    let mut hs = v8::HandleScope::new(&mut locker);
    let scope = hs.enter();
    let mut context = v8::Context::new(scope);
    context.enter();
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
    context.exit();
  }
  drop(locker);
}

#[test]
fn promise_rejected() {
  setup();
  let mut params = v8::Isolate::create_params();
  params.set_array_buffer_allocator(v8::new_default_allocator());
  let isolate = v8::Isolate::new(params);
  let mut locker = v8::Locker::new(&isolate);
  {
    let mut hs = v8::HandleScope::new(&mut locker);
    let scope = hs.enter();
    let mut context = v8::Context::new(scope);
    context.enter();
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
    context.exit();
  }
  drop(locker);
}

extern "C" fn fn_callback(info: &FunctionCallbackInfo) {
  assert_eq!(info.length(), 0);
  {
    let rv = &mut info.get_return_value();
    #[allow(mutable_transmutes)]
    #[allow(clippy::transmute_ptr_to_ptr)]
    let info: &mut FunctionCallbackInfo = unsafe { std::mem::transmute(info) };
    {
      let mut hs = v8::HandleScope::new(info);
      let scope = hs.enter();
      let s = v8::String::new(scope, "Hello callback!").unwrap();
      let value: Local<v8::Value> = s.into();
      let rv_value = rv.get(scope);
      assert!(rv_value.is_undefined());
      rv.set(value);
    }
  }
}

extern "C" fn fn_callback2(info: &FunctionCallbackInfo) {
  #[allow(mutable_transmutes)]
  #[allow(clippy::transmute_ptr_to_ptr)]
  let info: &mut FunctionCallbackInfo = unsafe { std::mem::transmute(info) };
  assert_eq!(info.length(), 2);
  let isolate = info.get_isolate();
  let mut locker = v8::Locker::new(&isolate);
  let mut context = isolate.get_current_context();
  let arg1 = info.get_argument(0);
  let arg2 = info.get_argument(1);
  let rv = &mut info.get_return_value();
  {
    let mut hs = v8::HandleScope::new(&mut locker);
    let scope = hs.enter();
    context.enter();

    let arg1_val = v8::String::new(scope, "arg1").unwrap();
    assert!(arg1.is_string());
    assert!(arg1.strict_equals(arg1_val.into()));

    let arg2_val = v8::Integer::new(scope, 2);
    assert!(arg2.is_number());
    assert!(arg2.strict_equals(arg2_val.into()));

    let s = v8::String::new(scope, "Hello callback!").unwrap();
    let value: Local<v8::Value> = s.into();
    let rv_value = rv.get(scope);
    assert!(rv_value.is_undefined());
    rv.set(value);
    context.exit();
  }
}

#[test]
fn function() {
  setup();
  let mut params = v8::Isolate::create_params();
  params.set_array_buffer_allocator(v8::new_default_allocator());
  let isolate = v8::Isolate::new(params);
  let mut locker = v8::Locker::new(&isolate);

  {
    let mut hs = v8::HandleScope::new(&mut locker);
    let scope = hs.enter();
    let mut context = v8::Context::new(scope);
    context.enter();
    let global = context.global(scope);
    let recv: Local<v8::Value> = global.into();
    // create function using template
    let mut fn_template = v8::FunctionTemplate::new(scope, fn_callback);
    let mut function = fn_template
      .get_function(scope, context)
      .expect("Unable to create function");
    let _value =
      v8::Function::call(&mut *function, scope, context, recv, 0, vec![]);
    // create function without a template
    let mut function = v8::Function::new(scope, context, fn_callback2)
      .expect("Unable to create function");
    let arg1 = v8::String::new(scope, "arg1").unwrap();
    let arg2 = v8::Integer::new(scope, 2);
    let maybe_value = v8::Function::call(
      &mut *function,
      scope,
      context,
      recv,
      2,
      vec![arg1.into(), arg2.into()],
    );
    let value = maybe_value.unwrap();
    let value_str = value.to_string(scope).unwrap();
    let rust_str = value_str.to_rust_string_lossy(scope);
    assert_eq!(rust_str, "Hello callback!".to_string());
    context.exit();
  }
  drop(locker);
}

extern "C" fn promise_reject_callback(msg: v8::PromiseRejectMessage) {
  let event = msg.get_event();
  assert_eq!(event, v8::PromiseRejectEvent::PromiseRejectWithNoHandler);
  let mut promise = msg.get_promise();
  assert_eq!(promise.state(), v8::PromiseState::Rejected);
  let mut promise_obj: Local<v8::Object> = promise.into();
  let isolate = promise_obj.get_isolate();
  let value = msg.get_value();
  let mut locker = v8::Locker::new(isolate);
  {
    let mut hs = v8::HandleScope::new(&mut locker);
    let scope = hs.enter();
    let value_str = value.to_string(scope).unwrap();
    let rust_str = value_str.to_rust_string_lossy(scope);
    assert_eq!(rust_str, "promise rejected".to_string());
  }
  drop(locker);
}

#[test]
fn set_promise_reject_callback() {
  setup();
  let mut params = v8::Isolate::create_params();
  params.set_array_buffer_allocator(v8::new_default_allocator());
  let mut isolate = v8::Isolate::new(params);
  isolate.set_promise_reject_callback(promise_reject_callback);
  isolate.enter();
  let mut locker = v8::Locker::new(&isolate);
  {
    let mut hs = v8::HandleScope::new(&mut locker);
    let scope = hs.enter();
    let mut context = v8::Context::new(scope);
    context.enter();
    let mut resolver = v8::PromiseResolver::new(scope, context).unwrap();
    let value = v8::String::new(scope, "promise rejected").unwrap();
    resolver.reject(context, value.into());
    context.exit();
  }
  drop(locker);
  isolate.exit();
}

fn mock_script_origin<'sc>(
  scope: &mut impl v8::ToLocal<'sc>,
  resource_name_: &str,
) -> v8::ScriptOrigin<'sc> {
  let resource_name = v8_str(scope, resource_name_);
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

fn mock_source<'sc>(
  scope: &mut impl ToLocal<'sc>,
  resource_name: &str,
  source: &str,
) -> v8::script_compiler::Source {
  let source_str = v8_str(scope, source);
  let script_origin = mock_script_origin(scope, resource_name);
  v8::script_compiler::Source::new(source_str, &script_origin)
}

#[test]
fn script_compiler_source() {
  let g = setup();
  let mut params = v8::Isolate::create_params();
  params.set_array_buffer_allocator(v8::new_default_allocator());
  let mut isolate = v8::Isolate::new(params);
  isolate.set_promise_reject_callback(promise_reject_callback);
  isolate.enter();
  let mut locker = v8::Locker::new(&isolate);
  {
    let mut hs = v8::HandleScope::new(&mut locker);
    let scope = hs.enter();
    let mut context = v8::Context::new(scope);
    context.enter();

    let source = "1+2";
    let script_origin = mock_script_origin(scope, "foo.js");
    let source =
      v8::script_compiler::Source::new(v8_str(scope, source), &script_origin);

    let result = v8::script_compiler::compile_module(&isolate, source);
    assert!(result.is_some());

    context.exit();
  }
  drop(locker);
  isolate.exit();
  drop(g);
}

#[test]
fn module_instantiation_failures1() {
  let g = setup();
  let mut params = v8::Isolate::create_params();
  params.set_array_buffer_allocator(v8::new_default_allocator());
  let mut isolate = v8::Isolate::new(params);
  isolate.enter();
  let mut locker = v8::Locker::new(&isolate);
  {
    let mut hs = v8::HandleScope::new(&mut locker);
    let scope = hs.enter();
    let mut context = v8::Context::new(scope);
    context.enter();

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
      fn resolve_callback(
        context: v8::Local<v8::Context>,
        _specifier: v8::Local<v8::String>,
        _referrer: v8::Local<v8::Module>,
      ) -> *mut v8::Module {
        let mut cbs = v8::CallbackScope::new(context);
        let mut hs = v8::HandleScope::new(cbs.enter());
        let scope = hs.enter();
        let e = v8_str(scope, "boom");
        scope.isolate().throw_exception(e.into());
        std::ptr::null_mut()
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

    context.exit();
  }
  drop(locker);
  isolate.exit();
  drop(g);
}

fn compile_specifier_as_module_resolve_callback(
  context: v8::Local<v8::Context>,
  specifier: v8::Local<v8::String>,
  _referrer: v8::Local<v8::Module>,
) -> *mut v8::Module {
  let mut cbs = v8::CallbackScope::new(context);
  let mut hs = v8::EscapableHandleScope::new(cbs.enter());
  let scope = hs.enter();
  let origin = mock_script_origin(scope, "module.js");
  let source = v8::script_compiler::Source::new(specifier, &origin);
  let module =
    v8::script_compiler::compile_module(scope.isolate(), source).unwrap();
  &mut *scope.escape(module)
}

#[test]
fn module_evaluation() {
  let g = setup();
  let mut params = v8::Isolate::create_params();
  params.set_array_buffer_allocator(v8::new_default_allocator());
  let mut isolate = v8::Isolate::new(params);
  isolate.enter();
  let mut locker = v8::Locker::new(&isolate);
  {
    let mut hs = v8::HandleScope::new(&mut locker);
    let scope = hs.enter();
    let mut context = v8::Context::new(scope);
    context.enter();

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

    context.exit();
  }
  drop(locker);
  isolate.exit();
  drop(g);
}

#[test]
fn primitive_array() {
  let g = setup();
  let mut params = v8::Isolate::create_params();
  params.set_array_buffer_allocator(v8::new_default_allocator());
  let mut isolate = v8::Isolate::new(params);
  isolate.enter();
  let mut locker = v8::Locker::new(&isolate);
  {
    let mut hs = v8::HandleScope::new(&mut locker);
    let scope = hs.enter();
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
    array.set(scope, 1, string.into());
    assert!(array.get(scope, 0).is_undefined());
    assert!(array.get(scope, 1).is_string());

    let num = v8::Number::new(scope, 0.42);
    array.set(scope, 2, num.into());
    assert!(array.get(scope, 0).is_undefined());
    assert!(array.get(scope, 1).is_string());
    assert!(array.get(scope, 2).is_number());

    context.exit();
  }
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
  params.set_array_buffer_allocator(v8::new_default_allocator());
  let mut isolate = v8::Isolate::new(params);
  isolate.enter();
  let mut locker = v8::Locker::new(&isolate);
  {
    let mut hs = v8::HandleScope::new(&mut locker);
    let scope = hs.enter();
    let mut context = v8::Context::new(scope);
    context.enter();

    assert!(v8_str(scope, "a").strict_equals(v8_str(scope, "a").into()));
    assert!(!v8_str(scope, "a").strict_equals(v8_str(scope, "b").into()));

    assert!(v8_str(scope, "a").same_value(v8_str(scope, "a").into()));
    assert!(!v8_str(scope, "a").same_value(v8_str(scope, "b").into()));

    context.exit();
  }
  drop(locker);
  isolate.exit();
  drop(g);
}

#[test]
fn array_buffer_view() {
  let g = setup();
  let mut params = v8::Isolate::create_params();
  params.set_array_buffer_allocator(v8::new_default_allocator());
  let mut isolate = v8::Isolate::new(params);
  isolate.enter();
  let mut locker = v8::Locker::new(&isolate);
  {
    let mut hs = v8::HandleScope::new(&mut locker);
    let s = hs.enter();
    let mut context = v8::Context::new(s);
    context.enter();
    let source = v8::String::new(s, "new Uint8Array([23,23,23,23])").unwrap();
    let mut script = v8::Script::compile(s, context, source, None).unwrap();
    source.to_rust_string_lossy(s);
    let result = script.run(s, context).unwrap();
    // TODO: safer casts.
    let result = unsafe { Local::<v8::ArrayBufferView>::cast(result) };
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
    context.exit();
  }
  drop(locker);
  isolate.exit();
  drop(g);
}

#[test]
fn snapshot_creator() {
  let g = setup();
  // First we create the snapshot, there is a single global variable 'a' set to
  // the value 3.
  let mut startup_data = {
    let mut snapshot_creator = v8::SnapshotCreator::new(None);
    let isolate = snapshot_creator.get_isolate();
    let mut locker = v8::Locker::new(&isolate);
    {
      let mut hs = v8::HandleScope::new(&mut locker);
      let scope = hs.enter();
      let mut context = v8::Context::new(scope);
      context.enter();

      let source = v8::String::new(scope, "a = 1 + 2").unwrap();
      let mut script =
        v8::Script::compile(scope, context, source, None).unwrap();
      script.run(scope, context).unwrap();

      snapshot_creator.set_default_context(context);

      context.exit();
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
    params.set_snapshot_blob(&mut startup_data);
    let isolate = v8::Isolate::new(params);
    let mut locker = v8::Locker::new(&isolate);
    {
      let mut hs = v8::HandleScope::new(&mut locker);
      let scope = hs.enter();
      let mut context = v8::Context::new(scope);
      context.enter();
      let source = v8::String::new(scope, "a === 3").unwrap();
      let mut script =
        v8::Script::compile(scope, context, source, None).unwrap();
      let result = script.run(scope, context).unwrap();
      let true_val = v8::new_true(scope).into();
      assert!(result.same_value(true_val));
      context.exit();
    }
    // TODO(ry) WARNING! startup_data needs to be kept alive as long the isolate
    // using it. See note in CreateParams::set_snapshot_blob
    drop(startup_data);
  }

  drop(g);
}

lazy_static! {
  static ref EXTERNAL_REFERENCES: v8::ExternalReferences =
    v8::ExternalReferences::new(&[fn_callback]);
}

#[test]
fn external_references() {
  let g = setup();
  // First we create the snapshot, there is a single global variable 'a' set to
  // the value 3.
  let mut startup_data = {
    let mut snapshot_creator =
      v8::SnapshotCreator::new(Some(&EXTERNAL_REFERENCES));
    let isolate = snapshot_creator.get_isolate();
    let mut locker = v8::Locker::new(&isolate);
    {
      let mut hs = v8::HandleScope::new(&mut locker);
      let scope = hs.enter();
      let mut context = v8::Context::new(scope);
      context.enter();

      // create function using template
      let mut fn_template = v8::FunctionTemplate::new(scope, fn_callback);
      let function = fn_template
        .get_function(scope, context)
        .expect("Unable to create function");

      let global = context.global(scope);
      global.set(context, v8_str(scope, "F").into(), function.into());

      snapshot_creator.set_default_context(context);

      context.exit();
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
    params.set_snapshot_blob(&mut startup_data);
    params.set_external_references(&EXTERNAL_REFERENCES);
    let isolate = v8::Isolate::new(params);
    let mut locker = v8::Locker::new(&isolate);
    {
      let mut hs = v8::HandleScope::new(&mut locker);
      let scope = hs.enter();
      let mut context = v8::Context::new(scope);
      context.enter();

      let result =
        eval(scope, context, "if(F() != 'wrong answer') throw 'boom1'");
      assert!(result.is_none());

      let result =
        eval(scope, context, "if(F() != 'Hello callback!') throw 'boom2'");
      assert!(result.is_some());

      context.exit();
    }
    // TODO(ry) WARNING! startup_data needs to be kept alive as long the isolate
    // using it. See note in CreateParams::set_snapshot_blob
    drop(startup_data);
  }

  drop(g);
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
  let g = setup();
  let mut params = v8::Isolate::create_params();
  params.set_array_buffer_allocator(v8::new_default_allocator());
  let mut isolate = v8::Isolate::new(params);
  isolate.enter();
  let mut locker = v8::Locker::new(&isolate);
  {
    let mut hs = v8::HandleScope::new(&mut locker);
    let s = hs.enter();
    let mut context = v8::Context::new(s);
    context.enter();
    let source = v8::String::new(s, "new Uint8Array([23,23,23,23])").unwrap();
    let mut script = v8::Script::compile(s, context, source, None).unwrap();
    source.to_rust_string_lossy(s);
    let result = script.run(s, context).unwrap();
    // TODO: safer casts.
    let result = unsafe { Local::<v8::ArrayBufferView>::cast(result) };
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
    context.exit();
  }
  drop(locker);
  isolate.exit();
  drop(g);
}

#[test]
fn dynamic_import() {
  let g = setup();
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

  isolate.enter();
  let mut locker = v8::Locker::new(&isolate);
  {
    let mut hs = v8::HandleScope::new(&mut locker);
    let s = hs.enter();
    let mut context = v8::Context::new(s);
    context.enter();

    let result = eval(
      s,
      context,
      "(async function () {\n\
       let x = await import('bar.js');\n\
       })();",
    );
    assert!(result.is_some());
    assert_eq!(CALL_COUNT.load(Ordering::SeqCst), 1);

    context.exit();
  }
  drop(locker);
  isolate.exit();
  drop(g);
}

#[test]
fn shared_array_buffer() {
  let g = setup();
  let mut params = v8::Isolate::create_params();
  params.set_array_buffer_allocator(v8::new_default_allocator());
  let mut isolate = v8::Isolate::new(params);
  isolate.enter();
  let mut locker = v8::Locker::new(&isolate);
  {
    let mut hs = v8::HandleScope::new(&mut locker);
    let s = hs.enter();
    let mut context = v8::Context::new(s);
    context.enter();
    let maybe_sab = v8::SharedArrayBuffer::new(s, 16);
    assert!(maybe_sab.is_some());
    let sab = maybe_sab.unwrap();
    let mut backing_store = sab.get_backing_store();
    let shared_buf = backing_store.data_bytes();
    shared_buf[5] = 12;
    shared_buf[12] = 52;
    let global = context.global(s);
    assert_eq!(
      global.create_data_property(
        context,
        v8_str(s, "shared").into(),
        sab.into(),
      ),
      v8::MaybeBool::JustTrue
    );
    let source = v8::String::new(
      s,
      "sharedBytes = new Uint8Array(shared); sharedBytes[2] = 16; sharedBytes[14] = 62; sharedBytes[5] + sharedBytes[12]",
    )
        .unwrap();
    let mut script = v8::Script::compile(s, context, source, None).unwrap();
    source.to_rust_string_lossy(s);
    let result = script.run(s, context).unwrap();
    // TODO: safer casts.
    let result = unsafe { Local::<v8::Integer>::cast(result) };
    assert_eq!(result.value(), 64);
    assert_eq!(shared_buf[2], 16);
    assert_eq!(shared_buf[14], 62);
    context.exit();
  }
  drop(locker);
  isolate.exit();
  drop(g);
}

#[test]
#[allow(clippy::cognitive_complexity)]
fn value_checker() {
  let g = setup();
  let mut params = v8::Isolate::create_params();
  params.set_array_buffer_allocator(v8::new_default_allocator());
  let mut isolate = v8::Isolate::new(params);
  isolate.enter();
  let mut locker = v8::Locker::new(&isolate);
  {
    let mut hs = v8::HandleScope::new(&mut locker);
    let scope = hs.enter();
    let mut context = v8::Context::new(scope);
    context.enter();

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

    let value = eval(scope, context, "123").unwrap();
    assert!(value.is_number());

    let value = eval(scope, context, "123").unwrap();
    assert!(value.is_number());
    assert!(value.is_int32());
    assert!(value.is_uint32());

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

    context.exit();
  }
  drop(locker);
  isolate.exit();
  drop(g);
}
