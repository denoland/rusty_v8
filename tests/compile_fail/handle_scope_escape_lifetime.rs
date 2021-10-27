// Copyright 2019-2020 the Deno authors. All rights reserved. MIT license.

pub fn main() {
  let mut isolate = v8::Isolate::new(mock());
  let mut scope1 = v8::HandleScope::new(&mut isolate);

  let _local = {
    let mut scope2 = v8::HandleScope::new(&mut scope1);
    let mut scope3 = v8::HandleScope::new(&mut scope2);
    let mut scope4 = v8::EscapableHandleScope::new(&mut scope3);
    let value = v8::Integer::new(&mut scope4, 42);
    scope4.escape(value)
  };
}

fn mock<T>() -> T {
  unimplemented!()
}
