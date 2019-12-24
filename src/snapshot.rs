use crate::support::int;
use crate::Context;
use crate::Isolate;
use crate::Local;
use std::mem::MaybeUninit;

extern "C" {
  fn v8__SnapshotCreator__CONSTRUCT(buf: &mut MaybeUninit<SnapshotCreator>);
  fn v8__SnapshotCreator__DESTRUCT(this: &mut SnapshotCreator);
  fn v8__SnapshotCreator__GetIsolate(
    this: &mut SnapshotCreator,
  ) -> &mut Isolate;
  fn v8__SnapshotCreator__CreateBlob(
    this: *mut SnapshotCreator,
    function_code_handling: FunctionCodeHandling,
  ) -> StartupData;
  fn v8__SnapshotCreator__SetDefaultContext(
    this: &mut SnapshotCreator,
    context: *mut Context,
  );
}

#[derive(Debug)]
#[repr(C)]
pub struct StartupData {
  pub data: *const u8,
  pub raw_size: int,
}

#[repr(C)]
pub enum FunctionCodeHandling {
  Clear,
  Keep,
}

/// Helper class to create a snapshot data blob.
#[repr(C)]
pub struct SnapshotCreator([usize; 1]);

impl Default for SnapshotCreator {
  /// Create and enter an isolate, and set it up for serialization.
  /// The isolate is created from scratch.
  fn default() -> Self {
    let mut snapshot_creator: MaybeUninit<Self> = MaybeUninit::uninit();

    unsafe {
      v8__SnapshotCreator__CONSTRUCT(&mut snapshot_creator);
      snapshot_creator.assume_init()
    }
  }
}

impl Drop for SnapshotCreator {
  fn drop(&mut self) {
    unsafe { v8__SnapshotCreator__DESTRUCT(self) };
  }
}

impl SnapshotCreator {
  /// Set the default context to be included in the snapshot blob.
  /// The snapshot will not contain the global proxy, and we expect one or a
  /// global object template to create one, to be provided upon deserialization.
  pub fn set_default_context<'sc>(&mut self, mut context: Local<'sc, Context>) {
    unsafe { v8__SnapshotCreator__SetDefaultContext(self, &mut *context) };
  }

  /// Creates a snapshot data blob.
  /// This must not be called from within a handle scope.
  ///
  /// Returns { nullptr, 0 } on failure, and a startup snapshot on success.
  /// The caller acquires ownership of the data array in the return value.
  pub fn create_blob(
    &mut self,
    function_code_handling: FunctionCodeHandling,
  ) -> StartupData {
    unsafe { v8__SnapshotCreator__CreateBlob(self, function_code_handling) }
  }

  /// Returns the isolate prepared by the snapshot creator.
  pub fn get_isolate(&mut self) -> &Isolate {
    unsafe { v8__SnapshotCreator__GetIsolate(self) }
  }
}
