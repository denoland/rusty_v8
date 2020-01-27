// Copyright 2019-2020 the Deno authors. All rights reserved. MIT license.
use rusty_v8 as v8;

pub fn main() {
  let mut locker = v8::Locker::new(mock::<v8::IsolateHandle>());
  let mut hs = v8::HandleScope::new(locker.enter());
  let scope = hs.enter();
  let context: v8::Local<v8::Context> = mock();

  let _leaked = {
    let mut try_catch = v8::TryCatch::new(scope);
    let tc = try_catch.enter();
    let exception = tc.exception().unwrap();
    let stack_trace = tc.stack_trace(scope, context).unwrap();
    let message = tc.message().unwrap();
    (exception, stack_trace, message)
  };
}

fn mock<T>() -> T {
  unimplemented!()
}
