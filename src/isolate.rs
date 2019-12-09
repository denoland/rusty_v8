use std::ops::Deref;
use std::ops::DerefMut;

use crate::array_buffer::Allocator;
use crate::support::Delete;
use crate::support::Opaque;
use crate::support::UniqueRef;

extern "C" {
  fn v8__Isolate__New(params: *mut CreateParams) -> &'static mut CxxIsolate;
  fn v8__Isolate__Dispose(this: &mut CxxIsolate) -> ();
  fn v8__Isolate__Enter(this: &mut CxxIsolate) -> ();
  fn v8__Isolate__Exit(this: &mut CxxIsolate) -> ();

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
  /// Creates a new isolate.  Does not change the currently entered
  /// isolate.
  ///
  /// When an isolate is no longer used its resources should be freed
  /// by calling V8::dispose().  Using the delete operator is not allowed.
  ///
  /// V8::initialize() must have run prior to this.
  pub fn new(params: UniqueRef<CreateParams>) -> Self {
    // TODO: support CreateParams.
    crate::V8::assert_initialized();
    Self(unsafe { v8__Isolate__New(params.into_raw()) })
  }

  /// Initial configuration parameters for a new Isolate.
  pub fn create_params() -> UniqueRef<CreateParams> {
    CreateParams::new()
  }

  /// Sets this isolate as the entered one for the current thread.
  /// Saves the previously entered one (if any), so that it can be
  /// restored when exiting.  Re-entering an isolate is allowed.
  pub fn enter(&mut self) {
    unsafe { v8__Isolate__Enter(self.0) }
  }

  /// Exits this isolate by restoring the previously entered one in the
  /// current thread.  The isolate may still stay the same, if it was
  /// entered more than once.
  ///
  /// Requires: self == Isolate::GetCurrent().
  pub fn exit(&mut self) {
    unsafe { v8__Isolate__Exit(self.0) }
  }
}

impl Drop for Isolate {
  fn drop(&mut self) {
    unsafe { v8__Isolate__Dispose(self.0) }
  }
}

impl Deref for Isolate {
  type Target = CxxIsolate;
  fn deref(&self) -> &Self::Target {
    self.0
  }
}

impl DerefMut for Isolate {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.0
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
