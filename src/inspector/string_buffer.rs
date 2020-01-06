use super::string_view::StringView;
use crate::support::CxxVTable;
use crate::support::Delete;
use crate::support::UniquePtr;

// class StringBuffer {
//  public:
//   virtual ~StringBuffer() = default;
//   virtual const StringView& string() = 0;
//   // This method copies contents.
//   static std::unique_ptr<StringBuffer> create(const StringView&);
// };

// TODO: in C++, this class is intended to be user-extensible, just like
// like `Task`, `Client`, `Channel`. In Rust this would ideally also be the
// case, but currently to obtain a `UniquePtr<StringBuffer>` is by making a
// copy using `StringBuffer::create()`.

extern "C" {
  fn v8_inspector__StringBuffer__DELETE(this: &'static mut StringBuffer) -> ();
  fn v8_inspector__StringBuffer__string(this: &mut StringBuffer)
    -> &StringView;
  fn v8_inspector__StringBuffer__create(
    source: &StringView,
  ) -> UniquePtr<StringBuffer>;
}

#[repr(C)]
pub struct StringBuffer {
  _cxx_vtable: CxxVTable,
}

// TODO: make it possible to obtain a `UniquePtr<StringBuffer>` directly from
// an owned `Vec<u8>` or `Vec<u16>`,
impl StringBuffer {
  // The C++ class definition does not declare `string()` to be a const method,
  // therefore we declare self as mutable here.
  // TODO: figure out whether it'd be safe to assume a const receiver here.
  // That would make it possible to implement `Deref<Target = StringBuffer>`.
  pub fn string(&mut self) -> &StringView {
    unsafe { v8_inspector__StringBuffer__string(self) }
  }

  /// This method copies contents.
  pub fn create(source: &StringView) -> UniquePtr<StringBuffer> {
    unsafe { v8_inspector__StringBuffer__create(source) }
  }
}

impl Delete for StringBuffer {
  fn delete(&'static mut self) {
    unsafe { v8_inspector__StringBuffer__DELETE(self) }
  }
}
