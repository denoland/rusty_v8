use rusty_v8 as v8;

#[test]
fn isolate_with_state() {
  v8::V8::initialize_platform(v8::new_default_platform());
  v8::V8::initialize();

  struct IsolateWithStateInner<S> {
    isolate: Option<v8::OwnedIsolate>,
    state: S,
  }
  struct IsolateWithState<S>(Box<IsolateWithStateInner<S>>);

  impl<S> IsolateWithState<S> {
    fn from(isolate: &mut v8::Isolate) -> &mut S {
      unsafe { &mut *(isolate.get_data(0) as *mut S) }
    }

    fn new(state: S) -> IsolateWithState<S> {
      let mut params = v8::Isolate::create_params();
      params.set_array_buffer_allocator(v8::new_default_allocator());
      let mut isolate = v8::Isolate::new(params);

      let mut boxed = Box::new(IsolateWithStateInner {
        isolate: None,
        state,
      });

      let ptr = Box::into_raw(boxed);
      unsafe { isolate.set_data(0, ptr as *mut _) };
      boxed = unsafe { Box::from_raw(ptr) };
      boxed.isolate = Some(isolate);
      IsolateWithState(boxed)
    }
  }

  use std::ops::{Deref, DerefMut};
  impl<S> Deref for IsolateWithState<S> {
    type Target = v8::OwnedIsolate;

    fn deref(&self) -> &Self::Target {
      self.0.isolate.as_ref().unwrap()
    }
  }

  impl<S> DerefMut for IsolateWithState<S> {
    fn deref_mut(&mut self) -> &mut Self::Target {
      self.0.isolate.as_mut().unwrap()
    }
  }

  fn change_state(
    scope: v8::FunctionCallbackScope,
    _args: v8::FunctionCallbackArguments,
    _rv: v8::ReturnValue,
  ) {
    todo!()
  }

  struct State {
    a: bool,
    pub context: v8::Global<v8::Context>,
  }

  let setup_guard = setup();

  let state = State {
    a: false,
    context: v8::Global::new(),
  };

  let mut isolate_with_state = IsolateWithState::new(state);

  {
    let mut hs = v8::HandleScope::new(isolate_with_state.deref_mut());
    let scope = hs.enter();
    let context = v8::Context::new(scope);
    let mut cs = v8::ContextScope::new(scope, context);
    let scope = cs.enter();
    let global = context.global(scope);

    /*
    let state = IsolateWithState::from(scope.isolate());
    state.context.set(scope, context);
    */

    let mut change_state_tmpl = v8::FunctionTemplate::new(scope, change_state);
    let change_state_val =
      change_state_tmpl.get_function(scope, context).unwrap();
    global.set(
      context,
      v8::String::new(scope, "change_state").unwrap().into(),
      change_state_val.into(),
    );
  }
}

/*
#[test]
fn wrap_isolate() {
  // This function tests a common pattern we use in the rest of Deno, which is
  // adding functionality to an Isolate by wrapping it in structs with more
  // state.
  static CALL_COUNT: AtomicUsize = AtomicUsize::new(0);

  fn change_state(
    scope: v8::FunctionCallbackScope,
    _args: v8::FunctionCallbackArguments,
    _rv: v8::ReturnValue,
  ) {
    CALL_COUNT.fetch_add(1, Ordering::SeqCst);
    let isolate = scope.isolate();
    let _ptr = isolate.get_data(0) as ;
    // TODO how to change Isolate2.state ?
  }

  struct Isolate2 {
    inner: v8::OwnedIsolate,
    context: v8::Global<v8::Context>,
    state: bool,
  }

  impl Isolate2 {
    fn new() -> Self {
      let mut params = v8::Isolate::create_params();
      params.set_array_buffer_allocator(v8::new_default_allocator());
      let isolate = v8::Isolate::new(params);


      let boxed = Box::new(Isolate2 {
        inner: isolate,
        context: v8::Global::new(),
        state: false,
      });

      let ptr = Box::into_raw(boxed);
      inner.set_data(0, ptr);
    }

    fn execute(&mut self, code: &str) -> bool {
      let mut hs = v8::HandleScope::new(&mut self.inner);
      let scope = hs.enter();
      let context = self.context.get(scope).unwrap();
      let mut cs = v8::ContextScope::new(scope, context);
      let scope = cs.enter();
      let result = eval(scope, context, code);
      result.is_some()
    }

    fn setup(&mut self) {
      let mut hs = v8::HandleScope::new(&mut self.inner);
      let scope = hs.enter();
      let context = v8::Context::new(scope);
      let mut cs = v8::ContextScope::new(scope, context);
      let scope = cs.enter();
      let global = context.global(scope);
      self.context.set(scope, context);
      let mut change_state_tmpl =
        v8::FunctionTemplate::new(scope, change_state);
      let change_state_val =
        change_state_tmpl.get_function(scope, context).unwrap();
      global.set(
        context,
        v8::String::new(scope, "change_state").unwrap().into(),
        change_state_val.into(),
      );
    }
  }

  let _setup_guard = setup();
  let mut isolate2 = Isolate2::new();
  isolate2.setup();
  assert!(isolate2.execute("1+2"));
  assert_eq!(CALL_COUNT.load(Ordering::SeqCst), 0);
  assert!(isolate2.execute("change_state()"));
  assert_eq!(CALL_COUNT.load(Ordering::SeqCst), 1);
}
*/
