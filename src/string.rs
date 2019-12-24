use std::convert::TryInto;
use std::default::Default;
use std::mem::forget;
use std::ops::Deref;
use std::slice;

use crate::support::char;
use crate::support::int;
use crate::support::Opaque;
use crate::InIsolate;
use crate::Isolate;
use crate::Local;
use crate::ToLocal;
use crate::Value;

extern "C" {
  fn v8__String__NewFromUtf8(
    isolate: *mut Isolate,
    data: *const char,
    new_type: NewStringType,
    length: int,
  ) -> *mut String;

  fn v8__String__Length(this: &String) -> int;

  fn v8__String__Utf8Length(this: &String, isolate: *mut Isolate) -> int;

  fn v8__String__WriteUtf8(
    this: &String,
    isolate: *mut Isolate,
    buffer: *mut char,
    length: int,
    nchars_ref: *mut int,
    options: WriteOptions,
  ) -> int;
}

#[repr(C)]
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

/// A JavaScript string value (ECMA-262, 4.3.17).
#[repr(C)]
pub struct String(Opaque);

impl String {
  pub fn new_from_utf8<'sc>(
    scope: &mut impl ToLocal<'sc>,
    buffer: &[u8],
    new_type: NewStringType,
  ) -> Option<Local<'sc, String>> {
    let ptr = unsafe {
      v8__String__NewFromUtf8(
        scope.isolate(),
        buffer.as_ptr() as *const char,
        new_type,
        buffer.len().try_into().ok()?,
      )
    };
    unsafe { scope.to_local(ptr) }
  }

  /// Returns the number of characters (UTF-16 code units) in this string.
  pub fn length(&self) -> usize {
    unsafe { v8__String__Length(self) as usize }
  }

  /// Returns the number of bytes in the UTF-8 encoded representation of this
  /// string.
  pub fn utf8_length(&self, scope: &mut impl InIsolate) -> usize {
    unsafe { v8__String__Utf8Length(self, scope.isolate()) as usize }
  }

  pub fn write_utf8(
    &self,
    scope: &mut impl InIsolate,
    buffer: &mut [u8],
    nchars_ref: Option<&mut usize>,
    options: WriteOptions,
  ) -> usize {
    let mut nchars_ref_int: int = 0;
    let bytes = unsafe {
      v8__String__WriteUtf8(
        self,
        scope.isolate(),
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
  pub fn new<'sc>(
    scope: &mut impl ToLocal<'sc>,
    value: &str,
  ) -> Option<Local<'sc, String>> {
    Self::new_from_utf8(scope, value.as_ref(), NewStringType::Normal)
  }

  // Convenience function not present in the original V8 API.
  pub fn to_rust_string_lossy(
    &self,
    scope: &mut impl InIsolate,
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

impl Deref for String {
  type Target = Value;
  fn deref(&self) -> &Self::Target {
    unsafe { &*(self as *const _ as *const Value) }
  }
}
