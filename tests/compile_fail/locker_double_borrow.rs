// Copyright 2019-2020 the Deno authors. All rights reserved. MIT license.
// Test that you cannot create two Lockers for the same isolate simultaneously.
// The borrow checker should prevent this at compile time.

pub fn main() {
  let mut isolate = v8::Isolate::new_unentered(mock());
  let _locker1 = v8::Locker::new(&mut isolate);
  // Error: cannot borrow `isolate` as mutable more than once
  let _locker2 = v8::Locker::new(&mut isolate);
}

fn mock<T>() -> T {
  unimplemented!()
}
