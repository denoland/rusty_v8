// Copyright 2019-2026 the Deno authors. All rights reserved. MIT license.

// Opening a `Global<T>` into a plain `&T` requires an unsafe block because the
// caller must keep the isolate and any Locker alive and must not send the
// reference across threads. Use `Local::new(scope, &global)` instead when
// possible.
use std::pin::pin;

pub fn main() {
  let mut isolate = v8::Isolate::new(mock());
  let global = {
    let scope = pin!(v8::HandleScope::new(&mut isolate));
    let scope = scope.init();
    let local = v8::String::new(&scope, "x").unwrap();
    v8::Global::new(&scope, local)
  };
  let _: &v8::String = global.open(&mut isolate);
}

fn mock<T>() -> T {
  unimplemented!()
}
