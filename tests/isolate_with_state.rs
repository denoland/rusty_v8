//! This shows a pattern for adding state to an Isolate. The pattern is used
//! extensively in Deno.

use rusty_v8 as v8;
use std::cell::RefCell;

struct State1 {
  pub magic_number: usize,
  pub count: usize,
  pub js_count: v8::Global<v8::Integer>,
}

struct Isolate1(v8::OwnedIsolate, Box<RefCell<State1>>);

impl Isolate1 {
  fn new() -> Isolate1 {
    let mut params = v8::Isolate::create_params();
    params.set_array_buffer_allocator(v8::new_default_allocator());
    let isolate = v8::Isolate::new(params);

    let state = State1 {
      magic_number: 0xCAFEBABE,
      count: 0,
      js_count: v8::Global::new(),
    };

    assert!(isolate.get_data(0).is_null());
    let state_cell = Box::new(RefCell::new(state));
    let mut isolate1 = Isolate1(isolate, state_cell);
    let ptr = isolate1.1.as_ptr();
    unsafe { isolate1.0.set_data(0, ptr as *mut _) };
    isolate1
  }

  pub fn setup(&mut self) {
    let mut hs = v8::HandleScope::new(&mut self.0);
    let scope = hs.enter();
    let mut context = v8::Context::new(scope);
    context.enter();

    let global = context.global(scope);

    {
      let mut state = self.1.borrow_mut();
      let js_count = v8::Integer::new(scope, 0);
      state.js_count.set(scope, js_count);
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
    let ptr = scope.isolate().get_data(0) as *mut State1;
    let mut state = unsafe { &mut *ptr };
    assert_eq!(state.magic_number, 0xCAFEBABE);
    state.count += 1;

    let js_count = state.js_count.get(scope).unwrap();
    let js_count_value = js_count.value() as i32;
    let js_count_next = v8::Integer::new(scope, js_count_value + 1);
    state.js_count.set(scope, js_count_next);
  }

  fn count(&self) -> usize {
    self.1.borrow().count
  }

  fn js_count(&mut self) -> i64 {
    let mut hs = v8::HandleScope::new(&mut self.0);
    let scope = hs.enter();
    let state = self.1.borrow_mut();
    let js_count = state.js_count.get(scope).unwrap();
    js_count.value()
  }
}

#[test]
fn isolate_with_state() {
  v8::V8::initialize_platform(v8::new_default_platform());
  v8::V8::initialize();

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
