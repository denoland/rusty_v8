use std::ops::Deref;

use crate::isolate::Isolate;
use crate::scope::Entered;
use crate::support::Opaque;
use crate::value::Value;
use crate::Local;

extern "C" {
  fn v8__Number__New(isolate: *mut Isolate, value: f64) -> *mut Number;
  fn v8__Number__Value(this: &Number) -> f64;
  fn v8__Integer__New(isolate: *mut Isolate, value: i32) -> *mut Integer;
  fn v8__Integer__NewFromUnsigned(
    isolate: *mut Isolate,
    value: u32,
  ) -> *mut Integer;
  fn v8__Integer__Value(this: *const Integer) -> i64;
}

/// A JavaScript number value (ECMA-262, 4.3.20)
#[repr(C)]
pub struct Number(Opaque);

impl Number {
  pub fn new<'sc>(
    scope: &mut impl AsMut<Isolate>,
    value: f64,
  ) -> Local<'sc, Number> {
    unsafe {
      let local = v8__Number__New(scope.as_mut(), value);
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
  pub fn new<'sc>(
    scope: &mut impl AsMut<Entered<'sc, Isolate>>,
    value: i32,
  ) -> Local<'sc, Integer> {
    unsafe {
      let local = v8__Integer__New(&mut **(scope.as_mut()), value);
      Local::from_raw(local).unwrap()
    }
  }

  pub fn new_from_unsigned<'sc>(
    scope: &mut impl AsMut<Isolate>,
    value: u32,
  ) -> Local<'sc, Integer> {
    unsafe {
      let local = v8__Integer__NewFromUnsigned(scope.as_mut(), value);
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
