// Copyright 2019-2021 the Deno authors. All rights reserved. MIT license.

use std::ffi::c_void;

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

  pub fn value(&self) -> *mut c_void {
    unsafe { v8__External__Value(self) }
  }
}
