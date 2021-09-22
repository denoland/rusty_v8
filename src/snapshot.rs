use crate::external_references::ExternalReferences;
use crate::isolate::Locker;
use crate::scope::data::ScopeData;
use crate::support::char;
use crate::support::int;
use crate::support::intptr_t;
use crate::Context;
use crate::Data;
use crate::Isolate;
use crate::Local;

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
  fn v8__SnapshotCreator__AddData_to_isolate(
    this: *mut SnapshotCreator,
    data: *const Data,
  ) -> usize;
  fn v8__SnapshotCreator__AddData_to_context(
    this: *mut SnapshotCreator,
    context: *const Context,
    data: *const Data,
  ) -> usize;
  fn v8__StartupData__DESTRUCT(this: *mut StartupData);
}

// TODO(piscisaureus): merge this struct with
// `isolate_create_params::raw::StartupData`.
#[repr(C)]
#[derive(Debug)]
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
#[derive(Debug)]
pub enum FunctionCodeHandling {
  Clear,
  Keep,
}

/// Helper class to create a snapshot data blob.
#[repr(C)]
#[derive(Debug)]
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
    let snapshot_creator = unsafe {
      v8__SnapshotCreator__CONSTRUCT(
        &mut snapshot_creator,
        external_references_ptr,
      );
      snapshot_creator.assume_init()
    };
    {
      let isolate = unsafe {
        v8__SnapshotCreator__GetIsolate(&snapshot_creator)
          .as_mut()
          .unwrap()
      };
      ScopeData::new_root(isolate);
      isolate.create_annex(Box::new(()), true, false);
    }
    snapshot_creator
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
  pub fn set_default_context(&mut self, context: Local<Context>) {
    unsafe { v8__SnapshotCreator__SetDefaultContext(self, &*context) };
  }

  /// Attach arbitrary `v8::Data` to the isolate snapshot, which can be
  /// retrieved via `HandleScope::get_context_data_from_snapshot_once()` after
  /// deserialization. This data does not survive when a new snapshot is created
  /// from an existing snapshot.
  pub fn add_isolate_data<T>(&mut self, data: Local<T>) -> usize
  where
    for<'l> Local<'l, T>: Into<Local<'l, Data>>,
  {
    unsafe { v8__SnapshotCreator__AddData_to_isolate(self, &*data.into()) }
  }

  /// Attach arbitrary `v8::Data` to the context snapshot, which can be
  /// retrieved via `HandleScope::get_context_data_from_snapshot_once()` after
  /// deserialization. This data does not survive when a new snapshot is
  /// created from an existing snapshot.
  pub fn add_context_data<T>(
    &mut self,
    context: Local<Context>,
    data: Local<T>,
  ) -> usize
  where
    for<'l> Local<'l, T>: Into<Local<'l, Data>>,
  {
    unsafe {
      v8__SnapshotCreator__AddData_to_context(self, &*context, &*data.into())
    }
  }

  pub unsafe fn get_isolate_ptr(&self) -> *mut Isolate {
    v8__SnapshotCreator__GetIsolate(self)
  }

  /// Borrows the Isolate from the SnapshotCreator.
  pub fn get_isolate(&mut self) -> Locker {
    unsafe { Locker::new(self.get_isolate_ptr().as_mut().unwrap()) }
  }

  /// Borrows the Isolate from the SnapshotCreator, also returning a reference
  /// to the SnapshotCreator.
  pub fn get_isolate_and_handle(&mut self) -> (Locker, &mut SnapshotCreator) {
    unsafe { (Locker::new(self.get_isolate_ptr().as_mut().unwrap()), self) }
  }

  /// Creates a snapshot data blob.
  /// This must not be called from within a handle scope.
  pub fn into_blob(
    mut self,
    function_code_handling: FunctionCodeHandling,
  ) -> Option<StartupData> {
    self.create_blob(function_code_handling)
  }

  /// Creates a snapshot data blob.
  /// This must not be called from within a handle scope.
  pub fn create_blob(
    &mut self,
    function_code_handling: FunctionCodeHandling,
  ) -> Option<StartupData> {
    {
      let isolate = unsafe { &mut *v8__SnapshotCreator__GetIsolate(self) };
      ScopeData::get_root_mut(isolate);
    }
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
