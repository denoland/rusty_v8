use std::ops::Deref;

use crate::support::Opaque;
use crate::Isolate;
use crate::Local;
use crate::Value;

extern "C" {
  fn v8__Number__New(isolate: &Isolate, value: f64) -> *mut Number;
  fn v8__Number__Value(this: &Number) -> f64;
  fn v8__Integer__New(isolate: &Isolate, value: i32) -> *mut Integer;
  fn v8__Integer__NewFromUnsigned(
    isolate: &Isolate,
    value: u32,
  ) -> *mut Integer;
  fn v8__Integer__Value(this: &Integer) -> i64;
}

/// A JavaScript number value (ECMA-262, 4.3.20)
#[repr(C)]
pub struct Number(Opaque);

impl Number {
  pub fn new<'sc>(isolate: &Isolate, value: f64) -> Local<Number> {
    unsafe {
      let local = v8__Number__New(isolate, value);
      Local::from_raw(local).unwrap()
    }
  }

  pub fn value(&self) -> f64 {
    unsafe { v8__Number__Value(self) }
  }
}

impl Deref for Number {
  type Target = Value;
  fn deref(&self) -> &Self::Target {
    unsafe { &*(self as *const _ as *const Value) }
  }
}

/// A JavaScript value representing a signed integer.
#[repr(C)]
pub struct Integer(Opaque);

impl Integer {
  pub fn new<'sc>(isolate: &Isolate, value: i32) -> Local<Integer> {
    unsafe {
      let local = v8__Integer__New(isolate, value);
      Local::from_raw(local).unwrap()
    }
  }

  pub fn new_from_unsigned<'sc>(
    isolate: &Isolate,
    value: u32,
  ) -> Local<Integer> {
    unsafe {
      let local = v8__Integer__NewFromUnsigned(isolate, value);
      Local::from_raw(local).unwrap()
    }
  }

  pub fn value(&self) -> i64 {
    unsafe { v8__Integer__Value(self) }
  }
}

impl Deref for Integer {
  type Target = Number;
  fn deref(&self) -> &Self::Target {
    unsafe { &*(self as *const _ as *const Number) }
  }
}
