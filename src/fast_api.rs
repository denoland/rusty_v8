use crate::support::Opaque;
use libc::c_void;
use std::mem::transmute_copy;
use std::ptr::NonNull;

extern "C" {
  fn v8__CTypeInfo__New(ty: CType) -> *mut CTypeInfo;
  fn v8__CTypeInfo__New__From__Slice(
    len: usize,
    tys: *const CTypeSequenceInfo,
  ) -> *mut CTypeInfo;
  fn v8__CFunctionInfo__New(
    return_info: *const CTypeInfo,
    args_len: usize,
    args_info: *const CTypeInfo,
  ) -> *mut CFunctionInfo;
  fn v8__CFunction__New(
    func_ptr: *const c_void,
    info: *const CFunctionInfo,
  ) -> *mut CFunction;
}

#[repr(C)]
#[derive(Default)]
pub struct CFunctionInfo(Opaque);

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
    let info = v8__CFunctionInfo__New(return_type, args_len, args);
    NonNull::new_unchecked(v8__CFunction__New(func_ptr, info))
  }
}

#[repr(C)]
#[derive(Debug)]
pub struct CTypeInfo(Opaque);

impl CTypeInfo {
  pub(crate) fn new(ty: CType) -> NonNull<CTypeInfo> {
    unsafe { NonNull::new_unchecked(v8__CTypeInfo__New(ty)) }
  }

  pub(crate) fn new_from_slice(types: &[Type]) -> NonNull<CTypeInfo> {
    let mut structs = vec![];

    for type_ in types.iter() {
      structs.push(type_.into())
    }

    unsafe {
      NonNull::new_unchecked(v8__CTypeInfo__New__From__Slice(
        structs.len(),
        structs.as_ptr(),
      ))
    }
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
}

impl From<&Type> for CType {
  fn from(ty: &Type) -> CType {
    match ty {
      Type::Void => CType::Void,
      Type::Bool => CType::Bool,
      Type::Int32 => CType::Int32,
      Type::Uint32 => CType::Uint32,
      Type::Int64 => CType::Int64,
      Type::Uint64 => CType::Uint64,
      Type::Float32 => CType::Float32,
      Type::Float64 => CType::Float64,
      Type::V8Value => CType::V8Value,
    }
  }
}

impl From<&Type> for CTypeSequenceInfo {
  fn from(ty: &Type) -> CTypeSequenceInfo {
    CTypeSequenceInfo {
      c_type: ty.into(),
      sequence_type: SequenceType::Scalar,
    }
  }
}

#[repr(C)]
struct CTypeSequenceInfo {
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
