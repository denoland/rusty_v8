// Copyright 2019-2021 the Deno authors. All rights reserved. MIT license.
use std::sync::atomic::{AtomicUsize, Ordering};
use v8::cppgc::{GarbageCollected, Visitor};

struct CppGCGuard {
  pub platform: v8::SharedRef<v8::Platform>,
}

fn initalize_test() -> CppGCGuard {
  v8::V8::set_flags_from_string("--no_freeze_flags_after_init --expose-gc");
  let platform = v8::new_unprotected_default_platform(0, false).make_shared();
  v8::V8::initialize_platform(platform.clone());
  v8::V8::initialize();
  v8::cppgc::initalize_process(platform.clone());

  CppGCGuard { platform }
}

impl Drop for CppGCGuard {
  fn drop(&mut self) {
    unsafe {
      v8::cppgc::shutdown_process();
      v8::V8::dispose();
    }
    v8::V8::dispose_platform();
  }
}

const DEFAULT_CPP_GC_EMBEDDER_ID: u16 = 0xde90;

#[test]
fn cppgc_object_wrap() {
  let guard = initalize_test();

  static TRACE_COUNT: AtomicUsize = AtomicUsize::new(0);
  static DROP_COUNT: AtomicUsize = AtomicUsize::new(0);

  struct Wrap;

  impl GarbageCollected for Wrap {
    fn trace(&self, _: &Visitor) {
      TRACE_COUNT.fetch_add(1, Ordering::SeqCst);
    }
  }

  impl Drop for Wrap {
    fn drop(&mut self) {
      DROP_COUNT.fetch_add(1, Ordering::SeqCst);
    }
  }

  fn op_make_wrap(
    scope: &mut v8::HandleScope,
    _: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
  ) {
    let templ = v8::ObjectTemplate::new(scope);
    templ.set_internal_field_count(2);

    let obj = templ.new_instance(scope).unwrap();

    let member =
      v8::cppgc::make_garbage_collected(scope.get_cpp_heap(), Box::new(Wrap));

    obj.set_aligned_pointer_in_internal_field(
      0,
      &DEFAULT_CPP_GC_EMBEDDER_ID as *const u16 as _,
    );
    obj.set_aligned_pointer_in_internal_field(1, member.handle as _);

    rv.set(obj.into());
  }

  {
    let isolate = &mut v8::Isolate::new(Default::default());
    // Create a managed heap.
    let heap = v8::cppgc::Heap::create(
      guard.platform.clone(),
      v8::cppgc::HeapCreateParams::new(v8::cppgc::WrapperDescriptor::new(
        0,
        1,
        DEFAULT_CPP_GC_EMBEDDER_ID,
      )),
    );

    isolate.attach_cpp_heap(&heap);

    let handle_scope = &mut v8::HandleScope::new(isolate);
    let context = v8::Context::new(handle_scope);
    let scope = &mut v8::ContextScope::new(handle_scope, context);
    let global = context.global(scope);
    {
      let func = v8::Function::new(scope, op_make_wrap).unwrap();
      let name = v8::String::new(scope, "make_wrap").unwrap();
      global.set(scope, name.into(), func.into()).unwrap();
    }

    let source = v8::String::new(
      scope,
      r#"
      make_wrap(); // Inaccessible after scope.
      globalThis.wrap = make_wrap(); // Accessible after scope.
    "#,
    )
    .unwrap();
    execute_script(scope, source);

    assert_eq!(DROP_COUNT.load(Ordering::SeqCst), 0);

    scope
      .request_garbage_collection_for_testing(v8::GarbageCollectionType::Full);

    assert!(TRACE_COUNT.load(Ordering::SeqCst) > 0);
    assert_eq!(DROP_COUNT.load(Ordering::SeqCst), 1);
  }
}

fn execute_script(
  context_scope: &mut v8::ContextScope<v8::HandleScope>,
  script: v8::Local<v8::String>,
) {
  let scope = &mut v8::HandleScope::new(context_scope);
  let try_catch = &mut v8::TryCatch::new(scope);

  let script = v8::Script::compile(try_catch, script, None)
    .expect("failed to compile script");

  if script.run(try_catch).is_none() {
    let exception_string = try_catch
      .stack_trace()
      .or_else(|| try_catch.exception())
      .map(|value| value.to_rust_string_lossy(try_catch))
      .unwrap_or_else(|| "no stack trace".into());

    panic!("{}", exception_string);
  }
}
