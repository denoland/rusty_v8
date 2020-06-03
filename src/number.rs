use crate::isolate::Isolate;
use crate::HandleScope;
use crate::Integer;
use crate::Local;
use crate::Number;

extern "C" {
  fn v8__Number__New(isolate: *mut Isolate, value: f64) -> *const Number;
  fn v8__Number__Value(this: *const Number) -> f64;
  fn v8__Integer__New(isolate: *mut Isolate, value: i32) -> *const Integer;
  fn v8__Integer__NewFromUnsigned(
    isolate: *mut Isolate,
    value: u32,
  ) -> *const Integer;
  fn v8__Integer__Value(this: *const Integer) -> i64;
}

impl Number {
  pub fn new<'s>(
    scope: &mut HandleScope<'s, ()>,
    value: f64,
  ) -> Local<'s, Number> {
    unsafe {
      scope.cast_local(|sd| v8__Number__New(sd.get_isolate_ptr(), value))
    }
    .unwrap()
  }

  pub fn value(&self) -> f64 {
    unsafe { v8__Number__Value(self) }
  }
}

impl Integer {
  pub fn new<'s>(
    scope: &mut HandleScope<'s, ()>,
    value: i32,
  ) -> Local<'s, Integer> {
    unsafe {
      scope.cast_local(|sd| v8__Integer__New(sd.get_isolate_ptr(), value))
    }
    .unwrap()
  }

  pub fn new_from_unsigned<'s>(
    scope: &mut HandleScope<'s, ()>,
    value: u32,
  ) -> Local<'s, Integer> {
    unsafe {
      scope.cast_local(|sd| {
        v8__Integer__NewFromUnsigned(sd.get_isolate_ptr(), value)
      })
    }
    .unwrap()
  }

  pub fn value(&self) -> i64 {
    unsafe { v8__Integer__Value(self) }
  }
}
