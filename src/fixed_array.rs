// Copyright 2019-2021 the Deno authors. All rights reserved. MIT license.
use crate::support::int;
use crate::Context;
use crate::Data;
use crate::FixedArray;
use crate::HandleScope;
use crate::Local;

extern "C" {
  fn v8__FixedArray__Length(this: *const FixedArray) -> int;

  fn v8__FixedArray__Get(
    this: *const FixedArray,
    context: *const Context,
    index: int,
  ) -> *const Data;
}

impl FixedArray {
  pub fn length(&self) -> usize {
    unsafe { v8__FixedArray__Length(self) as usize }
  }

  pub fn get<'s>(
    &self,
    scope: &mut HandleScope<'s>,
    index: usize,
  ) -> Option<Local<'s, Data>> {
    if index >= self.length() {
      return None;
    }

    unsafe {
      scope.cast_local(|sd| {
        v8__FixedArray__Get(self, &*sd.get_current_context(), index as int)
      })
    }
  }
}
