// Copyright 2019-2020 the Deno authors. All rights reserved. MIT license.
use std::pin::pin;

pub fn main() {
  let mut isolate = v8::Isolate::new(mock());
  let scope1 = pin!(v8::HandleScope::new(&mut isolate));
  let mut scope1 = scope1.init();

  let _local = {
    let scope = pin!(v8::HandleScope::new(&mut scope1));
    let mut scope = scope.init();
    let scope3 = pin!(v8::HandleScope::new(&mut scope));
    let mut scope3 = scope3.init();
    let context = v8::Context::new(&mut scope3, v8::ContextOptions::default());
    let mut scope3 = v8::ContextScope::new(&mut scope3, context);
    let scope4 = pin!(v8::EscapableHandleScope::new(&mut *scope3));
    let mut scope4 = scope4.init();
    let value = v8::Integer::new(&mut scope4, 42);
    scope4.escape(value)
  };
}

fn mock<T>() -> T {
  unimplemented!()
}
