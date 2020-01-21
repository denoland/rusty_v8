use std::fmt;
use std::iter::ExactSizeIterator;
use std::iter::IntoIterator;
use std::marker::PhantomData;
use std::ops::Deref;
use std::ptr::null;
use std::ptr::NonNull;
use std::slice;
use std::string;

// class StringView {
//  public:
//   StringView() : m_is8Bit(true), m_length(0), m_characters8(nullptr) {}
//
//   StringView(const uint8_t* characters, size_t length)
//       : m_is8Bit(true), m_length(length), m_characters8(characters) {}
//
//   StringView(const uint16_t* characters, size_t length)
//       : m_is8Bit(false), m_length(length), m_characters16(characters) {}
//
//   bool is8Bit() const { return m_is8Bit; }
//   size_t length() const { return m_length; }
//
//   const uint8_t* characters8() const { return m_characters8; }
//   const uint16_t* characters16() const { return m_characters16; }
//
//  private:
//   bool m_is8Bit;
//   size_t m_length;
//   union {
//     const uint8_t* m_characters8;
//     const uint16_t* m_characters16;
//   };
// };

// Notes:
//  * This class is ported, not wrapped using bindings.
//  * Since Rust `repr(bool)` is not allowed, we're assuming that `bool` and
//    `u8` have the same size. This is assumption is checked in 'support.h'.
//    TODO: find/open upstream issue to allow #[repr(bool)] support.

#[repr(u8)]
pub enum StringView<'a> {
  // Do not reorder!
  U16(CharacterArray<'a, u16>),
  U8(CharacterArray<'a, u8>),
}

impl StringView<'static> {
  pub fn empty() -> Self {
    Self::U8(CharacterArray::<'static, u8>::empty())
  }
}

impl fmt::Display for StringView<'_> {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      Self::U16(v) => v.fmt(f),
      Self::U8(v) => v.fmt(f),
    }
  }
}

impl<'a> From<&'a [u8]> for StringView<'a> {
  fn from(v: &'a [u8]) -> Self {
    Self::U8(CharacterArray::<'a, u8>::from(v))
  }
}

impl<'a> From<&'a [u16]> for StringView<'a> {
  fn from(v: &'a [u16]) -> Self {
    Self::U16(CharacterArray::<'a, u16>::from(v))
  }
}

impl<'a> StringView<'a> {
  pub fn is_8bit(&self) -> bool {
    match self {
      Self::U16(..) => false,
      Self::U8(..) => true,
    }
  }

  pub fn is_empty(&self) -> bool {
    self.len() == 0
  }

  pub fn len(&self) -> usize {
    match self {
      Self::U16(v) => v.len(),
      Self::U8(v) => v.len(),
    }
  }

  pub fn characters8(&self) -> Option<&[u8]> {
    match self {
      Self::U16(..) => None,
      Self::U8(v) => Some(v),
    }
  }

  pub fn characters16(&self) -> Option<&[u16]> {
    match self {
      Self::U16(v) => Some(v),
      Self::U8(..) => None,
    }
  }
}

impl<'a: 'b, 'b> IntoIterator for &'a StringView<'b> {
  type IntoIter = StringViewIterator<'a, 'b>;
  type Item = u16;

  fn into_iter(self) -> Self::IntoIter {
    StringViewIterator { view: self, pos: 0 }
  }
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct CharacterArray<'a, T> {
  m_length: usize,
  m_characters: *const T,
  _phantom: PhantomData<&'a T>,
}

impl CharacterArray<'static, u8> {
  pub fn empty() -> Self {
    Self {
      m_length: 0,
      m_characters: null(),
      _phantom: PhantomData,
    }
  }
}

impl<'a, T> CharacterArray<'a, T>
where
  T: Copy,
{
  #[inline(always)]
  fn len(&self) -> usize {
    self.m_length
  }

  #[inline(always)]
  fn get_at(&self, index: usize) -> Option<T> {
    if index < self.m_length {
      Some(unsafe { *self.m_characters.add(index) })
    } else {
      None
    }
  }
}

unsafe impl<'a, T> Send for CharacterArray<'a, T> where T: Copy {}
unsafe impl<'a, T> Sync for CharacterArray<'a, T> where T: Sync {}

impl fmt::Display for CharacterArray<'_, u8> {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    f.write_str(&string::String::from_utf8_lossy(&*self))
  }
}

impl fmt::Display for CharacterArray<'_, u16> {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    f.write_str(&string::String::from_utf16_lossy(&*self))
  }
}

impl<'a, T> From<&'a [T]> for CharacterArray<'a, T> {
  fn from(v: &'a [T]) -> Self {
    Self {
      m_length: v.len(),
      m_characters: v.as_ptr(),
      _phantom: PhantomData,
    }
  }
}

impl<'a, T> Deref for CharacterArray<'a, T> {
  type Target = [T];

  fn deref(&self) -> &[T] {
    let Self {
      m_length,
      mut m_characters,
      ..
    } = *self;
    if m_characters.is_null() {
      assert_eq!(m_length, 0);
      m_characters = NonNull::dangling().as_ptr()
    };
    unsafe { slice::from_raw_parts(m_characters, m_length) }
  }
}

#[derive(Copy, Clone)]
pub struct StringViewIterator<'a: 'b, 'b> {
  view: &'a StringView<'b>,
  pos: usize,
}

impl<'a: 'b, 'b> Iterator for StringViewIterator<'a, 'b> {
  type Item = u16;

  fn next(&mut self) -> Option<Self::Item> {
    let result = Some(match self.view {
      StringView::U16(v) => v.get_at(self.pos)?,
      StringView::U8(v) => u16::from(v.get_at(self.pos)?),
    });
    self.pos += 1;
    result
  }
}

impl<'a: 'b, 'b> ExactSizeIterator for StringViewIterator<'a, 'b> {
  fn len(&self) -> usize {
    self.view.len()
  }
}

#[test]
fn string_view_display() {
  let ok: [u16; 2] = [111, 107];
  assert_eq!("ok", format!("{}", StringView::from(&ok[..])));
  assert_eq!("ok", format!("{}", StringView::from(&b"ok"[..])));
}
