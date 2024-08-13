use std::ffi::c_void;

use crate::binding::*;
use crate::Isolate;
use crate::Local;
use crate::Value;

#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct CFunction(v8__CFunction);

impl CFunction {
  pub const fn new(address: *const c_void, type_info: &CFunctionInfo) -> Self {
    Self(v8__CFunction {
      address_: address,
      type_info_: &type_info.0,
    })
  }

  pub const fn address(&self) -> *const c_void {
    self.0.address_
  }

  pub const fn type_info(&self) -> &CFunctionInfo {
    // SAFETY: We initialize this field with a reference. and
    // the layout of CFunctionInfo is identical to v8_CFunctionInfo.
    unsafe { &*(self.0.type_info_ as *const CFunctionInfo) }
  }
}

#[repr(transparent)]
pub struct CFunctionInfo(v8__CFunctionInfo);

impl CFunctionInfo {
  /// Construct a struct to hold a CFunction's type information.
  /// |return_info| describes the function's return type.
  /// |arg_info| is an array of |arg_count| CTypeInfos describing the
  ///   arguments. Only the last argument may be of the special type
  ///   CTypeInfo::kCallbackOptionsType.
  pub const fn new(
    return_info: CTypeInfo,
    arg_info: &[CTypeInfo],
    repr: Int64Representation,
  ) -> Self {
    Self(v8__CFunctionInfo {
      arg_count_: arg_info.len() as _,
      arg_info_: arg_info.as_ptr() as _,
      repr_: repr as _,
      return_info_: return_info.0,
    })
  }
}

#[derive(Clone, Copy)]
#[repr(u8)]
pub enum Int64Representation {
  /// Use numbers to represent 64 bit integers.
  Number = v8_CFunctionInfo_Int64Representation_kNumber,
  /// Use BigInts to represent 64 bit integers.
  BigInt = v8_CFunctionInfo_Int64Representation_kBigInt,
}

#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct CTypeInfo(v8_CTypeInfo);

impl CTypeInfo {
  pub const fn new(
    r#type: Type,
    sequence_type: SequenceType,
    flags: Flags,
  ) -> Self {
    Self(v8_CTypeInfo {
      flags_: flags.bits(),
      sequence_type_: sequence_type as _,
      type_: r#type as _,
    })
  }
}

#[derive(Clone, Copy)]
#[repr(u8)]
pub enum Type {
  Void = v8_CTypeInfo_Type_kVoid,
  Bool = v8_CTypeInfo_Type_kBool,
  Uint8 = v8_CTypeInfo_Type_kUint8,
  Int32 = v8_CTypeInfo_Type_kInt32,
  Uint32 = v8_CTypeInfo_Type_kUint32,
  Int64 = v8_CTypeInfo_Type_kInt64,
  Uint64 = v8_CTypeInfo_Type_kUint64,
  Float32 = v8_CTypeInfo_Type_kFloat32,
  Float64 = v8_CTypeInfo_Type_kFloat64,
  Pointer = v8_CTypeInfo_Type_kPointer,
  V8Value = v8_CTypeInfo_Type_kV8Value,
  SeqOneByteString = v8_CTypeInfo_Type_kSeqOneByteString,
  ApiObject = v8_CTypeInfo_Type_kApiObject,
  Any = v8_CTypeInfo_Type_kAny,
  CallbackOptions = 255,
}

impl Type {
  pub const fn scalar(self) -> CTypeInfo {
    CTypeInfo::new(self, SequenceType::Scalar, Flags::empty())
  }

  pub const fn typed_array(self) -> CTypeInfo {
    CTypeInfo::new(self, SequenceType::IsTypedArray, Flags::empty())
  }
}

#[derive(Clone, Copy)]
#[repr(u8)]
pub enum SequenceType {
  Scalar = v8_CTypeInfo_SequenceType_kScalar,
  /// sequence<T>
  IsSequence = v8_CTypeInfo_SequenceType_kIsSequence,
  /// TypedArray of T or any ArrayBufferView if T is void
  IsTypedArray = v8_CTypeInfo_SequenceType_kIsTypedArray,
  /// ArrayBuffer
  IsArrayBuffer = v8_CTypeInfo_SequenceType_kIsArrayBuffer,
}

bitflags::bitflags! {
  pub struct Flags: u8 {
    /// Must be an ArrayBuffer or TypedArray
    const AllowShared = v8_CTypeInfo_Flags_kAllowSharedBit;
    /// T must be integral
    const EnforceRange = v8_CTypeInfo_Flags_kEnforceRangeBit;
    /// T must be integral
    const Clamp = v8_CTypeInfo_Flags_kClampBit;
    /// T must be float or double
    const IsRestricted = v8_CTypeInfo_Flags_kIsRestrictedBit;
  }
}

/// A struct which may be passed to a fast call callback, like so
/// ```c
/// void FastMethodWithOptions(int param, FastApiCallbackOptions& options);
/// ```
#[repr(C)]
pub struct FastApiCallbackOptions<'a> {
  pub isolate: *mut Isolate,
  /// If the callback wants to signal an error condition or to perform an
  /// allocation, it must set options.fallback to true and do an early return
  /// from the fast method. Then V8 checks the value of options.fallback and if
  /// it's true, falls back to executing the SlowCallback, which is capable of
  /// reporting the error (either by throwing a JS exception or logging to the
  /// console) or doing the allocation. It's the embedder's responsibility to
  /// ensure that the fast callback is idempotent up to the point where error and
  /// fallback conditions are checked, because otherwise executing the slow
  /// callback might produce visible side-effects twice.
  pub fallback: bool,
  /// The `data` passed to the FunctionTemplate constructor, or `undefined`.
  pub data: Local<'a, Value>,
  /// When called from WebAssembly, a view of the calling module's memory.
  pub wasm_memory: *const FastApiTypedArray<u8>,
}

// https://source.chromium.org/chromium/chromium/src/+/main:v8/include/v8-fast-api-calls.h;l=336
#[repr(C)]
pub struct FastApiTypedArray<T: Default> {
  /// Returns the length in number of elements.
  pub length: usize,
  // This pointer should include the typed array offset applied.
  // It's not guaranteed that it's aligned to sizeof(T), it's only
  // guaranteed that it's 4-byte aligned, so for 8-byte types we need to
  // provide a special implementation for reading from it, which hides
  // the possibly unaligned read in the `get` method.
  data: *mut T,
}

impl<T: Default> FastApiTypedArray<T> {
  /// Performs an unaligned-safe read of T from the underlying data.
  #[inline(always)]
  pub const fn get(&self, index: usize) -> T {
    debug_assert!(index < self.length);
    // SAFETY: src is valid for reads, and is a valid value for T
    unsafe { std::ptr::read_unaligned(self.data.add(index)) }
  }

  /// Returns a slice pointing to the underlying data if safe to do so.
  #[inline(always)]
  pub fn get_storage_if_aligned(&self) -> Option<&mut [T]> {
    // V8 may provide an invalid or null pointer when length is zero, so we just
    // ignore that value completely and create an empty slice in this case.
    if self.length == 0 {
      return Some(&mut []);
    }
    // Ensure that we never return an unaligned or null buffer
    if self.data.is_null() || (self.data as usize) % align_of::<T>() != 0 {
      return None;
    }
    Some(unsafe { std::slice::from_raw_parts_mut(self.data, self.length) })
  }
}

/// Any TypedArray. It uses kTypedArrayBit with base type void
/// Overloaded args of ArrayBufferView and TypedArray are not supported
/// (for now) because the generic “any” ArrayBufferView doesn’t have its
/// own instance type. It could be supported if we specify that
/// TypedArray<T> always has precedence over the generic ArrayBufferView,
/// but this complicates overload resolution.
#[repr(C)]
pub struct FastApiArrayBufferView {
  pub data: *mut c_void,
  pub byte_length: usize,
}

// FastApiOneByteString is an alias for SeqOneByteString and the type is widely used in deno_core.
#[allow(unused)]
#[repr(C)]
pub struct FastApiOneByteString {
  data: *const u8,
  pub length: u32,
}

impl FastApiOneByteString {
  #[inline(always)]
  pub fn as_bytes(&self) -> &[u8] {
    // Ensure that we never create a null-ptr slice (even a zero-length null-ptr slice
    // is invalid because of Rust's niche packing).
    if self.data.is_null() {
      return &mut [];
    }

    // SAFETY: The data is guaranteed to be valid for the length of the string.
    unsafe { std::slice::from_raw_parts(self.data, self.length as usize) }
  }
}
