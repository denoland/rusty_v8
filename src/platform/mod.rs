pub mod task;

pub use task::Task;

use crate::support::Delete;
use crate::support::Opaque;
use crate::support::UniquePtr;

extern "C" {
  // TODO: move this to libplatform.rs?
  fn v8__platform__NewDefaultPlatform() -> *mut Platform;

  fn v8__Platform__DELETE(this: &'static mut Platform) -> ();
}

pub fn new_default_platform() -> UniquePtr<Platform> {
  // TODO: support optional arguments.
  unsafe { UniquePtr::from_raw(v8__platform__NewDefaultPlatform()) }
}

#[repr(C)]
pub struct Platform(Opaque);

impl Delete for Platform {
  fn delete(&'static mut self) {
    unsafe { v8__Platform__DELETE(self) }
  }
}
