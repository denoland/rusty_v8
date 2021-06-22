use std::convert::TryInto;
use std::default::Default;
use std::mem::forget;
use std::slice;

use crate::support::char;
use crate::support::int;
use crate::HandleScope;
use crate::Isolate;
use crate::Local;
use crate::String;

extern "C" {
  fn v8__String__Empty(isolate: *mut Isolate) -> *const String;

  fn v8__String__NewFromUtf8(
    isolate: *mut Isolate,
    data: *const char,
    new_type: NewStringType,
    length: int,
  ) -> *const String;

  fn v8__String__NewFromOneByte(
    isolate: *mut Isolate,
    data: *const u8,
    new_type: NewStringType,
    length: int,
  ) -> *const String;

  fn v8__String__NewFromTwoByte(
    isolate: *mut Isolate,
    data: *const u16,
    new_type: NewStringType,
    length: int,
  ) -> *const String;

  fn v8__String__Length(this: *const String) -> int;

  fn v8__String__Utf8Length(this: *const String, isolate: *mut Isolate) -> int;

  fn v8__String__Write(
    this: *const String,
    isolate: *mut Isolate,
    buffer: *mut u16,
    start: int,
    length: int,
    options: WriteOptions,
  ) -> int;

  fn v8__String__WriteOneByte(
    this: *const String,
    isolate: *mut Isolate,
    buffer: *mut u8,
    start: int,
    length: int,
    options: WriteOptions,
  ) -> int;

  fn v8__String__WriteUtf8(
    this: *const String,
    isolate: *mut Isolate,
    buffer: *mut char,
    length: int,
    nchars_ref: *mut int,
    options: WriteOptions,
  ) -> int;

  fn v8__String__NewExternalOneByteStatic(
    isolate: *mut Isolate,
    buffer: *const char,
    length: int,
  ) -> *const String;

  fn v8__String__NewExternalTwoByteStatic(
    isolate: *mut Isolate,
    buffer: *const u16,
    length: int,
  ) -> *const String;

  fn v8__String__IsExternal(this: *const String) -> bool;
  fn v8__String__IsExternalOneByte(this: *const String) -> bool;
  fn v8__String__IsExternalTwoByte(this: *const String) -> bool;
  fn v8__String__IsOneByte(this: *const String) -> bool;
  fn v8__String__ContainsOnlyOneByte(this: *const String) -> bool;
}

#[repr(C)]
#[derive(Debug)]
pub enum NewStringType {
  Normal,
  Internalized,
}

impl Default for NewStringType {
  fn default() -> Self {
    NewStringType::Normal
  }
}

bitflags! {
  #[derive(Default)]
  #[repr(transparent)]
  pub struct WriteOptions: int {
    const NO_OPTIONS = 0;
    const HINT_MANY_WRITES_EXPECTED = 1;
    const NO_NULL_TERMINATION = 2;
    const PRESERVE_ONE_BYTE_NULL = 4;
    // Used by WriteUtf8 to replace orphan surrogate code units with the
    // unicode replacement character. Needs to be set to guarantee valid UTF-8
    // output.
    const REPLACE_INVALID_UTF8 = 8;
  }
}

impl String {
  pub fn empty<'s>(scope: &mut HandleScope<'s, ()>) -> Local<'s, String> {
    // FIXME(bnoordhuis) v8__String__Empty() is infallible so there
    // is no need to box up the result, only to unwrap it again.
    unsafe { scope.cast_local(|sd| v8__String__Empty(sd.get_isolate_ptr())) }
      .unwrap()
  }

  /// Allocates a new string from UTF-8 data. Only returns an empty value when
  /// length > kMaxLength
  pub fn new_from_utf8<'s>(
    scope: &mut HandleScope<'s, ()>,
    buffer: &[u8],
    new_type: NewStringType,
  ) -> Option<Local<'s, String>> {
    if buffer.is_empty() {
      return Some(Self::empty(scope));
    }
    let buffer_len = buffer.len().try_into().ok()?;
    unsafe {
      scope.cast_local(|sd| {
        v8__String__NewFromUtf8(
          sd.get_isolate_ptr(),
          buffer.as_ptr() as *const char,
          new_type,
          buffer_len,
        )
      })
    }
  }

  /// Allocates a new string from Latin-1 data.  Only returns an empty value when
  /// length > kMaxLength.
  pub fn new_from_one_byte<'s>(
    scope: &mut HandleScope<'s, ()>,
    buffer: &[u8],
    new_type: NewStringType,
  ) -> Option<Local<'s, String>> {
    let buffer_len = buffer.len().try_into().ok()?;
    unsafe {
      scope.cast_local(|sd| {
        v8__String__NewFromOneByte(
          sd.get_isolate_ptr(),
          buffer.as_ptr(),
          new_type,
          buffer_len,
        )
      })
    }
  }

  /// Allocates a new string from UTF-16 data. Only returns an empty value when
  /// length > kMaxLength.
  pub fn new_from_two_byte<'s>(
    scope: &mut HandleScope<'s, ()>,
    buffer: &[u16],
    new_type: NewStringType,
  ) -> Option<Local<'s, String>> {
    let buffer_len = buffer.len().try_into().ok()?;
    unsafe {
      scope.cast_local(|sd| {
        v8__String__NewFromTwoByte(
          sd.get_isolate_ptr(),
          buffer.as_ptr(),
          new_type,
          buffer_len,
        )
      })
    }
  }

  /// Returns the number of characters (UTF-16 code units) in this string.
  pub fn length(&self) -> usize {
    unsafe { v8__String__Length(self) as usize }
  }

  /// Returns the number of bytes in the UTF-8 encoded representation of this
  /// string.
  pub fn utf8_length(&self, scope: &mut Isolate) -> usize {
    unsafe { v8__String__Utf8Length(self, scope) as usize }
  }

  /// Writes the contents of the string to an external buffer, as 16-bit
  /// (UTF-16) character codes.
  pub fn write(
    &self,
    scope: &mut Isolate,
    buffer: &mut [u16],
    start: usize,
    options: WriteOptions,
  ) -> usize {
    unsafe {
      v8__String__Write(
        self,
        scope,
        buffer.as_mut_ptr(),
        start.try_into().unwrap_or(int::max_value()),
        buffer.len().try_into().unwrap_or(int::max_value()),
        options,
      ) as usize
    }
  }

  /// Writes the contents of the string to an external buffer, as one-byte
  /// (Latin-1) characters.
  pub fn write_one_byte(
    &self,
    scope: &mut Isolate,
    buffer: &mut [u8],
    start: usize,
    options: WriteOptions,
  ) -> usize {
    unsafe {
      v8__String__WriteOneByte(
        self,
        scope,
        buffer.as_mut_ptr(),
        start.try_into().unwrap_or(int::max_value()),
        buffer.len().try_into().unwrap_or(int::max_value()),
        options,
      ) as usize
    }
  }

  /// Writes the contents of the string to an external buffer, as UTF-8.
  pub fn write_utf8(
    &self,
    scope: &mut Isolate,
    buffer: &mut [u8],
    nchars_ref: Option<&mut usize>,
    options: WriteOptions,
  ) -> usize {
    let mut nchars_ref_int: int = 0;
    let bytes = unsafe {
      v8__String__WriteUtf8(
        self,
        scope,
        buffer.as_mut_ptr() as *mut char,
        buffer.len().try_into().unwrap_or(int::max_value()),
        &mut nchars_ref_int,
        options,
      )
    };
    if let Some(r) = nchars_ref {
      *r = nchars_ref_int as usize;
    }
    bytes as usize
  }

  // Convenience function not present in the original V8 API.
  pub fn new<'s>(
    scope: &mut HandleScope<'s, ()>,
    value: &str,
  ) -> Option<Local<'s, String>> {
    Self::new_from_utf8(scope, value.as_ref(), NewStringType::Normal)
  }

  // Creates a v8::String from a `&'static [u8]`,
  // must be Latin-1 or ASCII, not UTF-8 !
  pub fn new_external_onebyte_static<'s>(
    scope: &mut HandleScope<'s, ()>,
    buffer: &'static [u8],
  ) -> Option<Local<'s, String>> {
    let buffer_len = buffer.len().try_into().ok()?;
    unsafe {
      scope.cast_local(|sd| {
        v8__String__NewExternalOneByteStatic(
          sd.get_isolate_ptr(),
          buffer.as_ptr() as *const char,
          buffer_len,
        )
      })
    }
  }

  // Creates a v8::String from a `&'static [u16]`.
  pub fn new_external_twobyte_static<'s>(
    scope: &mut HandleScope<'s, ()>,
    buffer: &'static [u16],
  ) -> Option<Local<'s, String>> {
    let buffer_len = buffer.len().try_into().ok()?;
    unsafe {
      scope.cast_local(|sd| {
        v8__String__NewExternalTwoByteStatic(
          sd.get_isolate_ptr(),
          buffer.as_ptr(),
          buffer_len,
        )
      })
    }
  }

  /// True if string is external
  pub fn is_external(&self) -> bool {
    // TODO: re-enable on next v8-release
    // Right now it fallbacks to Value::IsExternal, which is incorrect
    // See: https://source.chromium.org/chromium/_/chromium/v8/v8.git/+/1dd8624b524d14076160c1743f7da0b20fbe68e0
    // unsafe { v8__String__IsExternal(self) }

    // Fallback for now (though functionally identical)
    self.is_external_onebyte() || self.is_external_twobyte()
  }

  /// True if string is external & one-byte
  /// (e.g: created with new_external_onebyte_static)
  pub fn is_external_onebyte(&self) -> bool {
    unsafe { v8__String__IsExternalOneByte(self) }
  }

  /// True if string is external & two-byte
  /// (e.g: created with new_external_twobyte_static)
  pub fn is_external_twobyte(&self) -> bool {
    unsafe { v8__String__IsExternalTwoByte(self) }
  }

  /// True if string is known to contain only one-byte data.
  /// Doesn't read the string so can return false positives.
  ///
  /// For a method that will not return false positives at the cost of
  /// potentially reading the entire string, use [`contains_only_onebyte()`].
  ///
  /// [`contains_only_onebyte()`]: String::contains_only_onebyte
  pub fn is_onebyte(&self) -> bool {
    unsafe { v8__String__IsExternalOneByte(self) }
  }

  /// True if the string contains only one-byte data.
  /// Will read the entire string in some cases.
  pub fn contains_only_onebyte(&self) -> bool {
    unsafe { v8__String__ContainsOnlyOneByte(self) }
  }

  /// Convenience function not present in the original V8 API.
  pub fn to_rust_string_lossy(
    &self,
    scope: &mut Isolate,
  ) -> std::string::String {
    let capacity = self.utf8_length(scope);
    let mut string = std::string::String::with_capacity(capacity);
    let data = string.as_mut_ptr();
    forget(string);
    let length = self.write_utf8(
      scope,
      unsafe { slice::from_raw_parts_mut(data, capacity) },
      None,
      WriteOptions::NO_NULL_TERMINATION | WriteOptions::REPLACE_INVALID_UTF8,
    );
    unsafe { std::string::String::from_raw_parts(data, length, capacity) }
  }
}
