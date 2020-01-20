// Copyright 2019-2020 the Deno authors. All rights reserved. MIT license.
use rusty_v8 as v8;

pub fn main() {
  let context: v8::Local<v8::Context> = mock();
  let mut cs = v8::CallbackScope::new(context);
  let _hs = v8::EscapableHandleScope::new(cs.enter());
}

fn mock<T>() -> T {
  unimplemented!()
}
