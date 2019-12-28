use std::ops::Deref;

use crate::support::Opaque;
use crate::support::SharedRef;
use crate::BackingStore;
use crate::Isolate;
use crate::Local;
use crate::ToLocal;
use crate::Value;

extern "C" {
  fn v8__SharedArrayBuffer__New(
    isolate: *mut Isolate,
    byte_length: usize,
  ) -> *mut SharedArrayBuffer;
  fn v8__SharedArrayBuffer__ByteLength(
    self_: *const SharedArrayBuffer,
  ) -> usize;
  fn v8__SharedArrayBuffer__GetBackingStore(
    self_: *const SharedArrayBuffer,
  ) -> SharedRef<BackingStore>;
}

#[repr(C)]
pub struct SharedArrayBuffer(Opaque);

impl SharedArrayBuffer {
  pub fn new<'sc>(
    scope: &mut impl ToLocal<'sc>,
    byte_length: usize,
  ) -> Option<Local<'sc, SharedArrayBuffer>> {
    unsafe {
      Local::from_raw(v8__SharedArrayBuffer__New(scope.isolate(), byte_length))
    }
  }

  pub fn byte_length(&self) -> usize {
    unsafe { v8__SharedArrayBuffer__ByteLength(self) }
  }

  pub fn get_backing_store(&self) -> SharedRef<BackingStore> {
    unsafe { v8__SharedArrayBuffer__GetBackingStore(self) }
  }
}

impl Deref for SharedArrayBuffer {
  type Target = Value;
  fn deref(&self) -> &Self::Target {
    unsafe { &*(self as *const _ as *const Value) }
  }
}
