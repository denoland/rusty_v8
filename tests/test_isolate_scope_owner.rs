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

// The bindings behind the debug check, called directly so that they can be
// tested in release builds and with values of several isolates at once.
unsafe extern "C" {
  fn v8__Data__MaybeGetIsolate(
    this: *const v8::Data,
  ) -> v8::UnsafeRawIsolatePtr;
  fn v8__Data__IsCompatibleWithIsolate(
    this: *const v8::Data,
    isolate: v8::UnsafeRawIsolatePtr,
  ) -> bool;
  fn v8__Data__IsCompatibleWith(
    this: *const v8::Data,
    other: *const v8::Data,
  ) -> bool;
}

fn addr(isolate: v8::UnsafeRawIsolatePtr) -> usize {
  // SAFETY: `UnsafeRawIsolatePtr` is a transparent wrapper around a pointer.
  unsafe { std::mem::transmute(isolate) }
}

struct Values {
  isolate: v8::UnsafeRawIsolatePtr,
  // A value in the isolate's own heap.
  object: v8::Global<v8::Data>,
  // A value in read-only space, which all isolates of a group share.
  undefined: v8::Global<v8::Data>,
  smi: v8::Global<v8::Data>,
}

impl Values {
  fn new(isolate: &mut v8::OwnedIsolate) -> Self {
    let ptr = unsafe { isolate.as_raw_isolate_ptr() };
    v8::scope!(let scope, isolate);
    let object = v8::String::new(scope, "on the heap").unwrap();
    let undefined = v8::undefined(scope);
    let smi = v8::Integer::new(scope, 7);
    Self {
      isolate: ptr,
      object: v8::Global::new(scope, v8::Local::<v8::Data>::from(object)),
      undefined: v8::Global::new(scope, v8::Local::<v8::Data>::from(undefined)),
      smi: v8::Global::new(scope, v8::Local::<v8::Data>::from(smi)),
    }
  }
}

fn get(
  isolate: &mut v8::Isolate,
  global: &v8::Global<v8::Data>,
) -> *const v8::Data {
  unsafe { global.open(isolate) }
}

#[test]
fn compatibility_of_values_of_isolates_in_one_group() {
  setup();
  let a = &mut v8::Isolate::new(Default::default());
  let va = Values::new(a);
  let b = &mut v8::Isolate::new(Default::default());
  let vb = Values::new(b);

  let a_object = get(a, &va.object);
  let a_undefined = get(a, &va.undefined);
  let a_smi = get(a, &va.smi);
  let b_object = get(b, &vb.object);
  let b_undefined = get(b, &vb.undefined);

  unsafe {
    assert_eq!(addr(v8__Data__MaybeGetIsolate(a_object)), addr(va.isolate));
    assert!(v8__Data__MaybeGetIsolate(a_undefined).is_null());
    assert!(v8__Data__MaybeGetIsolate(a_smi).is_null());

    assert!(v8__Data__IsCompatibleWithIsolate(a_object, va.isolate));
    assert!(!v8__Data__IsCompatibleWithIsolate(a_object, vb.isolate));
    assert!(v8__Data__IsCompatibleWithIsolate(a_undefined, vb.isolate));
    assert!(v8__Data__IsCompatibleWithIsolate(a_smi, vb.isolate));

    assert!(v8__Data__IsCompatibleWith(a_object, a_object));
    assert!(!v8__Data__IsCompatibleWith(a_object, b_object));
    assert!(!v8__Data__IsCompatibleWith(b_object, a_object));
    assert!(v8__Data__IsCompatibleWith(a_object, b_undefined));
    assert!(v8__Data__IsCompatibleWith(b_undefined, a_object));
    assert!(v8__Data__IsCompatibleWith(a_undefined, b_undefined));
    assert!(v8__Data__IsCompatibleWith(a_smi, b_object));
  }
}

#[test]
fn compatibility_of_values_of_isolates_in_different_groups() {
  setup();
  if !v8::IsolateGroup::can_create_new_groups() {
    eprintln!("skipped: V8 was built without support for multiple groups");
    return;
  }
  let group1 = v8::IsolateGroup::create();
  let group2 = v8::IsolateGroup::create();
  let a = &mut v8::Isolate::new_with_group(&group1, Default::default());
  let va = Values::new(a);
  let a2 = &mut v8::Isolate::new_with_group(&group1, Default::default());
  let va2 = Values::new(a2);
  let b = &mut v8::Isolate::new_with_group(&group2, Default::default());
  let vb = Values::new(b);

  let a_object = get(a, &va.object);
  let a_undefined = get(a, &va.undefined);
  let a_smi = get(a, &va.smi);
  let a2_undefined = get(a2, &va2.undefined);
  let b_object = get(b, &vb.object);
  let b_undefined = get(b, &vb.undefined);

  unsafe {
    assert!(v8__Data__IsCompatibleWithIsolate(a_undefined, va2.isolate));
    assert!(!v8__Data__IsCompatibleWithIsolate(a_undefined, vb.isolate));
    assert!(!v8__Data__IsCompatibleWithIsolate(a_object, vb.isolate));
    assert!(v8__Data__IsCompatibleWithIsolate(a_smi, vb.isolate));

    assert!(v8__Data__IsCompatibleWith(a_undefined, a2_undefined));
    assert!(!v8__Data__IsCompatibleWith(a_undefined, b_undefined));
    assert!(!v8__Data__IsCompatibleWith(a_object, b_undefined));
    assert!(!v8__Data__IsCompatibleWith(b_object, a_undefined));
    assert!(v8__Data__IsCompatibleWith(a_smi, b_undefined));
  }
}
