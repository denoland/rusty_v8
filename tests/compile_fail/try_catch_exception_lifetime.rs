// Copyright 2019-2020 the Deno authors. All rights reserved. MIT license.

pub fn main() {
  let mut isolate = v8::Isolate::new(mock());
  v8::make_handle_scope!(scope1, &mut isolate);
  let context = v8::Context::new(scope1, Default::default());
  let mut scope = v8::ContextScope::new(scope1, context);

  let _exception = {
    v8::make_handle_scope!(scope3, &mut *scope);
    v8::make_handle_scope!(scope4, scope3);
    let try_catch = std::pin::pin!(v8::TryCatch::new(&mut *scope4));
    let try_catch = try_catch.init();
    try_catch.exception().unwrap()
  };
}

fn mock<T>() -> T {
  unimplemented!()
}
