use crate::support::CxxVTable;
use crate::support::Delete;
use crate::support::UniquePtr;
use crate::v8::StringView;

// class StringBuffer {
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
  // The C++ class definition does not declare `string()` to be a const method,
  // therefore we declare self as mutable here.
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

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_string_buffer() {
    let chars = b"Hello Venus!";
    let mut buf = {
      let src_view = StringView::from(&chars[..]);
      StringBuffer::create(&src_view)
    };
    let view = buf.as_mut().unwrap().string();

    assert_eq!(chars.len(), view.into_iter().len());
    assert_eq!(chars.len(), view.length());
    for (c1, c2) in chars.iter().copied().map(u16::from).zip(view) {
      assert_eq!(c1, c2);
    }
  }
}
