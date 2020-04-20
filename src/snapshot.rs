use crate::external_references::ExternalReferences;
use crate::support::char;
use crate::support::int;
use crate::support::intptr_t;
use crate::Context;
use crate::Isolate;
use crate::Local;
use crate::OwnedIsolate;

use std::borrow::Borrow;
use std::convert::TryFrom;
use std::mem::MaybeUninit;
use std::ops::Deref;

extern "C" {
  fn v8__SnapshotCreator__CONSTRUCT(
    buf: *mut MaybeUninit<SnapshotCreator>,
    external_references: *const intptr_t,
  );
  fn v8__SnapshotCreator__DESTRUCT(this: *mut SnapshotCreator);
  fn v8__SnapshotCreator__GetIsolate(
    this: *mut SnapshotCreator,
  ) -> *mut Isolate;
  fn v8__SnapshotCreator__CreateBlob(
    this: *mut SnapshotCreator,
    function_code_handling: FunctionCodeHandling,
  ) -> StartupData;
  fn v8__SnapshotCreator__SetDefaultContext(
    this: *mut SnapshotCreator,
    context: *const Context,
  );
  fn v8__StartupData__DESTRUCT(this: *mut StartupData);
}

#[repr(C)]
pub struct StartupData {
  data: *const char,
  raw_size: int,
}

impl Deref for StartupData {
  type Target = [u8];
  fn deref(&self) -> &Self::Target {
    let data = self.data as *const u8;
    let len = usize::try_from(self.raw_size).unwrap();
    unsafe { std::slice::from_raw_parts(data, len) }
  }
}

impl AsRef<[u8]> for StartupData {
  fn as_ref(&self) -> &[u8] {
    &**self
  }
}

impl Borrow<[u8]> for StartupData {
  fn borrow(&self) -> &[u8] {
    &**self
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
  pub fn new(external_references: Option<&'static ExternalReferences>) -> Self {
    let mut snapshot_creator: MaybeUninit<Self> = MaybeUninit::uninit();
    let external_references_ptr = if let Some(er) = external_references {
      er.as_ptr()
    } else {
      std::ptr::null()
    };
    unsafe {
      v8__SnapshotCreator__CONSTRUCT(
        &mut snapshot_creator,
        external_references_ptr,
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
  pub fn set_default_context<'sc>(&mut self, context: Local<'sc, Context>) {
    unsafe { v8__SnapshotCreator__SetDefaultContext(self, &*context) };
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

  /// This is marked unsafe because it should be called at most once per snapshot
  /// creator.
  // TODO Because the SnapshotCreator creates its own isolate, we need a way to
  // get an owned handle to it. This is a questionable design which ought to be
  // revisited after the libdeno integration is complete.
  pub unsafe fn get_owned_isolate(&mut self) -> OwnedIsolate {
    let isolate_ptr = v8__SnapshotCreator__GetIsolate(self);
    OwnedIsolate::new(isolate_ptr, Box::new(()))
  }
}
