use std::ops::Deref;

use crate::array_buffer::Allocator;
use crate::support::Delete;
use crate::support::Opaque;
use crate::support::UniqueRef;
use crate::V8::assert_initialized;

extern "C" {
  fn v8__Isolate__New(params: *mut CreateParams) -> &'static mut CxxIsolate;
  fn v8__Isolate__Dispose(this: &mut CxxIsolate) -> ();

  fn v8__Isolate__CreateParams__NEW() -> *mut CreateParams;
  fn v8__Isolate__CreateParams__DELETE(this: &mut CreateParams);
  fn v8__Isolate__CreateParams__SET__array_buffer_allocator(
    this: &mut CreateParams,
    value: *mut Allocator,
  );
}

#[repr(C)]
pub struct CxxIsolate(Opaque);

pub trait LockedIsolate {
  fn cxx_isolate(&mut self) -> &mut CxxIsolate;
}

#[repr(transparent)]
pub struct Isolate(&'static mut CxxIsolate);

impl Isolate {
  pub fn new(params: UniqueRef<CreateParams>) -> Self {
    // TODO: support CreateParams.
    assert_initialized();
    Self(unsafe { v8__Isolate__New(params.into_raw()) })
  }
}

impl Drop for Isolate {
  fn drop(&mut self) {
    unsafe { v8__Isolate__Dispose(self.0) }
  }
}

impl Deref for Isolate {
  type Target = CxxIsolate;
  fn deref(&self) -> &CxxIsolate {
    self.0
  }
}

#[repr(C)]
pub struct CreateParams(Opaque);

impl CreateParams {
  pub fn new() -> UniqueRef<CreateParams> {
    unsafe { UniqueRef::from_raw(v8__Isolate__CreateParams__NEW()) }
  }

  pub fn set_array_buffer_allocator(&mut self, value: UniqueRef<Allocator>) {
    unsafe {
      v8__Isolate__CreateParams__SET__array_buffer_allocator(
        self,
        value.into_raw(),
      )
    };
  }
}

impl Delete for CreateParams {
  fn delete(&'static mut self) {
    unsafe { v8__Isolate__CreateParams__DELETE(self) }
  }
}

#[cfg(disabled_test)]
mod tests {
  use super::*;
  use crate::platform::*;
  use crate::V8::*;

  #[test]
  fn test_isolate() {
    initialize_platform(new_default_platform());
    initialize();
    let mut params = CreateParams::new();
    params.set_array_buffer_allocator(Allocator::new_default_allocator());
    Isolate::new(params);
  }
}
