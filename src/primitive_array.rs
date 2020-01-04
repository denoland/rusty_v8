// Copyright 2019-2020 the Deno authors. All rights reserved. MIT license.
use crate::support::int;
use crate::support::Opaque;
use crate::Isolate;
use crate::Local;
use crate::Primitive;
use crate::ToLocal;

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
    scope: &mut impl ToLocal<'sc>,
    length: usize,
  ) -> Local<'sc, PrimitiveArray> {
    let ptr =
      unsafe { v8__PrimitiveArray__New(scope.isolate(), length as int) };
    unsafe { scope.to_local(ptr) }.unwrap()
  }

  pub fn length(&self) -> usize {
    unsafe { v8__PrimitiveArray__Length(self) as usize }
  }

  pub fn set<'sc>(
    &self,
    scope: &mut impl ToLocal<'sc>,
    index: usize,
    item: impl Into<Local<'sc, Primitive>>,
  ) {
    unsafe {
      v8__PrimitiveArray__Set(
        self,
        scope.isolate(),
        index as int,
        &*item.into(),
      )
    }
  }

  pub fn get<'sc>(
    &self,
    scope: &mut impl ToLocal<'sc>,
    index: usize,
  ) -> Local<'sc, Primitive> {
    let ptr =
      unsafe { v8__PrimitiveArray__Get(self, scope.isolate(), index as int) };
    unsafe { scope.to_local(ptr) }.unwrap()
  }
}
