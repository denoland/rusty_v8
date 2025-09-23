// Copyright 2019-2020 the Deno authors. All rights reserved. MIT license.
use std::pin::pin;

pub fn main() {
  let mut isolate = v8::Isolate::new(mock());
  let scope1 = pin!(v8::HandleScope::new(&mut isolate));
  let mut scope1 = scope1.init();

  let _boxed_local = {
    let scope = pin!(v8::HandleScope::new(&mut scope1));
    let mut scope = scope.init();
    let scope3 = pin!(v8::HandleScope::new(&mut scope));
    let mut scope3 = scope3.init();
    Box::new(v8::Integer::new(&mut scope3, 123))
  };
}

fn mock<T>() -> T {
  unimplemented!()
}
