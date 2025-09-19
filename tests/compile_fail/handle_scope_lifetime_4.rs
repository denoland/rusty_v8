// Copyright 2019-2020 the Deno authors. All rights reserved. MIT license.
use std::pin::pin;

pub fn main() {
  let mut isolate = v8::Isolate::new(mock());
  let scope1 = pin!(v8::HandleScope::new(&mut isolate));
  let mut scope1 = scope1.init();

  let context = v8::Context::new(&mut scope1, v8::ContextOptions::default());
  let mut context_scope = v8::ContextScope::new(&mut scope1, context);

  let mut _scope3 = {
    v8::scope!(scope, &mut context_scope);
    v8::EscapableHandleScope::new(scope)
  };
}

fn mock<T>() -> T {
  unimplemented!()
}
