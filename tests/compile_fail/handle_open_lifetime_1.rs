// Copyright 2019-2020 the Deno authors. All rights reserved. MIT license.

pub fn main() {
  let isolate = &mut v8::Isolate::new(mock());
  let context: v8::Local<v8::Context> = mock();
  let scope1 = &mut v8::HandleScope::with_context(isolate, context);

  let global = {
    let local1 = v8::Object::new(scope1);
    v8::Global::new(scope1, local1)
  };

  let local = {
    let scope2 = &mut v8::HandleScope::new(scope1);
    v8::Handle::open(&global, scope2)
  };

  let _ = v8::undefined(scope1);
  assert_eq!(global, local);
}

fn mock<T>() -> T {
  unimplemented!()
}
