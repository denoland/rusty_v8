use std::convert::TryInto;
use std::ops::Deref;

use crate::support::int;
use crate::support::Opaque;
use crate::ArrayBuffer;
use crate::Local;
use crate::Object;

extern "C" {
  fn v8__ArrayBufferView__Buffer(
    this: *const ArrayBufferView,
  ) -> *mut ArrayBuffer;
  fn v8__ArrayBufferView__ByteLength(this: *const ArrayBufferView) -> usize;
  fn v8__ArrayBufferView__ByteOffset(this: *const ArrayBufferView) -> usize;
  fn v8__ArrayBufferView__CopyContents(
    this: *const ArrayBufferView,
    dest: *mut u8,
    byte_length: int,
  ) -> usize;
}

/// A base class for an instance of one of "views" over ArrayBuffer,
/// including TypedArrays and DataView (ES6 draft 15.13).
#[repr(C)]
pub struct ArrayBufferView(Opaque);

impl ArrayBufferView {
  /// Returns underlying ArrayBuffer.
  pub fn buffer<'sc>(&self) -> Option<Local<'sc, ArrayBuffer>> {
    unsafe { Local::from_raw_(v8__ArrayBufferView__Buffer(self)) }
  }

  /// Size of a view in bytes.
  pub fn byte_length(&self) -> usize {
    unsafe { v8__ArrayBufferView__ByteLength(self) }
  }

  /// Byte offset in |Buffer|.
  pub fn byte_offset(&self) -> usize {
    unsafe { v8__ArrayBufferView__ByteOffset(self) }
  }

  /// Copy the contents of the ArrayBufferView's buffer to an embedder defined
  /// memory without additional overhead that calling ArrayBufferView::Buffer
  /// might incur.
  /// Returns the number of bytes actually written.
  pub fn copy_contents(&self, dest: &mut [u8]) -> usize {
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
