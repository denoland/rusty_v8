use std::marker::PhantomData;
use std::ops::Deref;
use std::ptr::null;
use std::ptr::NonNull;
use std::slice;

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
//  * Since Rust `repr(bool)` is not allowed, assume `bool` and `u8` have the
//    same size. TODO: find/open upstream issue to allow #[repr(bool)] support.

#[repr(u8)]
#[derive(Copy, Clone, Debug)]
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
  pub fn is8Bit(&self) -> bool {
    match self {
      Self::U16(..) => false,
      Self::U8(..) => true,
    }
  }

  pub fn length(&self) -> usize {
    match self {
      Self::U16(v) => v.m_length,
      Self::U8(v) => v.m_length,
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

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct CharacterArray<'a, T> {
  m_length: usize,
  m_characters: *const T,
  _phantom: PhantomData<&'a T>,
}

impl CharacterArray<'static, u8> {
  fn empty() -> Self {
    Self {
      m_length: 0,
      m_characters: null(),
      _phantom: PhantomData,
    }
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
