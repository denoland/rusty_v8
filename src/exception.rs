#![allow(non_snake_case)]
use crate::Local;
use crate::String;
use crate::Value;

extern "C" {
  fn v8__Exception__RangeError(message: *mut String) -> *mut Value;
  fn v8__Exception__ReferenceError(message: *mut String) -> *mut Value;
  fn v8__Exception__SyntaxError(message: *mut String) -> *mut Value;
  fn v8__Exception__TypeError(message: *mut String) -> *mut Value;
  fn v8__Exception__Error(message: *mut String) -> *mut Value;
}

pub mod Exception {
  use super::*;

  pub fn RangeError<'sc>(mut message: Local<'_, String>) -> Local<'_, Value> {
    unsafe { Local::from_raw(v8__Exception__RangeError(&mut *message)) }
      .unwrap()
  }

  pub fn ReferenceError<'sc>(
    mut message: Local<'_, String>,
  ) -> Local<'_, Value> {
    unsafe { Local::from_raw(v8__Exception__ReferenceError(&mut *message)) }
      .unwrap()
  }

  pub fn SyntaxError<'sc>(mut message: Local<'_, String>) -> Local<'_, Value> {
    unsafe { Local::from_raw(v8__Exception__SyntaxError(&mut *message)) }
      .unwrap()
  }

  pub fn TypeError<'sc>(mut message: Local<'_, String>) -> Local<'_, Value> {
    unsafe { Local::from_raw(v8__Exception__TypeError(&mut *message)) }.unwrap()
  }

  pub fn Error<'sc>(mut message: Local<'_, String>) -> Local<'_, Value> {
    unsafe { Local::from_raw(v8__Exception__Error(&mut *message)) }.unwrap()
  }
}
