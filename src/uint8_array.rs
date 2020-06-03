// Copyright 2019-2020 the Deno authors. All rights reserved. MIT license.
use crate::ArrayBuffer;
use crate::HandleScope;
use crate::Local;
use crate::Uint8Array;

extern "C" {
  fn v8__Uint8Array__New(
    buf_ptr: *const ArrayBuffer,
    byte_offset: usize,
    length: usize,
  ) -> *const Uint8Array;
}

impl Uint8Array {
  pub fn new<'s>(
    scope: &mut HandleScope<'s>,
    buf: Local<ArrayBuffer>,
    byte_offset: usize,
    length: usize,
  ) -> Option<Local<'s, Uint8Array>> {
    unsafe {
      scope.cast_local(|_| v8__Uint8Array__New(&*buf, byte_offset, length))
    }
  }
}
