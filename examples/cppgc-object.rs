// Copyright 2019-2021 the Deno authors. All rights reserved. MIT license.
use std::cell::Cell;

struct Wrappable {
  id: String,
  trace_count: Cell<u16>,
}

unsafe impl v8::cppgc::GarbageCollected for Wrappable {
  fn trace(&self, _visitor: &v8::cppgc::Visitor) {
    println!("Wrappable::trace() {}", self.id);
    self.trace_count.set(self.trace_count.get() + 1);
  }

  fn get_name(&self) -> &'static std::ffi::CStr {
    c"Wrappable"
  }
}

impl Drop for Wrappable {
  fn drop(&mut self) {
    println!("Wrappable::drop() {}", self.id);
  }
}

const TAG: u16 = 1;

fn main() {
  let platform = v8::new_default_platform(0, false).make_shared();
  v8::V8::set_flags_from_string("--no_freeze_flags_after_init --expose-gc");
  v8::V8::initialize_platform(platform.clone());
  v8::V8::initialize();

  v8::cppgc::initialize_process(platform.clone());

  {
    let heap =
      v8::cppgc::Heap::create(platform, v8::cppgc::HeapCreateParams::default());
    let isolate =
      &mut v8::Isolate::new(v8::CreateParams::default().cpp_heap(heap));

    let handle_scope = &mut v8::HandleScope::new(isolate);
    let context = v8::Context::new(handle_scope, Default::default());
    let scope = &mut v8::ContextScope::new(handle_scope, context);
    let global = context.global(scope);
    {
      let func = v8::Function::new(
        scope,
        |scope: &mut v8::HandleScope,
         args: v8::FunctionCallbackArguments,
         mut rv: v8::ReturnValue| {
          let id = args.get(0).to_rust_string_lossy(scope);

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

          let member = unsafe {
            v8::cppgc::make_garbage_collected(
              scope.get_cpp_heap().unwrap(),
              Wrappable {
                trace_count: Cell::new(0),
                id,
              },
            )
          };

          unsafe {
            v8::Object::wrap::<TAG, Wrappable>(scope, obj, &member);
          }

          rv.set(obj.into());
        },
      )
      .unwrap();
      let name = v8::String::new(scope, "make_wrap").unwrap();
      global.set(scope, name.into(), func.into()).unwrap();
    }

    let source = v8::String::new(
      scope,
      r#"
      make_wrap('gc me pls'); // Inaccessible after scope.
      globalThis.wrap = make_wrap('dont gc me'); // Accessible after scope.
    "#,
    )
    .unwrap();
    execute_script(scope, source);

    scope
      .request_garbage_collection_for_testing(v8::GarbageCollectionType::Full);
  }

  // Gracefully shutdown the process.
  unsafe {
    v8::cppgc::shutdown_process();
    v8::V8::dispose();
  }
  v8::V8::dispose_platform();
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
      .map_or_else(
        || "no stack trace".into(),
        |value| value.to_rust_string_lossy(try_catch),
      );

    panic!("{exception_string}");
  }
}
