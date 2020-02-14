//! This shows a pattern for adding state to an Isolate. The pattern is used
//! extensively in Deno.

use rusty_v8 as v8;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};
use std::rc::Rc;

struct State1 {
  pub count: usize,
  pub js_count: v8::Global<v8::Integer>,
}

struct Isolate1(v8::OwnedIsolate, PhantomData<State1>);

impl Isolate1 {
  fn new() -> Isolate1 {
    let mut params = v8::Isolate::create_params();
    params.set_array_buffer_allocator(v8::new_default_allocator());
    let mut isolate = v8::Isolate::new(params);

    let state = State1 {
      count: 0,
      js_count: v8::Global::new(),
    };

    assert!(isolate.get_data(0).is_null());
    let rc_state = Rc::new(state);
    let ptr = Rc::into_raw(rc_state);
    unsafe { isolate.set_data(0, ptr as *mut _) };
    Isolate1(isolate, PhantomData)
  }

  fn mod_state<'s, S, R, F>(scope: &mut S, f: F) -> R
  where
    S: v8::ToLocal<'s>,
    F: FnOnce(&mut S, &mut State1) -> R,
  {
    let ptr = scope.isolate().get_data(0) as *const State1;
    let mut rc_state = unsafe { Rc::from_raw(ptr) };
    let state = Rc::get_mut(&mut rc_state).unwrap();
    let r = f(scope, state);
    let ptr = Rc::into_raw(rc_state);
    unsafe { scope.isolate().set_data(0, ptr as *mut _) };
    r
  }

  pub fn setup(&mut self) {
    let mut hs = v8::HandleScope::new(&mut self.0);
    let scope = hs.enter();
    let mut context = v8::Context::new(scope);
    context.enter();

    let global = context.global(scope);

    Self::mod_state(scope, |scope, state| {
      let js_count = v8::Integer::new(scope, 0);
      state.js_count.set(scope, js_count);
    });

    let mut change_state_tmpl =
      v8::FunctionTemplate::new(scope, Self::change_state);
    let change_state_val =
      change_state_tmpl.get_function(scope, context).unwrap();
    global.set(
      context,
      v8::String::new(scope, "change_state").unwrap().into(),
      change_state_val.into(),
    );
  }

  pub fn exec(&mut self, src: &str) {
    let mut hs = v8::HandleScope::new(&mut self.0);
    let scope = hs.enter();

    let context = scope.get_current_context().unwrap();
    let source = v8::String::new(scope, src).unwrap();
    let mut script = v8::Script::compile(scope, context, source, None).unwrap();
    script.run(scope, context);
  }

  fn change_state(
    scope: v8::FunctionCallbackScope,
    _args: v8::FunctionCallbackArguments,
    _rv: v8::ReturnValue,
  ) {
    Self::mod_state(scope, |scope, state| {
      state.count += 1;
      let js_count = state.js_count.get(scope).unwrap();
      let js_count_value = js_count.value() as i32;
      let js_count_next = v8::Integer::new(scope, js_count_value + 1);
      state.js_count.set(scope, js_count_next);
    });
  }

  fn count(&mut self) -> usize {
    // TODO(ry) this is awful - but we're being forced into it.
    let mut hs = v8::HandleScope::new(self.deref_mut());
    let scope = hs.enter();
    Self::mod_state(scope, |_scope, state| state.count)
  }

  fn js_count(&mut self) -> i64 {
    let mut hs = v8::HandleScope::new(self.deref_mut());
    let scope = hs.enter();
    Self::mod_state(scope, |scope, state| {
      let js_count = state.js_count.get(scope).unwrap();
      js_count.value()
    })
  }
}

impl Deref for Isolate1 {
  type Target = v8::OwnedIsolate;

  fn deref(&self) -> &Self::Target {
    &self.0
  }
}

impl DerefMut for Isolate1 {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.0
  }
}

#[test]
fn isolate_with_state() {
  v8::V8::initialize_platform(v8::new_default_platform());
  v8::V8::initialize();

  let mut isolate_with_state = Isolate1::new();

  isolate_with_state.setup();
  assert_eq!(0, isolate_with_state.js_count());
  assert_eq!(0, isolate_with_state.count());
  isolate_with_state.exec("change_state()");
  assert_eq!(1, isolate_with_state.js_count());
  assert_eq!(1, isolate_with_state.count());
  isolate_with_state.exec("change_state()");
  assert_eq!(2, isolate_with_state.js_count());
  assert_eq!(2, isolate_with_state.count());
}
