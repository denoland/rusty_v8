// Copyright 2018-2019 the Deno authors. All rights reserved. MIT license.
#![cfg(test)]
use std::sync::Mutex;

lazy_static! {
  static ref INIT_LOCK: Mutex<u32> = Mutex::new(0);
}

pub struct TestGuard {}

impl Drop for TestGuard {
  fn drop(&mut self) {
    // TODO shutdown process cleanly.
    /*
    *g -= 1;
    if *g  == 0 {
      unsafe { crate::V8::dispose() };
      crate::V8::shutdown_platform();
    }
    drop(g);
    */
  }
}

pub fn setup() -> TestGuard {
  let mut g = INIT_LOCK.lock().unwrap();
  *g += 1;
  if *g == 1 {
    crate::V8::initialize_platform(crate::platform::new_default_platform());
    crate::V8::initialize();
  }
  drop(g);
  TestGuard {}
}
