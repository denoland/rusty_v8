use std::ops::Deref;

use crate::support::Opaque;
use crate::Isolate;
use crate::Local;
use crate::ToLocal;
use crate::Value;

extern "C" {
  fn v8__SharedArrayBuffer__New(
    isolate: *mut Isolate,
    data_ptr: *const u8,
    byte_length: usize,
  ) -> *mut SharedArrayBuffer;
}

pub struct SharedBuf {
  bytes: Vec<u8>,
}

impl SharedBuf {
  fn new(len: usize) -> Self {
    let mut bytes = Vec::new();
    bytes.resize(len, 0);
    Self { bytes }
  }

  fn as_backing_store(&self) -> (*const u8, usize) {
    (self.bytes.as_ptr(), self.bytes.len())
  }

  pub fn bytes(&self) -> &[u8] {
    &self.bytes[..]
  }

  pub fn bytes_mut(&mut self) -> &mut [u8] {
    &mut self.bytes[..]
  }
}

#[repr(C)]
pub struct SharedArrayBuffer(Opaque);

impl SharedArrayBuffer {
  pub fn new<'sc>(
    scope: &mut impl ToLocal<'sc>,
    length: usize,
  ) -> (Option<Local<'sc, SharedArrayBuffer>>, SharedBuf) {
    let shared_buf = SharedBuf::new(length);
    let (data_ptr, len) = shared_buf.as_backing_store();
    unsafe {
      (
        Local::from_raw(v8__SharedArrayBuffer__New(
          scope.isolate(),
          data_ptr,
          len,
        )),
        shared_buf,
      )
    }
  }
}

impl Deref for SharedArrayBuffer {
  type Target = Value;
  fn deref(&self) -> &Self::Target {
    unsafe { &*(self as *const _ as *const Value) }
  }
}
