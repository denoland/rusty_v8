// Copyright 2019-2020 the Deno authors. All rights reserved. MIT license.
pub fn main() {
  let mut isolate = v8::Isolate::new(mock());
  let mut scope1 = v8::HandleScope::new(&mut isolate);

  let _local = {
    let mut _scope2 = v8::EscapableHandleScope::new(&mut scope1);
    v8::Integer::new(&mut scope1, 123)
  };
}

fn mock<T>() -> T {
  unimplemented!()
}
