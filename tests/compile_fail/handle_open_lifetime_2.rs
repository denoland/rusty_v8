// Copyright 2019-2020 the Deno authors. All rights reserved. MIT license.

pub fn main() {
  let isolate = &mut v8::Isolate::new(mock());
  let context: v8::Local<v8::Context> = mock();
  let scope = &mut v8::HandleScope::with_context(isolate, context);

  let local1 = v8::Object::new(scope);
  let global1 = v8::Global::new(scope, local1);
  let local2 = v8::Handle::open(&global1, scope);

  drop(global1);
  let _global2 = v8::Global::new(scope, local2);
}

fn mock<T>() -> T {
  unimplemented!()
}
