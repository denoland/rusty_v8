use std::mem::MaybeUninit;
use std::ops::Deref;

use crate::support::Opaque;
use crate::Isolate;
use crate::Local;
use crate::ToLocal;
use crate::Value;

extern "C" {
  fn v8__SharedArrayBuffer__New(
    isolate: *mut Isolate,
    byte_length: usize,
  ) -> *mut SharedArrayBuffer;
  fn v8__SharedArrayBuffer__ByteLength(this: *const SharedArrayBuffer)
    -> usize;
  fn v8__SharedArrayBuffer__GetContents(
    this: *const SharedArrayBuffer,
    out: &mut MaybeUninit<SharedArrayBuffer__Contents>,
  );
  fn v8__SharedArrayBuffer__Contents__ByteLength(
    this: *const SharedArrayBuffer__Contents,
  ) -> usize;
  fn v8__SharedArrayBuffer__Contents__Data(
    this: *mut SharedArrayBuffer__Contents,
  ) -> *mut u8;
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

  pub fn get_contents(&self) -> SharedArrayBuffer__Contents {
    let mut out = MaybeUninit::<SharedArrayBuffer__Contents>::uninit();
    unsafe {
      v8__SharedArrayBuffer__GetContents(self, &mut out);
      out.assume_init()
    }
  }
}

impl Deref for SharedArrayBuffer {
  type Target = Value;
  fn deref(&self) -> &Self::Target {
    unsafe { &*(self as *const _ as *const Value) }
  }
}

#[repr(C)]
pub struct SharedArrayBuffer__Contents([usize; 1]);

impl SharedArrayBuffer__Contents {
  pub fn byte_length(&self) -> usize {
    unsafe { v8__SharedArrayBuffer__Contents__ByteLength(self) }
  }

  pub fn data<'a>(&mut self) -> &'a mut [u8] {
    unsafe {
      std::slice::from_raw_parts_mut::<'a, u8>(
        v8__SharedArrayBuffer__Contents__Data(self),
        self.byte_length(),
      )
    }
  }
}
