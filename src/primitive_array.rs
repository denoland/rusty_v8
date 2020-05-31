// Copyright 2019-2020 the Deno authors. All rights reserved. MIT license.
use crate::support::int;
use crate::Isolate;
use crate::Local;
use crate::Primitive;
use crate::PrimitiveArray;
use crate::ToLocal;

extern "C" {
  fn v8__PrimitiveArray__New(
    isolate: *mut Isolate,
    length: int,
  ) -> *const PrimitiveArray;

  fn v8__PrimitiveArray__Length(this: *const PrimitiveArray) -> int;

  fn v8__PrimitiveArray__Set(
    this: *const PrimitiveArray,
    isolate: *mut Isolate,
    index: int,
    item: *const Primitive,
  );

  fn v8__PrimitiveArray__Get(
    this: *const PrimitiveArray,
    isolate: *mut Isolate,
    index: int,
  ) -> *const Primitive;
}

impl PrimitiveArray {
  pub fn new<'sc>(
    scope: &mut impl ToLocal<'sc>,
    length: usize,
  ) -> Local<'sc, PrimitiveArray> {
    unsafe {
      scope.to_local(|scope| {
        v8__PrimitiveArray__New(scope.isolate(), length as int)
      })
    }
    .unwrap()
  }

  pub fn length(&self) -> usize {
    unsafe { v8__PrimitiveArray__Length(self) as usize }
  }

  pub fn set<'sc>(
    &self,
    scope: &mut impl ToLocal<'sc>,
    index: usize,
    item: Local<'_, Primitive>,
  ) {
    unsafe {
      v8__PrimitiveArray__Set(self, scope.isolate(), index as int, &*item)
    }
  }

  pub fn get<'sc>(
    &self,
    scope: &mut impl ToLocal<'sc>,
    index: usize,
  ) -> Local<'sc, Primitive> {
    unsafe {
      scope.to_local(|scope| {
        v8__PrimitiveArray__Get(self, scope.isolate(), index as int)
      })
    }
    .unwrap()
  }
}
