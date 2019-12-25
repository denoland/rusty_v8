use std::ops::DerefMut;

use crate::support::Opaque;
use crate::ArrayBuffer;
use crate::Local;

extern "C" {
  fn v8__Uint8Array__New(
    buf: *mut ArrayBuffer,
    byte_offset: usize,
    length: usize,
  ) -> *mut Uint8Array;
}

/// An instance of Uint8Array constructor (ES6 draft 15.13.6).
#[repr(C)]
pub struct Uint8Array(Opaque);

impl Uint8Array {
  pub fn new<'sc>(
    mut buf: Local<ArrayBuffer>,
    byte_offset: usize,
    length: usize,
  ) -> Option<Local<'sc, Uint8Array>> {
    unsafe {
      Local::from_raw(v8__Uint8Array__New(buf.deref_mut(), byte_offset, length))
    }
  }
}
