// Copyright 2019-2021 the Deno authors. All rights reserved. MIT license.
use crate::ArrayBuffer;
use crate::HandleScope;
use crate::Local;

macro_rules! typed_array {
  ($name:ident, $func:ident) => {
    use crate::$name;
    impl $name {
      pub fn new<'s>(
        scope: &mut HandleScope<'s>,
        buf: Local<ArrayBuffer>,
        byte_offset: usize,
        length: usize,
      ) -> Option<Local<'s, $name>> {
        extern "C" {
          fn $func(
            buf_ptr: *const ArrayBuffer,
            byte_offset: usize,
            length: usize,
          ) -> *const $name;
        }
        unsafe { scope.cast_local(|_| $func(&*buf, byte_offset, length)) }
      }
    }
  };
}

typed_array!(Uint8Array, v8__Uint8Array__New);
typed_array!(Uint8ClampedArray, v8__Uint8ClampedArray__New);
typed_array!(Int8Array, v8__Int8Array__New);
typed_array!(Uint16Array, v8__Uint16Array__New);
typed_array!(Int16Array, v8__Int16Array__New);
typed_array!(Uint32Array, v8__Uint32Array__New);
typed_array!(Int32Array, v8__Int32Array__New);
typed_array!(Float32Array, v8__Float32Array__New);
typed_array!(Float64Array, v8__Float64Array__New);
typed_array!(BigUint64Array, v8__BigUint64Array__New);
typed_array!(BigInt64Array, v8__BigInt64Array__New);
