use crate::isolate::Isolate;
use crate::Integer;
use crate::Local;
use crate::Number;
use crate::Scope;

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

impl Number {
  pub fn new<'s>(scope: &mut Scope, value: f64) -> Local<'s, Number> {
    let local = unsafe { v8__Number__New(scope.isolate(), value) };
    unsafe { scope.to_local(local) }.unwrap()
  }

  pub fn value(&self) -> f64 {
    unsafe { v8__Number__Value(self) }
  }
}

impl Integer {
  pub fn new<'s, 't: 's>(
    scope: &'s mut Scope,
    value: i32,
  ) -> Local<'t, Integer> {
    let local = unsafe { v8__Integer__New(scope.isolate(), value) };
    unsafe { scope.to_local(local) }.unwrap()
  }

  pub fn new_from_unsigned<'s>(
    scope: &mut Scope,
    value: u32,
  ) -> Local<'s, Integer> {
    let local = unsafe { v8__Integer__NewFromUnsigned(scope.isolate(), value) };
    unsafe { scope.to_local(local) }.unwrap()
  }

  pub fn value(&self) -> i64 {
    unsafe { v8__Integer__Value(self) }
  }
}
