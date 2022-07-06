use crate::support::Opaque;
use libc::c_void;
use std::mem::transmute_copy;
use std::ptr::NonNull;

extern "C" {
  fn v8__CTypeInfo__New(ty: CType) -> *mut CTypeInfo;
  fn v8__CTypeInfo__New__From__Slice(
    len: usize,
    tys: *const CType,
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

  pub(crate) fn new_from_slice(types: &[CType]) -> NonNull<CTypeInfo> {
    unsafe {
      NonNull::new_unchecked(v8__CTypeInfo__New__From__Slice(
        types.len(),
        types.as_ptr(),
      ))
    }
  }
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

pub trait FastFunction {
  type Signature;
  fn args(&self) -> &'static [CType] {
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
