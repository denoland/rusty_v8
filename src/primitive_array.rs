// Copyright 2018-2019 the Deno authors. All rights reserved. MIT license.
use crate::support::int;
use crate::support::Opaque;
use crate::HandleScope;
use crate::Isolate;
use crate::Local;
use crate::Primitive;

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
    scope: &mut HandleScope<'sc>,
    length: usize,
  ) -> Local<'sc, PrimitiveArray> {
    unsafe {
      let ptr = v8__PrimitiveArray__New(scope.as_mut(), length as int);
      Local::from_raw(ptr).unwrap()
    }
  }

  pub fn length(&self) -> usize {
    unsafe { v8__PrimitiveArray__Length(self) as usize }
  }

  pub fn set<'sc>(
    &self,
    scope: &mut HandleScope<'sc>,
    index: usize,
    item: Local<'_, Primitive>,
  ) {
    unsafe {
      v8__PrimitiveArray__Set(self, scope.as_mut(), index as int, &item)
    }
  }

  pub fn get<'sc>(
    &self,
    scope: &mut impl AsMut<Isolate>,
    index: usize,
  ) -> Local<'sc, Primitive> {
    unsafe {
      let ptr = v8__PrimitiveArray__Get(self, scope.as_mut(), index as int);
      Local::from_raw(ptr).unwrap()
    }
  }
}
