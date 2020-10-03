// Copyright 2019-2020 the Deno authors. All rights reserved. MIT license.

use std::ffi::c_void;
use std::rc::Rc;

use crate::External;
use crate::HandleScope;
use crate::Isolate;
use crate::Local;

extern "C" {
  fn v8__External__New(
    isolate: *mut Isolate,
    value: *mut c_void,
  ) -> *const External;
  fn v8__External__Value(this: *const External) -> *mut c_void;
}

impl External {
  pub fn new<'s>(
    scope: &mut HandleScope<'s, ()>,
    value: *mut c_void,
  ) -> Local<'s, Self> {
    unsafe {
      scope.cast_local(|sd| v8__External__New(sd.get_isolate_ptr(), value))
    }
    .unwrap()
  }

  pub fn new_rc<'s, T>(
    scope: &mut HandleScope<'s, ()>,
    value: Rc<T>,
  ) -> Local<'s, Self> {
    let value_ptr = Rc::into_raw(value) as *mut c_void;
    unsafe {
      scope.cast_local(|sd| v8__External__New(sd.get_isolate_ptr(), value_ptr))
    }
    .unwrap()
  }

  pub fn value(&self) -> *mut c_void {
    unsafe { v8__External__Value(self) }
  }

  pub fn value_rc<T>(&self) -> Rc<T> {
    unsafe {
      let value_ptr = v8__External__Value(self) as *mut T;
      Rc::from_raw(value_ptr)
    }
  }
}
