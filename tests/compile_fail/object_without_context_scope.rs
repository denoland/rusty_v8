// Copyright 2019-2020 the Deno authors. All rights reserved. MIT license.

pub fn main() {
  let mut isolate = v8::Isolate::new(mock());
  v8::scope!(scope, &mut isolate);
  let _object = v8::Object::new(&*scope);
}

fn mock<T>() -> T {
  unimplemented!()
}
