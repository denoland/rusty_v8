use crate::support::Opaque;
use crate::v8::assert_initialized;
use std::ops::Deref;

extern "C" {
  fn v8__Isolate__New() -> &'static mut UnlockedIsolate;
  fn v8__Isolate__Dispose(this: &mut UnlockedIsolate) -> ();
}

#[repr(C)]
pub struct UnlockedIsolate(Opaque);
#[repr(C)]
pub struct LockedIsolate(Opaque);

#[repr(transparent)]
pub struct Isolate(&'static mut UnlockedIsolate);

impl Isolate {
  pub fn new() -> Self {
    // TODO: support CreateParams.
    assert_initialized();
    Self(unsafe { v8__Isolate__New() })
  }
}

impl Drop for Isolate {
  fn drop(&mut self) {
    unsafe { v8__Isolate__Dispose(self.0) }
  }
}

impl Deref for Isolate {
  type Target = UnlockedIsolate;
  fn deref(&self) -> &UnlockedIsolate {
    self.0
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::platform::*;
  use crate::v8::*;

  #[test]
  fn test_isolate() {
    initialize_platform(new_default_platform());
    initialize();
    //let isolate = Isolate::new();
    dispose();
    shutdown_platform();
  }
}
