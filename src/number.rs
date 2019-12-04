use std::ops::Deref;

use crate::isolate::CxxIsolate;
use crate::isolate::LockedIsolate;
use crate::support::Opaque;
use crate::HandleScope;
use crate::Local;

extern "C" {
  fn v8__Number__New(isolate: &mut CxxIsolate, value: f64) -> *mut Number;
  fn v8__Number__Value(this: &Number) -> f64;
  fn v8__Integer__New(isolate: &mut CxxIsolate, value: i32) -> *mut Integer;
  fn v8__Integer__NewFromUnsigned(
    isolate: &mut CxxIsolate,
    value: u32,
  ) -> *mut Integer;
  fn v8__Integer__Value(this: &Integer) -> i64;
}

#[repr(C)]
pub struct Number(Opaque);

impl Number {
  pub fn new<'sc>(
    scope: &mut HandleScope<'sc>,
    value: f64,
  ) -> Local<'sc, Number> {
    unsafe {
      let local = v8__Number__New(scope.cxx_isolate(), value);
      Local::from_raw(local).unwrap()
    }
  }

  pub fn value(&self) -> f64 {
    unsafe { v8__Number__Value(self) }
  }
}

#[repr(C)]
pub struct Integer(Opaque);

impl Integer {
  pub fn new<'sc>(
    scope: &mut HandleScope<'sc>,
    value: i32,
  ) -> Local<'sc, Integer> {
    unsafe {
      let local = v8__Integer__New(scope.cxx_isolate(), value);
      Local::from_raw(local).unwrap()
    }
  }

  pub fn new_from_unsigned<'sc>(
    scope: &mut HandleScope<'sc>,
    value: u32,
  ) -> Local<'sc, Integer> {
    unsafe {
      let local = v8__Integer__NewFromUnsigned(scope.cxx_isolate(), value);
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
