use std::borrow::Cow;
use std::convert::TryInto;
use std::default::Default;
use std::mem::MaybeUninit;
use std::slice;

use crate::support::char;
use crate::support::int;
use crate::support::size_t;
use crate::HandleScope;
use crate::Isolate;
use crate::Local;
use crate::String;

extern "C" {
  fn v8__String__kMaxLength() -> size_t;

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

  fn v8__String__NewExternalOneByte(
    isolate: *mut Isolate,
    onebyte_const: *const OneByteConst,
  ) -> *const String;

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

  #[allow(dead_code)]
  fn v8__String__IsExternal(this: *const String) -> bool;
  fn v8__String__IsExternalOneByte(this: *const String) -> bool;
  fn v8__String__IsExternalTwoByte(this: *const String) -> bool;
  #[allow(dead_code)]
  fn v8__String__IsOneByte(this: *const String) -> bool;
  fn v8__String__ContainsOnlyOneByte(this: *const String) -> bool;
}

#[repr(C)]
#[derive(Debug)]
pub struct OneByteConst {
  vtable: *const OneByteConstNoOp,
  cached_data: *const char,
  length: usize,
}

// SAFETY: The vtable for OneByteConst is an immutable static and all
// of the included functions are thread-safe, the cached_data pointer
// is never changed and points to a static ASCII string, and the
// length is likewise never changed. Thus, it is safe to share the
// OneByteConst across threads. This means that multiple isolates
// can use the same OneByteConst statics simultaneously.
unsafe impl Sync for OneByteConst {}

extern "C" fn one_byte_const_no_op(_this: *const OneByteConst) {}
extern "C" fn one_byte_const_is_cacheable(_this: *const OneByteConst) -> bool {
  true
}
extern "C" fn one_byte_const_data(this: *const OneByteConst) -> *const char {
  // SAFETY: Only called from C++ with a valid OneByteConst pointer.
  unsafe { (*this).cached_data }
}
extern "C" fn one_byte_const_length(this: *const OneByteConst) -> usize {
  // SAFETY: Only called from C++ with a valid OneByteConst pointer.
  unsafe { (*this).length }
}

type OneByteConstNoOp = extern "C" fn(*const OneByteConst);
type OneByteConstIsCacheable = extern "C" fn(*const OneByteConst) -> bool;
type OneByteConstData = extern "C" fn(*const OneByteConst) -> *const char;
type OneByteConstLength = extern "C" fn(*const OneByteConst) -> usize;

#[repr(C)]
struct OneByteConstVtable {
  #[cfg(target_family = "windows")]
  // In SysV / Itanium ABI -0x10 offset of the vtable
  // tells how many bytes the vtable pointer pointing to
  // this vtable is offset from the base class. For
  // single inheritance this is always 0.
  _offset_to_top: usize,
  // In Itanium ABI the -0x08 offset contains the type_info
  // pointer, and in MSVC it contains the RTTI Complete Object
  // Locator pointer. V8 is normally compiled with `-fno-rtti`
  // meaning that this pointer is a nullptr on both
  // Itanium and MSVC.
  _typeinfo: *const (),
  // After the metadata fields come the virtual function
  // pointers. The vtable pointer in a class instance points
  // to the first virtual function pointer, making this
  // the 0x00 offset of the table.
  // The order of the virtual function pointers is determined
  // by their order of declaration in the classes.
  delete1: OneByteConstNoOp,
  // In SysV / Itanium ABI, a class vtable includes the
  // deleting destructor and the compete object destructor.
  // In MSVC, it only includes the deleting destructor.
  #[cfg(not(target_family = "windows"))]
  delete2: OneByteConstNoOp,
  is_cacheable: OneByteConstIsCacheable,
  dispose: OneByteConstNoOp,
  lock: OneByteConstNoOp,
  unlock: OneByteConstNoOp,
  data: OneByteConstData,
  length: OneByteConstLength,
}

const ONE_BYTE_CONST_VTABLE: OneByteConstVtable = OneByteConstVtable {
  #[cfg(target_family = "windows")]
  _offset_to_top: 0,
  _typeinfo: std::ptr::null(),
  delete1: one_byte_const_no_op,
  #[cfg(not(target_family = "windows"))]
  delete2: one_byte_const_no_op,
  is_cacheable: one_byte_const_is_cacheable,
  dispose: one_byte_const_no_op,
  lock: one_byte_const_no_op,
  unlock: one_byte_const_no_op,
  data: one_byte_const_data,
  length: one_byte_const_length,
};

/// Compile-time function to determine if a string is ASCII. Note that UTF-8 chars
/// longer than one byte have the high-bit set and thus, are not ASCII.
const fn is_ascii(s: &'static [u8]) -> bool {
  let mut i = 0;
  while i < s.len() {
    if !s[i].is_ascii() {
      return false;
    }
    i += 1;
  }
  true
}

#[repr(C)]
#[derive(Debug, Default)]
pub enum NewStringType {
  #[default]
  Normal,
  Internalized,
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
  /// The maximum length (in bytes) of a buffer that a v8::String can be built
  /// from. Attempting to create a v8::String from a larger buffer will result
  /// in None being returned.
  #[inline(always)]
  pub fn max_length() -> usize {
    unsafe { v8__String__kMaxLength() }
  }

  #[inline(always)]
  pub fn empty<'s>(scope: &mut HandleScope<'s, ()>) -> Local<'s, String> {
    // FIXME(bnoordhuis) v8__String__Empty() is infallible so there
    // is no need to box up the result, only to unwrap it again.
    unsafe { scope.cast_local(|sd| v8__String__Empty(sd.get_isolate_ptr())) }
      .unwrap()
  }

  /// Allocates a new string from UTF-8 data. Only returns an empty value when
  /// length > kMaxLength
  #[inline(always)]
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
  #[inline(always)]
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
  #[inline(always)]
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
  #[inline(always)]
  pub fn length(&self) -> usize {
    unsafe { v8__String__Length(self) as usize }
  }

  /// Returns the number of bytes in the UTF-8 encoded representation of this
  /// string.
  #[inline(always)]
  pub fn utf8_length(&self, scope: &mut Isolate) -> usize {
    unsafe { v8__String__Utf8Length(self, scope) as usize }
  }

  /// Writes the contents of the string to an external buffer, as 16-bit
  /// (UTF-16) character codes.
  #[inline(always)]
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
  #[inline(always)]
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

  /// Writes the contents of the string to an external [`MaybeUninit`] buffer, as one-byte
  /// (Latin-1) characters.
  #[inline(always)]
  pub fn write_one_byte_uninit(
    &self,
    scope: &mut Isolate,
    buffer: &mut [MaybeUninit<u8>],
    start: usize,
    options: WriteOptions,
  ) -> usize {
    unsafe {
      v8__String__WriteOneByte(
        self,
        scope,
        buffer.as_mut_ptr() as *mut u8,
        start.try_into().unwrap_or(int::max_value()),
        buffer.len().try_into().unwrap_or(int::max_value()),
        options,
      ) as usize
    }
  }

  /// Writes the contents of the string to an external buffer, as UTF-8.
  #[inline(always)]
  pub fn write_utf8(
    &self,
    scope: &mut Isolate,
    buffer: &mut [u8],
    nchars_ref: Option<&mut usize>,
    options: WriteOptions,
  ) -> usize {
    unsafe {
      // SAFETY:
      // We assume that v8 will overwrite the buffer without de-initializing any byte in it.
      // So the type casting of the buffer is safe.

      let buffer = {
        let len = buffer.len();
        let data = buffer.as_mut_ptr().cast();
        slice::from_raw_parts_mut(data, len)
      };
      self.write_utf8_uninit(scope, buffer, nchars_ref, options)
    }
  }

  /// Writes the contents of the string to an external [`MaybeUninit`] buffer, as UTF-8.
  pub fn write_utf8_uninit(
    &self,
    scope: &mut Isolate,
    buffer: &mut [MaybeUninit<u8>],
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
  #[inline(always)]
  pub fn new<'s>(
    scope: &mut HandleScope<'s, ()>,
    value: &str,
  ) -> Option<Local<'s, String>> {
    Self::new_from_utf8(scope, value.as_ref(), NewStringType::Normal)
  }

  // Compile-time function to create an external string resource.
  // The buffer is checked to contain only ASCII characters.
  #[inline(always)]
  pub const fn create_external_onebyte_const(
    buffer: &'static [u8],
  ) -> OneByteConst {
    // Assert that the buffer contains only ASCII, and that the
    // length is less or equal to (64-bit) v8::String::kMaxLength.
    assert!(is_ascii(buffer) && buffer.len() <= ((1 << 29) - 24));
    OneByteConst {
      vtable: &ONE_BYTE_CONST_VTABLE.delete1,
      cached_data: buffer.as_ptr() as *const char,
      length: buffer.len(),
    }
  }

  // Creates a v8::String from a `&'static OneByteConst`
  // which is guaranteed to be Latin-1 or ASCII.
  #[inline(always)]
  pub fn new_from_onebyte_const<'s>(
    scope: &mut HandleScope<'s, ()>,
    onebyte_const: &'static OneByteConst,
  ) -> Option<Local<'s, String>> {
    unsafe {
      scope.cast_local(|sd| {
        v8__String__NewExternalOneByte(sd.get_isolate_ptr(), onebyte_const)
      })
    }
  }

  // Creates a v8::String from a `&'static [u8]`,
  // must be Latin-1 or ASCII, not UTF-8 !
  #[inline(always)]
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
  #[inline(always)]
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
  #[inline(always)]
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
  #[inline(always)]
  pub fn is_external_onebyte(&self) -> bool {
    unsafe { v8__String__IsExternalOneByte(self) }
  }

  /// True if string is external & two-byte
  /// (e.g: created with new_external_twobyte_static)
  #[inline(always)]
  pub fn is_external_twobyte(&self) -> bool {
    unsafe { v8__String__IsExternalTwoByte(self) }
  }

  /// Will return true if and only if string is known for certain to contain only one-byte data,
  /// ie: Latin-1, a.k.a. ISO-8859-1 code points. Doesn't read the string so can return false
  /// negatives, and a return value of false does not mean this string is not one-byte data.
  ///
  /// For a method that will not return false negatives at the cost of
  /// potentially reading the entire string, use [`contains_only_onebyte()`].
  ///
  /// [`contains_only_onebyte()`]: String::contains_only_onebyte
  #[inline(always)]
  pub fn is_onebyte(&self) -> bool {
    unsafe { v8__String__IsOneByte(self) }
  }

  /// True if the string contains only one-byte data.
  /// Will read the entire string in some cases.
  #[inline(always)]
  pub fn contains_only_onebyte(&self) -> bool {
    unsafe { v8__String__ContainsOnlyOneByte(self) }
  }

  /// Creates a copy of a [`crate::String`] in a [`std::string::String`].
  /// Convenience function not present in the original V8 API.
  pub fn to_rust_string_lossy(
    &self,
    scope: &mut Isolate,
  ) -> std::string::String {
    let len_utf16 = self.length();

    // No need to allocate or do any work for zero-length strings
    if len_utf16 == 0 {
      return std::string::String::new();
    }

    let len_utf8 = self.utf8_length(scope);

    // If len_utf8 == len_utf16 and the string is one-byte, we can take the fast memcpy path. This is true iff the
    // string is 100% 7-bit ASCII.
    if self.is_onebyte() && len_utf8 == len_utf16 {
      unsafe {
        // Create an uninitialized buffer of `capacity` bytes. We need to be careful here to avoid
        // accidentally creating a slice of u8 which would be invalid.
        let layout = std::alloc::Layout::from_size_align(len_utf16, 1).unwrap();
        let data = std::alloc::alloc(layout) as *mut MaybeUninit<u8>;
        let buffer = std::ptr::slice_from_raw_parts_mut(data, len_utf16);

        // Write to this MaybeUninit buffer, assuming we're going to fill this entire buffer
        let length = self.write_one_byte_uninit(
          scope,
          &mut *buffer,
          0,
          WriteOptions::NO_NULL_TERMINATION
            | WriteOptions::REPLACE_INVALID_UTF8,
        );
        debug_assert!(length == len_utf16);

        // Return an owned string from this guaranteed now-initialized data
        let buffer = data as *mut u8;
        return std::string::String::from_raw_parts(buffer, length, len_utf16);
      }
    }

    // SAFETY: This allocates a buffer manually using the default allocator using the string's capacity.
    // We have a large number of invariants to uphold, so please check changes to this code carefully
    unsafe {
      // Create an uninitialized buffer of `capacity` bytes. We need to be careful here to avoid
      // accidentally creating a slice of u8 which would be invalid.
      let layout = std::alloc::Layout::from_size_align(len_utf8, 1).unwrap();
      let data = std::alloc::alloc(layout) as *mut MaybeUninit<u8>;
      let buffer = std::ptr::slice_from_raw_parts_mut(data, len_utf8);

      // Write to this MaybeUninit buffer, assuming we're going to fill this entire buffer
      let length = self.write_utf8_uninit(
        scope,
        &mut *buffer,
        None,
        WriteOptions::NO_NULL_TERMINATION | WriteOptions::REPLACE_INVALID_UTF8,
      );
      debug_assert!(length == len_utf8);

      // Return an owned string from this guaranteed now-initialized data
      let buffer = data as *mut u8;
      std::string::String::from_raw_parts(buffer, length, len_utf8)
    }
  }

  /// Converts a [`crate::String`] to either an owned [`std::string::String`], or a borrowed [`str`], depending on whether it fits into the
  /// provided buffer.
  pub fn to_rust_cow_lossy<'a, const N: usize>(
    &self,
    scope: &mut Isolate,
    buffer: &'a mut [MaybeUninit<u8>; N],
  ) -> Cow<'a, str> {
    let len_utf16 = self.length();

    // No need to allocate or do any work for zero-length strings
    if len_utf16 == 0 {
      return "".into();
    }

    // TODO(mmastrac): Ideally we should be able to access the string's internal representation
    let len_utf8 = self.utf8_length(scope);

    // If len_utf8 == len_utf16 and the string is one-byte, we can take the fast memcpy path. This is true iff the
    // string is 100% 7-bit ASCII.
    if self.is_onebyte() && len_utf8 == len_utf16 {
      if len_utf16 <= N {
        let length = self.write_one_byte_uninit(
          scope,
          buffer,
          0,
          WriteOptions::NO_NULL_TERMINATION,
        );
        debug_assert!(length == len_utf16);
        unsafe {
          // Get a slice of &[u8] of what we know is initialized now
          let buffer = &mut buffer[..length];
          let buffer = &mut *(buffer as *mut [_] as *mut [u8]);

          // We know it's valid UTF-8, so make a string
          return Cow::Borrowed(std::str::from_utf8_unchecked(buffer));
        }
      }

      unsafe {
        // Create an uninitialized buffer of `capacity` bytes. We need to be careful here to avoid
        // accidentally creating a slice of u8 which would be invalid.
        let layout = std::alloc::Layout::from_size_align(len_utf16, 1).unwrap();
        let data = std::alloc::alloc(layout) as *mut MaybeUninit<u8>;
        let buffer = std::ptr::slice_from_raw_parts_mut(data, len_utf16);

        // Write to this MaybeUninit buffer, assuming we're going to fill this entire buffer
        let length = self.write_one_byte_uninit(
          scope,
          &mut *buffer,
          0,
          WriteOptions::NO_NULL_TERMINATION
            | WriteOptions::REPLACE_INVALID_UTF8,
        );
        debug_assert!(length == len_utf16);

        // Return an owned string from this guaranteed now-initialized data
        let buffer = data as *mut u8;
        return Cow::Owned(std::string::String::from_raw_parts(
          buffer, length, len_utf16,
        ));
      }
    }

    if len_utf8 <= N {
      // No malloc path
      let length = self.write_utf8_uninit(
        scope,
        buffer,
        None,
        WriteOptions::NO_NULL_TERMINATION | WriteOptions::REPLACE_INVALID_UTF8,
      );
      debug_assert!(length == len_utf8);

      // SAFETY: We know that we wrote `length` UTF-8 bytes. See `slice_assume_init_mut` for additional guarantee information.
      unsafe {
        // Get a slice of &[u8] of what we know is initialized now
        let buffer = &mut buffer[..length];
        let buffer = &mut *(buffer as *mut [_] as *mut [u8]);

        // We know it's valid UTF-8, so make a string
        return Cow::Borrowed(std::str::from_utf8_unchecked(buffer));
      }
    }

    // SAFETY: This allocates a buffer manually using the default allocator using the string's capacity.
    // We have a large number of invariants to uphold, so please check changes to this code carefully
    unsafe {
      // Create an uninitialized buffer of `capacity` bytes. We need to be careful here to avoid
      // accidentally creating a slice of u8 which would be invalid.
      let layout = std::alloc::Layout::from_size_align(len_utf8, 1).unwrap();
      let data = std::alloc::alloc(layout) as *mut MaybeUninit<u8>;
      let buffer = std::ptr::slice_from_raw_parts_mut(data, len_utf8);

      // Write to this MaybeUninit buffer, assuming we're going to fill this entire buffer
      let length = self.write_utf8_uninit(
        scope,
        &mut *buffer,
        None,
        WriteOptions::NO_NULL_TERMINATION | WriteOptions::REPLACE_INVALID_UTF8,
      );
      debug_assert!(length == len_utf8);

      // Return an owned string from this guaranteed now-initialized data
      let buffer = data as *mut u8;
      Cow::Owned(std::string::String::from_raw_parts(
        buffer, length, len_utf8,
      ))
    }
  }
}
