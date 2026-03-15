// Copyright 2019-2020 the Deno authors. All rights reserved. MIT license.
// Test that Locker is not Send - it cannot be transferred to another thread.
// Only UnenteredIsolate is Send, not the Locker itself.

pub fn main() {
  let mut isolate = v8::Isolate::new_unentered(mock());
  let locker = v8::Locker::new(&mut isolate);

  // Error: Locker is not Send
  std::thread::spawn(move || {
    drop(locker);
  });
}

fn mock<T>() -> T {
  unimplemented!()
}
