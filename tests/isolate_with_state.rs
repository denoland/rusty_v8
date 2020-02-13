//! This shows a pattern for adding state to an Isolate. The pattern is used
//! extensively in Deno.

use rusty_v8 as v8;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};
use std::sync::atomic::{AtomicUsize, Ordering};

struct IsolateWithState<S>(v8::OwnedIsolate, PhantomData<S>);

impl<S> IsolateWithState<S> {
  fn new(mut isolate: v8::OwnedIsolate, state: S) -> IsolateWithState<S> {
    assert!(isolate.get_data(0).is_null());
    let boxed = Box::new(state);
    let ptr = Box::into_raw(boxed);
    unsafe { isolate.set_data(0, ptr as *mut _) };
    IsolateWithState(isolate, PhantomData)
  }

  fn state(&mut self) -> &mut S {
    let ptr = self.0.get_data(0) as *mut S;
    unsafe { &mut *ptr }
  }

  fn from(isolate: &v8::Isolate) -> &mut S {
    let ptr = isolate.get_data(0) as *mut S;
    unsafe { &mut *ptr }
  }
}

impl<S> Deref for IsolateWithState<S> {
  type Target = v8::OwnedIsolate;

  fn deref(&self) -> &Self::Target {
    &self.0
  }
}

impl<S> DerefMut for IsolateWithState<S> {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.0
  }
}

struct State {
  pub a: bool,
  // pub context: v8::Global<v8::Context>,
}

#[test]
fn isolate_with_state() {
  v8::V8::initialize_platform(v8::new_default_platform());
  v8::V8::initialize();

  let state = State {
    a: false,
    // context: v8::Global::new(),
  };

  let mut params = v8::Isolate::create_params();
  params.set_array_buffer_allocator(v8::new_default_allocator());
  let isolate = v8::Isolate::new(params);

  let mut isolate_with_state = IsolateWithState::new(isolate, state);

  static CALL_COUNT: AtomicUsize = AtomicUsize::new(0);
  fn change_state(
    scope: v8::FunctionCallbackScope,
    _args: v8::FunctionCallbackArguments,
    _rv: v8::ReturnValue,
  ) {
    println!("change_state 0");
    let state = IsolateWithState::<State>::from(scope.isolate());
    println!("change_state 1");
    state.a = true;
    println!("change_state 2");
    CALL_COUNT.fetch_add(1, Ordering::SeqCst);
  }

  {
    let mut hs = v8::HandleScope::new(isolate_with_state.deref_mut());
    let scope = hs.enter();
    let context = v8::Context::new(scope);
    let mut cs = v8::ContextScope::new(scope, context);
    let scope = cs.enter();
    let global = context.global(scope);

    // let state = IsolateWithState::<State>::from(scope.isolate());
    // state.context.set(scope, context);

    let mut change_state_tmpl = v8::FunctionTemplate::new(scope, change_state);
    let change_state_val =
      change_state_tmpl.get_function(scope, context).unwrap();
    global.set(
      context,
      v8::String::new(scope, "change_state").unwrap().into(),
      change_state_val.into(),
    );

    let source = v8::String::new(scope, "change_state()").unwrap();
    let mut script = v8::Script::compile(scope, context, source, None).unwrap();
    script.run(scope, context);
  }
  let state = isolate_with_state.state();
  assert!(state.a);
  assert_eq!(CALL_COUNT.load(Ordering::SeqCst), 1);
}
