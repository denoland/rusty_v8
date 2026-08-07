// Copyright 2019-2026 the Deno authors. All rights reserved. MIT license.

// A shared reference extracted from a Local must not be sent to another
// thread. Even methods that do not take a scope may call into V8 and therefore
// require access to the handle's isolate on the current thread.
use std::pin::pin;

pub fn main() {
  let mut isolate = v8::Isolate::new(mock());
  let scope = pin!(v8::HandleScope::new(&mut isolate));
  let scope = scope.init();
  let local = v8::String::new(&scope, "x").unwrap();
  let value: &v8::String = &local;

  std::thread::scope(|threads| {
    threads.spawn(move || assert!(value.is_string()));
  });
}

fn mock<T>() -> T {
  unimplemented!()
}
