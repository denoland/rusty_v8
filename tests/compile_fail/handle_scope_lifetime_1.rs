// Copyright 2019-2020 the Deno authors. All rights reserved. MIT license.
use rusty_v8 as v8;

pub fn main() {
  let mut isolate = v8::Isolate::new(mock());
  let mut hs1 = v8::HandleScope::new(&mut isolate);
  let hs1 = hs1.enter();

  let mut hs2 = v8::EscapableHandleScope::new(hs1);
  let hs2 = hs2.enter();

  let _hs3 = v8::EscapableHandleScope::new(hs1);
  let _local = v8::Integer::new(hs2, 123);
}

fn mock<T>() -> T {
  unimplemented!()
}
