// Copyright 2019-2020 the Deno authors. All rights reserved. MIT license.

pub fn main() {
  let mut isolate = v8::Isolate::new(mock());
  let mut scope1 = v8::HandleScope::new(&mut isolate);

  let mut _scope3 = {
    let mut scope2 = v8::HandleScope::new(&mut scope1);
    v8::EscapableHandleScope::new(&mut scope2)
  };
}

fn mock<T>() -> T {
  unimplemented!()
}
