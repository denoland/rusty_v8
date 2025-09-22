// Copyright 2019-2020 the Deno authors. All rights reserved. MIT license.
use std::pin::pin;

pub fn main() {
  let mut isolate = v8::Isolate::new(mock());
  let mut scope_pinned = pin!(v8::HandleScope::new(&mut isolate));
  let mut scope = scope_pinned.as_mut().init();

  let local = v8::Integer::new(&mut scope, 123);
  drop(scope_pinned);

  local.is_int32();
}

fn mock<T>() -> T {
  unimplemented!()
}
