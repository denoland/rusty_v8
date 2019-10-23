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

mod tests {
  use super::*;

  #[test]
  fn test_string_buffer() {
    let chars = b"Hello Venus!";
    let mut buf = {
      let view1 = StringView::from(&chars[..]);
      StringBuffer::create(&view1)
    };
    let view2 = buf.as_mut().unwrap().string();

    let mut count = 0usize;
    for (c1, c2) in chars.iter().copied().map(|c| c as u16).zip(view2) {
      assert_eq!(c1, c2);
      count += 1;
    }
    assert_eq!(count, chars.len());
    assert_eq!(count, view2.length());
  }
}
