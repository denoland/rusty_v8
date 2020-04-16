pub mod task;

pub use task::{Task, TaskBase, TaskImpl};

use crate::support::Delete;
use crate::support::Opaque;
use crate::support::UniquePtr;
use crate::Isolate;

extern "C" {
  // TODO: move this to libplatform.rs?
  fn v8__platform__NewDefaultPlatform() -> *mut Platform;

  fn v8__Platform__DELETE(this: *mut Platform) -> ();
}

pub fn new_default_platform() -> UniquePtr<Platform> {
  // TODO: support optional arguments.
  unsafe { UniquePtr::from_raw(v8__platform__NewDefaultPlatform()) }
}

#[repr(C)]
pub struct Platform(Opaque);

impl Delete for Platform {
  fn delete(&mut self) {
    unsafe { v8__Platform__DELETE(self) }
  }
}

impl Platform {
  /// Pumps the message loop for the given isolate.
  ///
  /// The caller has to make sure that this is called from the right thread.
  /// Returns true if a task was executed, and false otherwise. Unless requested
  /// through the |behavior| parameter, this call does not block if no task is
  /// pending. The |platform| has to be created using |NewDefaultPlatform|.
  pub fn pump_message_loop(_platform: &Self, _isolate: &Isolate) -> bool {
    todo!()
  }
}
