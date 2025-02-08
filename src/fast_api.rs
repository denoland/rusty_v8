use crate::binding::*;
use crate::Isolate;
use crate::Local;
use crate::Value;
use std::ffi::c_void;
use std::marker::PhantomData;

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
    flags: Flags,
  ) -> Self {
    Self(v8_CTypeInfo {
      flags_: flags.bits(),
      sequence_type_: v8_CTypeInfo_SequenceType_kScalar,
      type_: r#type as _,
    })
  }

  #[deprecated]
  pub const fn new_deprecated(
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
  // const fn since From<T> is not const
  pub const fn as_info(self) -> CTypeInfo {
    CTypeInfo::new(self, Flags::empty())
  }

  #[deprecated]
  pub const fn scalar(self) -> CTypeInfo {
    CTypeInfo::new(self, Flags::empty())
  }

  #[deprecated]
  pub const fn typed_array(self) -> CTypeInfo {
    CTypeInfo::new_deprecated(self, SequenceType::IsTypedArray, Flags::empty())
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
  /// The `data` passed to the FunctionTemplate constructor, or `undefined`.
  pub data: Local<'a, Value>,
}
