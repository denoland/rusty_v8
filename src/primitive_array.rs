// Copyright 2018-2019 the Deno authors. All rights reserved. MIT license.
use crate::support::int;
use crate::support::Opaque;
use crate::HandleScope;
use crate::Isolate;
use crate::Local;
use crate::Primitive;

extern "C" {
  fn v8__PrimitiveArray__Length(this: &PrimitiveArray) -> int;

  fn v8__PrimitiveArray__Set(
    this: &PrimitiveArray,
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
  fn length(&self) -> usize {
    unsafe { v8__PrimitiveArray__Length(self) as usize }
  }

  fn set(&self, index: usize, item: Local<'_, Primitive>) {
    unsafe { v8__PrimitiveArray__Set(self, index as int, &item) }
  }

  fn get<'sc>(
    &self,
    scope: &mut HandleScope<'sc>,
    index: usize,
  ) -> Local<'sc, Primitive> {
    unsafe {
      let ptr = v8__PrimitiveArray__Get(self, scope.as_mut(), index as int);
      Local::from_raw(ptr).unwrap()
    }
  }
}
