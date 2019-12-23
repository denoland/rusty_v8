use std::convert::TryInto;
use std::ops::Deref;

use crate::support::int;
use crate::support::Opaque;
use crate::Object;

extern "C" {
  // TODO(afinch7) add this in when ArrayBuffer exists.
  // fn v8__ArrayBufferView__Buffer(this: &mut ArrayBufferView) -> *mut ArrayBuffer;
  fn v8__ArrayBufferView__ByteLength(this: &mut ArrayBufferView) -> usize;
  fn v8__ArrayBufferView__ByteOffset(this: &mut ArrayBufferView) -> usize;
  fn v8__ArrayBufferView__CopyContents(
    this: &mut ArrayBufferView,
    dest: *mut u8,
    byte_length: int,
  ) -> usize;
}

#[repr(C)]
pub struct ArrayBufferView(Opaque);

impl ArrayBufferView {
  pub fn byte_length(&mut self) -> usize {
    unsafe { v8__ArrayBufferView__ByteLength(self) }
  }

  pub fn byte_offset(&mut self) -> usize {
    unsafe { v8__ArrayBufferView__ByteOffset(self) }
  }

  pub fn copy_contents(&mut self, dest: &mut [u8]) -> usize {
    unsafe {
      v8__ArrayBufferView__CopyContents(
        self,
        dest.as_mut_ptr(),
        dest.len().try_into().unwrap(),
      )
    }
  }
}

impl Deref for ArrayBufferView {
  type Target = Object;
  fn deref(&self) -> &Self::Target {
    unsafe { &*(self as *const _ as *const Object) }
  }
}
