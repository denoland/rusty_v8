// Copyright 2019-2020 the Deno authors. All rights reserved. MIT license.
use rusty_v8 as v8;

pub fn main() {
  let mut locker = v8::Locker::new(mock());
  let mut hs0 = v8::HandleScope::new(locker.enter());
  let hs0 = hs0.enter();

  let _fail = {
    let mut hs1 = v8::HandleScope::new(hs0);
    let hs1 = hs1.enter();

    let mut hs2 = v8::EscapableHandleScope::new(hs1);
    let hs2 = hs2.enter();

    let value: v8::Local<v8::Value> = v8::Integer::new(hs2, 42).into();
    hs2.escape(value)
  };
}

fn mock<T>() -> T {
  unimplemented!()
}
