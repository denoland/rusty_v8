use std::convert::TryInto;
use std::ffi::c_void;

use crate::support::int;
use crate::ArrayBuffer;
use crate::ArrayBufferView;
use crate::HandleScope;
use crate::Local;

extern "C" {
  fn v8__ArrayBufferView__Buffer(
    this: *const ArrayBufferView,
  ) -> *const ArrayBuffer;
  fn v8__ArrayBufferView__ByteLength(this: *const ArrayBufferView) -> usize;
  fn v8__ArrayBufferView__ByteOffset(this: *const ArrayBufferView) -> usize;
  fn v8__ArrayBufferView__CopyContents(
    this: *const ArrayBufferView,
    dest: *mut c_void,
    byte_length: int,
  ) -> usize;
}

impl ArrayBufferView {
  /// Returns underlying ArrayBuffer.
  pub fn buffer<'s>(
    &self,
    scope: &mut HandleScope<'s>,
  ) -> Option<Local<'s, ArrayBuffer>> {
    unsafe { scope.cast_local(|_| v8__ArrayBufferView__Buffer(self)) }
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
        dest.as_mut_ptr() as *mut c_void,
        dest.len().try_into().unwrap(),
      )
    }
  }
}
