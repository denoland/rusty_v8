// Copyright 2019-2026 the Deno authors. All rights reserved. MIT license.

// `Global<T>` deliberately does not implement `Borrow<T>`. `fn borrow(&self)
// -> &T` has nowhere to take proof that the caller may touch the isolate, and
// nowhere to tie the returned reference's lifetime to that proof — and since
// `Global` is `Send + Sync`, a `&Global<T>` can be on any thread. Use
// `Local::new(scope, &global)` instead.
use std::pin::pin;

pub fn main() {
  let mut isolate = v8::Isolate::new(mock());
  let scope = pin!(v8::HandleScope::new(&mut isolate));
  let scope = scope.init();
  let local = v8::String::new(&scope, "x").unwrap();
  let global = v8::Global::new(&scope, local);
  let _: &v8::String = std::borrow::Borrow::borrow(&global);
}

fn mock<T>() -> T {
  unimplemented!()
}
