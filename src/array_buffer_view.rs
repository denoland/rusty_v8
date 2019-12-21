use crate::Objcet;
use crate::Value;

extern "C" {
  // TODO(afinch7) add this in when ArrayBuffer exists.
  // fn v8__ArrayBufferView__Buffer(this: &mut ArrayBufferView) -> *mut ArrayBuffer;
  fn v8__ArrayBufferView__ByteLength(this: &mut ArrayBufferView) -> usize;
  fn v8__ArrayBufferView__ByteOffset(this: &mut ArrayBufferView) -> usize;
  fn v8__ArrayBufferView_CopyContents(this: &mut ArrayBufferView, *mut [u8], int byte_length) -> usize;
}

#[repr(C)]
pub struct ArrayBufferView(Opaque);

impl Context {
  fn byte_length(&mut self) -> usize {
    unsafe { v8__ArrayBufferView__Buffer(self) }
  }

  fn byte_offset(&mut self) -> usize {
    unsafe { v8__ArrayBufferView__ByteOffset(self) }
  }

  fn copy_contents(&mut self, dest: &mut [u8]) -> usize {
    let byte_length = dest.len();
    unsafe { v8__ArrayBufferView_CopyContents(self, dest.as_mut_ptr(), byte_length) }
  } 
}

impl Deref for ArrayBufferView {
  type Target = Value;
  fn deref(&self) -> &Self::Target {
    unsafe { &*(self as *const _ as *const Value) }
  }
}

impl Deref for ArrayBufferView {
  type Target = Object;
  fn deref(&self) -> &Self::Target {
    unsafe { &*(self as *const _ as *const Object) }
  }
}
