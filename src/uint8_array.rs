// Copyright 2019-2020 the Deno authors. All rights reserved. MIT license.
use crate::ArrayBuffer;
use crate::Local;
use crate::ToLocal;
use crate::Uint8Array;

extern "C" {
  fn v8__Uint8Array__New(
    buf_ptr: *const ArrayBuffer,
    byte_offset: usize,
    length: usize,
  ) -> *const Uint8Array;
}

impl Uint8Array {
  pub fn new<'sc>(
    scope: &mut impl ToLocal<'sc>,
    buf: Local<ArrayBuffer>,
    byte_offset: usize,
    length: usize,
  ) -> Option<Local<'sc, Uint8Array>> {
    unsafe { scope.to_local(v8__Uint8Array__New(&*buf, byte_offset, length)) }
  }
}
