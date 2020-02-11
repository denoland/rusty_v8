use crate::external_references::ExternalReferences;
use crate::support::int;
use crate::support::intptr_t;
use crate::Context;
use crate::Isolate;
use crate::Local;
use crate::OwnedIsolate;
use std::borrow::Borrow;
use std::marker::PhantomData;
use std::mem::MaybeUninit;
use std::ops::Deref;
use std::ops::DerefMut;

extern "C" {
  fn v8__SnapshotCreator__CONSTRUCT(
    buf: &mut MaybeUninit<SnapshotCreator>,
    external_references: *const intptr_t,
  );
  fn v8__SnapshotCreator__DESTRUCT(this: &mut SnapshotCreator);
  fn v8__SnapshotCreator__GetIsolate(this: &SnapshotCreator) -> &mut Isolate;
  fn v8__SnapshotCreator__CreateBlob(
    this: *mut SnapshotCreator,
    function_code_handling: FunctionCodeHandling,
  ) -> OwnedStartupData;
  fn v8__SnapshotCreator__SetDefaultContext(
    this: &mut SnapshotCreator,
    context: *mut Context,
  );
  fn v8__StartupData__DESTRUCT(this: &mut StartupData);
}

#[repr(C)]
pub struct StartupData<'a> {
  data: *const u8,
  raw_size: int,
  _phantom: PhantomData<&'a [u8]>,
}

impl<'a> StartupData<'a> {
  pub fn new<D>(data: &'a D) -> Self
  where
    D: Borrow<[u8]> + ?Sized,
  {
    let data = data.borrow();
    Self {
      data: data.as_ptr(),
      raw_size: data.len() as int,
      _phantom: PhantomData,
    }
  }
}

impl<'a> Deref for StartupData<'a> {
  type Target = [u8];
  fn deref(&self) -> &Self::Target {
    unsafe { std::slice::from_raw_parts(self.data, self.raw_size as usize) }
  }
}

#[repr(transparent)]
pub struct OwnedStartupData(StartupData<'static>);

impl Deref for OwnedStartupData {
  type Target = StartupData<'static>;
  fn deref(&self) -> &Self::Target {
    &self.0
  }
}

impl DerefMut for OwnedStartupData {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.0
  }
}

impl Drop for OwnedStartupData {
  fn drop(&mut self) {
    unsafe { v8__StartupData__DESTRUCT(&mut self.0) }
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
  pub fn set_default_context<'sc>(&mut self, mut context: Local<'sc, Context>) {
    unsafe { v8__SnapshotCreator__SetDefaultContext(self, &mut *context) };
  }

  /// Creates a snapshot data blob.
  /// This must not be called from within a handle scope.
  pub fn create_blob(
    &mut self,
    function_code_handling: FunctionCodeHandling,
  ) -> Option<OwnedStartupData> {
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
    crate::isolate::new_owned_isolate(isolate_ptr)
  }

  /// Returns the isolate prepared by the snapshot creator.
  pub fn get_isolate(&self) -> &mut Isolate {
    unsafe { v8__SnapshotCreator__GetIsolate(self) }
  }
}
