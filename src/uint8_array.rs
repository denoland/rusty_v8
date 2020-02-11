use std::ops::DerefMut;

use crate::ArrayBuffer;
use crate::Local;
use crate::Uint8Array;

extern "C" {
  fn v8__Uint8Array__New(
    buf: *mut ArrayBuffer,
    byte_offset: usize,
    length: usize,
  ) -> *mut Uint8Array;
}

impl Uint8Array {
  pub fn new<'sc>(
    mut buf: Local<ArrayBuffer>,
    byte_offset: usize,
    length: usize,
  ) -> Option<Local<Uint8Array>> {
    unsafe {
      Local::from_raw(v8__Uint8Array__New(buf.deref_mut(), byte_offset, length))
    }
  }
}
