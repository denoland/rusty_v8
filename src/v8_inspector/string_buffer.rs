use crate::cxx_util::CxxVTable;
use crate::cxx_util::Delete;
use crate::cxx_util::UniquePtr;

use super::StringView;

// class V8_EXPORT StringBuffer {
//  public:
//   virtual ~StringBuffer() = default;
//   virtual const StringView& string() = 0;
//   // This method copies contents.
//   static std::unique_ptr<StringBuffer> create(const StringView&);
// };

extern "C" {
  fn v8_inspector__StringBuffer__DELETE(this: &'static mut StringBuffer) -> ();
  fn v8_inspector__StringBuffer__string(this: &mut StringBuffer)
    -> &StringView;
  fn v8_inspector__StringBuffer__create(
    source: &StringView,
  ) -> UniquePtr<StringBuffer>;
}

#[repr(C)]
#[derive(Debug)]
pub struct StringBuffer {
  _cxx_vtable: CxxVTable,
}

impl StringBuffer {
  pub fn string(&mut self) -> &StringView {
    unsafe { v8_inspector__StringBuffer__string(self) }
  }

  pub fn create(source: &StringView) -> UniquePtr<StringBuffer> {
    unsafe { v8_inspector__StringBuffer__create(source) }
  }
}

impl Delete for StringBuffer {
  fn delete(&'static mut self) {
    unsafe { v8_inspector__StringBuffer__DELETE(self) }
  }
}
