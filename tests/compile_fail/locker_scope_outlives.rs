// Copyright 2019-2020 the Deno authors. All rights reserved. MIT license.
// Test that HandleScope cannot outlive the Locker it was created from.
use std::pin::pin;

pub fn main() {
  let mut isolate = v8::Isolate::new_unentered(mock());
  let scope;
  {
    let mut locker = v8::Locker::new(&mut isolate);
    scope = pin!(v8::HandleScope::new(&mut *locker));
  }
  // Error: locker is dropped but scope still references it
  let _scope = scope.init();
}

fn mock<T>() -> T {
  unimplemented!()
}
