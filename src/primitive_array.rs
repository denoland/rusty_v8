// Copyright 2019-2020 the Deno authors. All rights reserved. MIT license.
use crate::support::int;
use crate::support::Opaque;
use crate::Isolate;
use crate::Local;
use crate::Primitive;
use crate::Scope;

extern "C" {
  fn v8__PrimitiveArray__New(
    isolate: *mut Isolate,
    length: int,
  ) -> *mut PrimitiveArray;

  fn v8__PrimitiveArray__Length(this: &PrimitiveArray) -> int;

  fn v8__PrimitiveArray__Set(
    this: &PrimitiveArray,
    isolate: *mut Isolate,
    index: int,
    item: &Primitive,
  );

  fn v8__PrimitiveArray__Get(
    this: &PrimitiveArray,
    isolate: *mut Isolate,
    index: int,
  ) -> *mut Primitive;
}

/// An array to hold Primitive values. This is used by the embedder to pass host
/// defined options to the ScriptOptions during compilation.
///
/// This is passed back to the embedder as part of
/// HostImportModuleDynamicallyCallback for module loading.
#[repr(C)]
pub struct PrimitiveArray(Opaque);

impl PrimitiveArray {
  pub fn new<'sc>(
    scope: &'sc mut Scope,
    length: usize,
  ) -> Local<PrimitiveArray> {
    let ptr =
      unsafe { v8__PrimitiveArray__New(scope.isolate(), length as int) };
    unsafe { scope.to_local(ptr) }.unwrap()
  }

  pub fn length(&self) -> usize {
    unsafe { v8__PrimitiveArray__Length(self) as usize }
  }

  pub fn set<'sc>(
    &self,
    scope: &'sc mut Scope,
    index: usize,
    item: Local<Primitive>,
  ) {
    unsafe {
      v8__PrimitiveArray__Set(self, scope.isolate(), index as int, &item)
    }
  }

  pub fn get<'sc>(
    &self,
    scope: &'sc mut Scope,
    index: usize,
  ) -> Local<Primitive> {
    let ptr =
      unsafe { v8__PrimitiveArray__Get(self, scope.isolate(), index as int) };
    unsafe { scope.to_local(ptr) }.unwrap()
  }
}
