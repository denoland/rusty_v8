// Copyright 2019-2021 the Deno authors. All rights reserved. MIT license.
use crate::support::size_t;
use crate::ArrayBuffer;
use crate::HandleScope;
use crate::Local;
use crate::TypedArray;

extern "C" {
  fn v8__TypedArray__kMaxByteLength() -> size_t;
  fn v8__TypedArray__Length(this: *const TypedArray) -> size_t;
}

impl TypedArray {
  /// The largest supported typed array byte size. Each subclass defines a
  /// type-specific max_length for the maximum length that can be passed to new.
  #[inline(always)]
  pub fn max_byte_length() -> usize {
    unsafe { v8__TypedArray__kMaxByteLength() }
  }

  /// Number of elements in this typed array
  /// (e.g. for Int16Array, |ByteLength|/2).
  #[inline(always)]
  pub fn length(&self) -> usize {
    unsafe { v8__TypedArray__Length(self) }
  }
}

macro_rules! typed_array {
  ($name:ident, $new_func:ident, $max_length_func:ident) => {
    use crate::$name;
    impl $name {
      #[inline(always)]
      pub fn new<'s>(
        scope: &mut HandleScope<'s>,
        buf: Local<ArrayBuffer>,
        byte_offset: usize,
        length: usize,
      ) -> Option<Local<'s, $name>> {
        extern "C" {
          fn $new_func(
            buf_ptr: *const ArrayBuffer,
            byte_offset: usize,
            length: usize,
          ) -> *const $name;
        }
        unsafe { scope.cast_local(|_| $new_func(&*buf, byte_offset, length)) }
      }

      #[doc = concat!("The largest ", stringify!($name), " size that can be constructed using `new`.")]
      #[inline(always)]
      pub fn max_length() -> usize {
        extern "C" {
          fn $max_length_func() -> usize;
        }
        unsafe { $max_length_func() }
      }

    }
  };
}

typed_array!(Uint8Array, v8__Uint8Array__New, v8__Uint8Array__kMaxLength);
typed_array!(
  Uint8ClampedArray,
  v8__Uint8ClampedArray__New,
  v8__Uint8ClampedArray__kMaxLength
);
typed_array!(Int8Array, v8__Int8Array__New, v8__Int8Array__kMaxLength);
typed_array!(
  Uint16Array,
  v8__Uint16Array__New,
  v8__Uint16Array__kMaxLength
);
typed_array!(Int16Array, v8__Int16Array__New, v8__Int16Array__kMaxLength);
typed_array!(
  Uint32Array,
  v8__Uint32Array__New,
  v8__Uint32Array__kMaxLength
);
typed_array!(Int32Array, v8__Int32Array__New, v8__Int32Array__kMaxLength);
typed_array!(
  Float32Array,
  v8__Float32Array__New,
  v8__Float32Array__kMaxLength
);
typed_array!(
  Float64Array,
  v8__Float64Array__New,
  v8__Float64Array__kMaxLength
);
typed_array!(
  BigUint64Array,
  v8__BigUint64Array__New,
  v8__BigUint64Array__kMaxLength
);
typed_array!(
  BigInt64Array,
  v8__BigInt64Array__New,
  v8__BigInt64Array__kMaxLength
);
