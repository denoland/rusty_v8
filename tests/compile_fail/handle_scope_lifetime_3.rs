// Copyright 2019-2020 the Deno authors. All rights reserved. MIT license.
use std::pin::pin;

pub fn main() {
  let mut isolate = v8::Isolate::new(mock());
  let scope1 = pin!(v8::HandleScope::new(&mut isolate));
  let mut scope1 = scope1.init();
  let context = v8::Context::new(&mut scope1, v8::ContextOptions::default());
  let mut context_scope = v8::ContextScope::new(&mut scope1, context);

  let _local = {
    let mut _scope2 =
      v8::EscapableHandleScope::new((&mut *context_scope).as_mut());
    v8::Integer::new(&mut scope1, 123)
  };
}

fn mock<T>() -> T {
  unimplemented!()
}
