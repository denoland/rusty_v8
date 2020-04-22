//! Example of wrapping a v8::Isolate to add functionality. This is a pattern we
//! hope to use in deno_core.

use rusty_v8 as v8;
use std::ops::Deref;

static START: std::sync::Once = std::sync::Once::new();

struct Layer1(v8::OwnedIsolate);

struct Layer1State {
  i: usize,
}

impl Layer1 {
  fn new() -> Layer1 {
    START.call_once(|| {
      v8::V8::initialize_platform(v8::new_default_platform().unwrap());
      v8::V8::initialize();
    });
    let mut isolate = v8::Isolate::new(Default::default());
    let state = Layer1State { i: 0 };
    isolate.set_data_2(state);
    Layer1(isolate)
  }

  // Returns false if there was an error.
  fn execute(&mut self, code: &str) -> bool {
    let mut hs = v8::HandleScope::new(&mut self.0);
    let scope = hs.enter();
    let context = v8::Context::new(scope);
    let mut cs = v8::ContextScope::new(scope, context);
    let scope = cs.enter();
    let source = v8::String::new(scope, code).unwrap();
    let mut script = v8::Script::compile(scope, context, source, None).unwrap();
    let r = script.run(scope, context);
    r.is_some()
  }
}

impl Deref for Layer1 {
  type Target = v8::Isolate;

  fn deref(&self) -> &v8::Isolate {
    &self.0
  }
}

#[test]
fn layer1_test() {
  let mut l = Layer1::new();
  assert!(l.execute("1 + 1"));
  assert!(!l.execute("throw 'foo'"));
}
