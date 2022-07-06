use crate::support::Opaque;
use libc::c_void;
use std::mem::transmute_copy;
use std::ptr::NonNull;

extern "C" {
  fn v8__CTypeInfo__New(ty: CType) -> *mut CTypeInfo;
  fn v8__CTypeInfo__New__Sequence(
    ty: CType,
    sequence_type: SequenceType,
  ) -> *mut CTypeInfo;
  fn v8__CTypeInfo__New__From__Slice(
    len: usize,
    tys: *const CTypeSequenceType,
  ) -> *mut CTypeInfo;
  fn v8__CFunction__New(
    func_ptr: *const c_void,
    return_info: *const CTypeInfo,
    args_len: usize,
    args_info: *const CTypeInfo,
  ) -> *mut CFunction;
}

#[repr(C)]
#[derive(Default)]
pub struct CFunction(Opaque);

impl CFunction {
  pub(crate) unsafe fn new(
    func_ptr: *const c_void,
    args: *const CTypeInfo,
    args_len: usize,
    return_type: *const CTypeInfo,
  ) -> NonNull<CFunction> {
    NonNull::new_unchecked(v8__CFunction__New(
      func_ptr,
      return_type,
      args_len,
      args,
    ))
  }
}

#[repr(C)]
#[derive(Debug)]
pub struct CTypeInfo(Opaque);

impl CTypeInfo {
  pub(crate) fn new(ty: CType) -> NonNull<CTypeInfo> {
    unsafe { NonNull::new_unchecked(v8__CTypeInfo__New(ty)) }
  }

  pub(crate) fn new_sequence(
    ty: CType,
    sequence_type: SequenceType,
  ) -> NonNull<CTypeInfo> {
    assert_ne!(
      sequence_type,
      SequenceType::Scalar,
      "Use CTypeInfo::new instead"
    );
    unsafe {
      NonNull::new_unchecked(v8__CTypeInfo__New__Sequence(ty, sequence_type))
    }
  }

  pub(crate) fn new_from_slice(types: &[Type]) -> NonNull<CTypeInfo> {
    let mut structs = vec![];

    for type_ in types {
      structs.push(type_.into_struct())
    }

    let ptr = unsafe {
      NonNull::new_unchecked(v8__CTypeInfo__New__From__Slice(
        structs.len(),
        structs.as_ptr(),
      ))
    };
    std::mem::forget(structs);
    ptr
  }
}

#[derive(Clone, Copy, PartialEq, Debug)]
#[repr(u8)]
pub enum SequenceType {
  Scalar,
  /// sequence<T>
  IsSequence,
  /// TypedArray of T or any ArrayBufferView if T is void
  IsTypedArray,
  /// ArrayBuffer
  IsArrayBuffer,
}

#[derive(Clone, Copy)]
#[repr(u8)]
pub enum CType {
  Void = 0,
  Bool,
  Int32,
  Uint32,
  Int64,
  Uint64,
  Float32,
  Float64,
  V8Value,
}

#[derive(Clone, Copy)]
pub enum Type {
  Void,
  Bool,
  Int32,
  Uint32,
  Int64,
  Uint64,
  Float32,
  Float64,
  V8Value,
  Sequence(CType),
  TypedArray(CType),
  ArrayBuffer(CType),
}

impl Type {
  fn into_struct(self) -> CTypeSequenceType {
    let (c_type, sequence_type) = match self {
      Self::Void => (CType::Void, SequenceType::Scalar),
      Self::Bool => (CType::Bool, SequenceType::Scalar),
      Self::Int32 => (CType::Int32, SequenceType::Scalar),
      Self::Uint32 => (CType::Uint32, SequenceType::Scalar),
      Self::Int64 => (CType::Int64, SequenceType::Scalar),
      Self::Uint64 => (CType::Uint64, SequenceType::Scalar),
      Self::Float32 => (CType::Float32, SequenceType::Scalar),
      Self::Float64 => (CType::Float64, SequenceType::Scalar),
      Self::V8Value => (CType::V8Value, SequenceType::Scalar),
      Self::Sequence(c_type) => (c_type, SequenceType::IsSequence),
      Self::TypedArray(c_type) => (c_type, SequenceType::IsTypedArray),
      Self::ArrayBuffer(c_type) => (c_type, SequenceType::IsArrayBuffer),
    };

    CTypeSequenceType {
      c_type,
      sequence_type,
    }
  }
}

#[repr(C)]
struct CTypeSequenceType {
  c_type: CType,
  sequence_type: SequenceType,
}

pub trait FastFunction {
  type Signature;
  fn args(&self) -> &'static [Type] {
    &[]
  }
  fn return_type(&self) -> CType {
    CType::Void
  }
  fn function(&self) -> Self::Signature;
  fn raw(&self) -> *const c_void {
    unsafe { transmute_copy(&self.function()) }
  }
}
