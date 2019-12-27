use crate::external_references::intptr_t;
use crate::external_references::ExternalReferences;
use crate::support::int;
use crate::Context;
use crate::Isolate;
use crate::Local;
use std::mem::MaybeUninit;
use std::ops::Deref;
use std::ops::DerefMut;

extern "C" {
  fn v8__SnapshotCreator__CONSTRUCT(
    buf: &mut MaybeUninit<SnapshotCreator>,
    external_references: *const intptr_t,
  );
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
  fn v8__StartupData__DESTRUCT(this: &mut StartupData);
}

#[derive(Debug)]
#[repr(C)]
pub struct StartupData {
  data: *mut u8,
  raw_size: int,
}

impl Deref for StartupData {
  type Target = [u8];
  fn deref(&self) -> &Self::Target {
    unsafe { std::slice::from_raw_parts(self.data, self.raw_size as usize) }
  }
}

impl DerefMut for StartupData {
  fn deref_mut(&mut self) -> &mut Self::Target {
    unsafe { std::slice::from_raw_parts_mut(self.data, self.raw_size as usize) }
  }
}

impl Drop for StartupData {
  fn drop(&mut self) {
    unsafe { v8__StartupData__DESTRUCT(self) }
  }
}

#[repr(C)]
pub enum FunctionCodeHandling {
  Clear,
  Keep,
}

/// Helper class to create a snapshot data blob.
#[repr(C)]
pub struct SnapshotCreator([usize; 1]);

impl SnapshotCreator {
  /// Create and enter an isolate, and set it up for serialization.
  /// The isolate is created from scratch.
  pub fn new(external_references: &ExternalReferences) -> Self {
    let mut snapshot_creator: MaybeUninit<Self> = MaybeUninit::uninit();
    unsafe {
      v8__SnapshotCreator__CONSTRUCT(
        &mut snapshot_creator,
        external_references.as_ptr(),
      );
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
  pub fn create_blob(
    &mut self,
    function_code_handling: FunctionCodeHandling,
  ) -> Option<StartupData> {
    let blob =
      unsafe { v8__SnapshotCreator__CreateBlob(self, function_code_handling) };
    if blob.data.is_null() {
      debug_assert!(blob.raw_size == 0);
      None
    } else {
      debug_assert!(blob.raw_size > 0);
      Some(blob)
    }
  }

  /// Returns the isolate prepared by the snapshot creator.
  pub fn get_isolate(&mut self) -> &Isolate {
    unsafe { v8__SnapshotCreator__GetIsolate(self) }
  }
}
