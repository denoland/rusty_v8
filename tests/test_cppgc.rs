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

const TAG: u16 = 1;

#[test]
fn cppgc_object_wrap() {
  let guard = initalize_test();

  static TRACE_COUNT: AtomicUsize = AtomicUsize::new(0);
  static DROP_COUNT: AtomicUsize = AtomicUsize::new(0);

  struct Wrap {
    value: v8::TracedReference<v8::Value>,
  }

  impl GarbageCollected for Wrap {
    fn trace(&self, visitor: &Visitor) {
      TRACE_COUNT.fetch_add(1, Ordering::SeqCst);
      visitor.trace(&self.value);
    }
  }

  impl Drop for Wrap {
    fn drop(&mut self) {
      DROP_COUNT.fetch_add(1, Ordering::SeqCst);
    }
  }

  fn op_wrap(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
  ) {
    fn empty(
      _scope: &mut v8::HandleScope,
      _args: v8::FunctionCallbackArguments,
      _rv: v8::ReturnValue,
    ) {
    }
    let templ = v8::FunctionTemplate::new(scope, empty);
    let func = templ.get_function(scope).unwrap();
    let obj = func.new_instance(scope, &[]).unwrap();
    assert!(obj.is_api_wrapper());

    let wrap = Wrap {
      value: v8::TracedReference::new(scope, args.get(0)),
    };
    let member = unsafe {
      v8::cppgc::make_garbage_collected(scope.get_cpp_heap().unwrap(), wrap)
    };

    unsafe {
      v8::Object::wrap::<TAG, Wrap>(scope, obj, &member);
    }

    rv.set(obj.into());
  }

  fn op_unwrap(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
  ) {
    let obj = args.get(0).try_into().unwrap();
    let member = unsafe { v8::Object::unwrap::<TAG, Wrap>(scope, obj) };
    rv.set(member.unwrap().value.get(scope).unwrap());
  }

  {
    // Create a managed heap.
    let heap = v8::cppgc::Heap::create(
      guard.platform.clone(),
      v8::cppgc::HeapCreateParams::default(),
    );
    let isolate =
      &mut v8::Isolate::new(v8::CreateParams::default().cpp_heap(heap));

    let handle_scope = &mut v8::HandleScope::new(isolate);
    let context = v8::Context::new(handle_scope);
    let scope = &mut v8::ContextScope::new(handle_scope, context);
    let global = context.global(scope);
    {
      let func = v8::Function::new(scope, op_wrap).unwrap();
      let name = v8::String::new(scope, "wrap").unwrap();
      global.set(scope, name.into(), func.into()).unwrap();
    }
    {
      let func = v8::Function::new(scope, op_unwrap).unwrap();
      let name = v8::String::new(scope, "unwrap").unwrap();
      global.set(scope, name.into(), func.into()).unwrap();
    }

    execute_script(
      scope,
      r#"
      {
        const x = {};
        const y = unwrap(wrap(x)); // collected
        if (x !== y) {
          throw new Error('mismatch');
        }
      }

      globalThis.wrapped = wrap(wrap({})); // not collected
    "#,
    );

    assert_eq!(DROP_COUNT.load(Ordering::SeqCst), 0);

    scope
      .request_garbage_collection_for_testing(v8::GarbageCollectionType::Full);

    assert!(TRACE_COUNT.load(Ordering::SeqCst) > 0);
    assert_eq!(DROP_COUNT.load(Ordering::SeqCst), 1);

    execute_script(
      scope,
      r#"
      globalThis.wrapped = undefined;
    "#,
    );

    scope
      .request_garbage_collection_for_testing(v8::GarbageCollectionType::Full);

    assert_eq!(DROP_COUNT.load(Ordering::SeqCst), 3);
  }
}

fn execute_script(
  context_scope: &mut v8::ContextScope<v8::HandleScope>,
  source: &str,
) {
  let scope = &mut v8::HandleScope::new(context_scope);
  let scope = &mut v8::TryCatch::new(scope);

  let source = v8::String::new(scope, source).unwrap();

  let script =
    v8::Script::compile(scope, source, None).expect("failed to compile script");

  if script.run(scope).is_none() {
    let exception_string = scope
      .stack_trace()
      .or_else(|| scope.exception())
      .map(|value| value.to_rust_string_lossy(scope))
      .unwrap_or_else(|| "no stack trace".into());

    panic!("{}", exception_string);
  }
}
