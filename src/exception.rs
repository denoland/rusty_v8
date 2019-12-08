#![allow(non_snake_case)]
use crate::Local;
use crate::String;
use crate::Value;

extern "C" {
  fn v8__Exception__TypeError(message: *mut String) -> *mut Value;
}

pub mod Exception {
  use super::*;

  pub fn TypeError<'sc>(mut message: Local<'_, String>) -> Local<'_, Value> {
    unsafe { Local::from_raw(v8__Exception__TypeError(&mut *message)) }.unwrap()
  }
}
