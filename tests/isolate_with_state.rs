//! This shows a pattern for adding state to an Isolate. The pattern is used
//! extensively in Deno.

use core::ops::Deref;
use core::ops::DerefMut;
use rusty_v8 as v8;

struct State1 {
  pub magic_number: usize,
  pub count: usize,
  pub js_count: v8::Global<v8::Integer>,
}

struct Isolate1(v8::OwnedIsolate);

impl Isolate1 {
  fn new() -> Isolate1 {
    let mut params = v8::Isolate::create_params();
    params.set_array_buffer_allocator(v8::new_default_allocator());
    let mut isolate = v8::Isolate::new(params);

    let state = State1 {
      magic_number: 0xCAFEBABE,
      count: 0,
      js_count: v8::Global::new(),
    };

    isolate.state_add(state);

    Isolate1(isolate)
  }

  pub fn setup(&mut self) {
    let mut hs = v8::HandleScope::new(&mut self.0);
    let scope = hs.enter();
    let mut context = v8::Context::new(scope);
    context.enter();

    let global = context.global(scope);

    {
      let js_count = v8::Integer::new(scope, 0);
      let state = scope.isolate().state_get::<State1>();
      assert_eq!(state.borrow().magic_number, 0xCAFEBABE);
      state.borrow_mut().js_count.set(scope, js_count);
    }

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
    let state = scope.isolate().state_get::<State1>();
    let mut state = state.borrow_mut();
    assert_eq!(state.magic_number, 0xCAFEBABE);
    state.count += 1;

    let js_count = state.js_count.get(scope).unwrap();
    let js_count_value = js_count.value() as i32;
    let js_count_next = v8::Integer::new(scope, js_count_value + 1);
    state.js_count.set(scope, js_count_next);
  }

  fn count(&self) -> usize {
    let state = self.0.state_get::<State1>();
    let state = state.borrow();
    assert_eq!(state.magic_number, 0xCAFEBABE);
    state.count
  }

  fn js_count(&mut self) -> i64 {
    let mut hs = v8::HandleScope::new(&mut self.0);
    let scope = hs.enter();

    let state = scope.isolate().state_get::<State1>();
    let state = state.borrow_mut();
    let js_count = state.js_count.get(scope).unwrap();

    js_count.value()
  }
}

impl Deref for Isolate1 {
  type Target = v8::Isolate;
  fn deref(&self) -> &v8::Isolate {
    &self.0
  }
}

impl DerefMut for Isolate1 {
  fn deref_mut(&mut self) -> &mut v8::Isolate {
    &mut self.0
  }
}

#[test]
fn isolate_with_state1() {
  init_v8();

  let mut isolate1 = Isolate1::new();

  isolate1.setup();
  assert_eq!(0, isolate1.count());
  assert_eq!(0, isolate1.js_count());
  isolate1.exec("change_state()");
  assert_eq!(1, isolate1.count());
  assert_eq!(1, isolate1.js_count());
  isolate1.exec("change_state()");
  assert_eq!(2, isolate1.count());
  assert_eq!(2, isolate1.js_count());
}

struct State2 {
  pub count2: usize,
  pub js_count2: v8::Global<v8::Integer>,
}

struct Isolate2(Isolate1);

impl Isolate2 {
  fn new() -> Isolate2 {
    let mut isolate1 = Isolate1::new();
    let state2 = State2 {
      count2: 0,
      js_count2: v8::Global::new(),
    };
    isolate1.state_add(state2);
    Isolate2(isolate1)
  }
}

#[test]
fn isolate_with_state2() {
  init_v8();

  let _isolate2 = Isolate2::new();

  /*
  isolate2.setup();
  assert_eq!(0, isolate2.count2());
  assert_eq!(0, isolate2.js_count2());
  isolate2.exec("change_state2()");
  assert_eq!(1, isolate2.count2());
  assert_eq!(1, isolate2.js_count2());
  isolate2.exec("change_state2()");
  assert_eq!(2, isolate2.count2());
  assert_eq!(2, isolate2.js_count2());
  */
}

fn init_v8() {
  use std::sync::Once;
  static INIT_V8: Once = Once::new();
  INIT_V8.call_once(|| {
    v8::V8::initialize_platform(v8::new_default_platform());
    v8::V8::initialize();
  });
}
