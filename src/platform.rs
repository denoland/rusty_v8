use crate::support::Opaque;
use crate::support::UniquePtr;

extern "C" {
  fn v8__platform__NewDefaultPlatform() -> *mut Platform;
  fn v8__Platform__DELETE(this: *mut Platform);
}

pub fn new_default_platform() -> UniquePtr<Platform> {
  unsafe { UniquePtr::from_raw(v8__platform__NewDefaultPlatform()) }
}

#[repr(C)]
#[derive(Debug)]
pub struct Platform(Opaque);

impl Drop for Platform {
  fn drop(&mut self) {
    unsafe { v8__Platform__DELETE(self) }
  }
}
