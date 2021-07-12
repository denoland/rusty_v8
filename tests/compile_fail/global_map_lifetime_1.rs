// Copyright 2019-2020 the Deno authors. All rights reserved. MIT license.
use rusty_v8 as v8;

pub fn main() {
  let mut isolate = v8::Isolate::new(mock());
  let scope = &mut v8::HandleScope::new(&mut isolate);
  let local = v8::Integer::new(scope, 42);
  let global = v8::Global::new(scope, local);
  let _ = global.map(|that| that);
}

fn mock<T>() -> T {
  unimplemented!()
}
