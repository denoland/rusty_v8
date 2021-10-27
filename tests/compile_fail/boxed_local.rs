// Copyright 2019-2020 the Deno authors. All rights reserved. MIT license.

pub fn main() {
  let mut isolate = v8::Isolate::new(mock());
  let mut scope1 = v8::HandleScope::new(&mut isolate);

  let _boxed_local = {
    let mut scope2 = v8::HandleScope::new(&mut scope1);
    let mut scope3 = v8::HandleScope::new(&mut scope2);
    Box::new(v8::Integer::new(&mut scope3, 123))
  };
}

fn mock<T>() -> T {
  unimplemented!()
}
