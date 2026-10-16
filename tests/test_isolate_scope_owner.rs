// Copyright 2019-2021 the Deno authors. All rights reserved. MIT license.

//! Tests for the rule that all scopes open on a thread at the same time belong
//! to the same `Isolate` (see "Isolates" in the `scope` module documentation).

use std::pin::pin;
use std::sync::Once;

fn setup() {
  static INIT: Once = Once::new();
  INIT.call_once(|| {
    v8::V8::initialize_platform(
      v8::new_default_platform(0, false).make_shared(),
    );
    v8::V8::initialize();
  });
}

#[test]
fn nested_scopes_of_one_isolate() {
  setup();
  let isolate = &mut v8::Isolate::new(Default::default());
  v8::scope!(let scope, isolate);
  let context = v8::Context::new(scope, Default::default());
  let scope = &mut v8::ContextScope::new(scope, context);
  let outer = v8::String::new(scope, "outer").unwrap();

  let escaped = {
    let inner = pin!(v8::EscapableHandleScope::new(scope));
    let inner = &mut inner.init();
    let s = v8::String::concat(inner, outer, outer).unwrap();
    inner.escape(s)
  };
  assert_eq!(escaped.to_rust_string_lossy(scope), "outerouter");

  {
    v8::scope!(let inner, scope);
    let s = v8::String::concat(inner, outer, outer).unwrap();
    assert_eq!(s.length(), 10);
  }

  // The function callback runs in a `CallbackScope` nested in `scope`.
  let function = v8::Function::new(
    scope,
    |scope: &mut v8::PinScope,
     args: v8::FunctionCallbackArguments,
     mut rv: v8::ReturnValue<v8::Value>| {
      let arg = args.get(0);
      assert!(arg.is_string());
      v8::scope!(let inner, scope);
      let s = v8::String::new(inner, "inner").unwrap();
      rv.set_int32(
        s.length() as i32 + arg.to_string(inner).unwrap().length() as i32,
      );
    },
  )
  .unwrap();
  let recv = v8::undefined(scope).into();
  let result = function.call(scope, recv, &[outer.into()]).unwrap();
  assert_eq!(result.int32_value(scope), Some(10));
}

#[test]
fn scopes_of_isolates_in_sequence() {
  setup();
  let a = &mut v8::Isolate::new(Default::default());
  {
    v8::scope!(let scope, a);
    let s = v8::String::new(scope, "from a").unwrap();
    assert_eq!(s.length(), 6);
  }
  let b = &mut v8::Isolate::new(Default::default());
  {
    v8::scope!(let scope, b);
    let s = v8::String::new(scope, "from b").unwrap();
    assert_eq!(s.length(), 6);
  }
}

#[test]
fn scopes_of_isolates_on_separate_threads() {
  setup();
  let isolate = &mut v8::Isolate::new(Default::default());
  v8::scope!(let scope, isolate);
  let s = v8::String::new(scope, "main").unwrap();
  std::thread::spawn(|| {
    let isolate = &mut v8::Isolate::new(Default::default());
    v8::scope!(let scope, isolate);
    let s = v8::String::new(scope, "thread").unwrap();
    assert_eq!(s.length(), 6);
  })
  .join()
  .unwrap();
  assert_eq!(s.length(), 4);
}

#[test]
#[should_panic(
  expected = "cannot open a scope on an Isolate while a scope of another Isolate is open on the same thread"
)]
fn scope_of_second_isolate_panics() {
  setup();
  let a = &mut v8::Isolate::new(Default::default());
  v8::scope!(let scope_a, a);
  let from_a = v8::String::new(scope_a, "from a").unwrap();

  let b = &mut v8::Isolate::new(Default::default());
  v8::scope!(let scope_b, b);
  let context_b = v8::Context::new(scope_b, Default::default());
  let scope_b = &mut v8::ContextScope::new(scope_b, context_b);
  let object_b = v8::Object::new(scope_b);
  let key = v8::String::new(scope_b, "key").unwrap();
  // Without the check this would store a pointer into a's heap in b's heap.
  object_b.set(scope_b, key.into(), from_a.into());
}

// `TracedReference::get()` does not check which isolate the reference belongs
// to, so it can produce a `Local` of one isolate in a scope of another without
// opening a second scope. Debug builds catch that when the `Local` is used.
#[cfg(debug_assertions)]
#[test]
#[should_panic(
  expected = "a handle of one Isolate was used while a scope of another Isolate is open"
)]
fn handle_of_other_isolate_panics_on_use_in_debug_builds() {
  setup();
  let a = &mut v8::Isolate::new(Default::default());
  let traced = {
    v8::scope!(let scope, a);
    let s = v8::String::new(scope, "from a").unwrap();
    v8::TracedReference::new(scope, s)
  };

  let b = &mut v8::Isolate::new(Default::default());
  v8::scope!(let scope, b);
  let from_a = traced.get(scope).unwrap();
  from_a.length();
}

#[cfg(debug_assertions)]
#[test]
fn handles_without_an_owner_pass_the_debug_check() {
  setup();
  let isolate = &mut v8::Isolate::new(Default::default());
  v8::scope!(let scope, isolate);
  // Read-only space values and Smis are shared or not on the heap at all.
  assert!(v8::undefined(scope).is_undefined());
  assert!(v8::null(scope).is_null());
  assert!(v8::Boolean::new(scope, true).is_true());
  assert!(v8::String::empty(scope).is_string());
  assert_eq!(v8::Integer::new(scope, 7).value(), 7);
  assert_eq!(v8::Number::new(scope, 7.5).value(), 7.5);
}
