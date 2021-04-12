use crate::support::Opaque;
use crate::support::UniquePtr;

extern "C" {
  fn v8__platform__NewDefaultPlatform() -> *mut Platform;
  fn v8__platform__NewSingleThreadedDefaultPlatform() -> *mut Platform;
  fn v8__Platform__DELETE(this: *mut Platform);
}

/// Returns a new instance of the default v8::Platform implementation.
pub fn new_default_platform() -> UniquePtr<Platform> {
  unsafe { UniquePtr::from_raw(v8__platform__NewDefaultPlatform()) }
}

/// The same as new_default_platform() but disables the worker thread pool.
/// It must be used with the --single-threaded V8 flag.
pub fn new_single_threaded_default_platform() -> UniquePtr<Platform> {
  unsafe {
    UniquePtr::from_raw(v8__platform__NewSingleThreadedDefaultPlatform())
  }
}

#[repr(C)]
#[derive(Debug)]
pub struct Platform(Opaque);

impl Drop for Platform {
  fn drop(&mut self) {
    unsafe { v8__Platform__DELETE(self) }
  }
}
