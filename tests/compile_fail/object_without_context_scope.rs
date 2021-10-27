// Copyright 2019-2020 the Deno authors. All rights reserved. MIT license.

pub fn main() {
  let mut isolate = v8::Isolate::new(mock());
  let mut scope = v8::HandleScope::new(&mut isolate);
  let _object = v8::Object::new(&mut scope);
}

fn mock<T>() -> T {
  unimplemented!()
}
