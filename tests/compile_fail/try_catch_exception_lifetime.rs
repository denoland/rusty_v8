// Copyright 2019-2020 the Deno authors. All rights reserved. MIT license.
use rusty_v8 as v8;

pub fn main() {
  let mut isolate = v8::Isolate::new(mock());
  let mut try_catch = v8::TryCatch::new(&mut isolate);
  let try_catch = try_catch.enter();

  let _exception = {
    let mut hs = v8::HandleScope::new(&mut isolate);
    let hs = hs.enter();
    try_catch.exception(hs).unwrap()
  };
}

fn mock<T>() -> T {
  unimplemented!()
}
