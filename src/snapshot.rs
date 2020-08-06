use crate::isolate_create_params::raw;
use crate::scope::data::ScopeData;
use crate::support::char;
use crate::support::int;
use crate::support::intptr_t;
use crate::support::Allocated;
use crate::Context;
use crate::CreateParams;
use crate::Isolate;
use crate::Local;

use std::borrow::Borrow;
use std::convert::TryFrom;
use std::mem::MaybeUninit;
use std::ops::Deref;
use std::ops::DerefMut;

extern "C" {
  fn v8__SnapshotCreator__CONSTRUCT(
    buf: *mut MaybeUninit<SnapshotCreator>,
    external_references: *const intptr_t,
    startup_data: *const raw::StartupData,
  );
  fn v8__SnapshotCreator__DESTRUCT(this: *mut SnapshotCreator);
  fn v8__SnapshotCreator__GetIsolate(
    this: *const SnapshotCreator,
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

// TODO(piscisaureus): merge this struct with
// `isolate_create_params::raw::StartupData`.
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
  pub fn new(
    external_references: Option<impl Allocated<[intptr_t]>>,
    snapshot_blob: Option<impl Allocated<[u8]>>,
  ) -> Self {
    let mut create_params = CreateParams::default();
    if let Some(er) = external_references {
      create_params = create_params.external_references(er);
    }
    if let Some(sb) = snapshot_blob {
      create_params = create_params.snapshot_blob(sb);
    }
    let (raw_create_params, create_param_allocations) =
      create_params.finalize();

    let mut snapshot_creator = unsafe {
      let mut buf = MaybeUninit::<Self>::uninit();
      v8__SnapshotCreator__CONSTRUCT(
        &mut buf,
        raw_create_params.external_references,
        raw_create_params.snapshot_blob,
      );
      buf.assume_init()
    };

    // Initialize extra (rusty_v8 specific) Isolate associated state.
    ScopeData::new_root(&mut snapshot_creator);
    snapshot_creator.create_annex(create_param_allocations);

    snapshot_creator
  }
}

impl Drop for SnapshotCreator {
  fn drop(&mut self) {
    unsafe {
      self.drop_scope_stack_and_annex();
      v8__SnapshotCreator__DESTRUCT(self);
    }
  }
}

impl SnapshotCreator {
  /// Set the default context to be included in the snapshot blob.
  /// The snapshot will not contain the global proxy, and we expect one or a
  /// global object template to create one, to be provided upon deserialization.
  pub fn set_default_context<'s>(&mut self, context: Local<'s, Context>) {
    unsafe { v8__SnapshotCreator__SetDefaultContext(self, &*context) };
  }

  /// Creates a snapshot data blob.
  /// This must not be called from within a handle scope.
  pub fn create_blob(
    &mut self,
    function_code_handling: FunctionCodeHandling,
  ) -> Option<impl Allocated<[u8]>> {
    // Make sure that all scopes have been properly exited.
    ScopeData::get_root_mut(self);

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
}

impl Deref for SnapshotCreator {
  type Target = Isolate;
  fn deref(&self) -> &Self::Target {
    unsafe { &*v8__SnapshotCreator__GetIsolate(self) }
  }
}

impl DerefMut for SnapshotCreator {
  fn deref_mut(&mut self) -> &mut Self::Target {
    unsafe { &mut *v8__SnapshotCreator__GetIsolate(self) }
  }
}
