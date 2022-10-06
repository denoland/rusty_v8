use crate::external_references::ExternalReferences;
use crate::isolate_create_params::raw;
use crate::scope::data::ScopeData;
use crate::support::char;
use crate::support::int;
use crate::support::intptr_t;
use crate::support::Allocated;
use crate::support::Allocation;
use crate::Context;
use crate::Data;
use crate::Isolate;
use crate::Local;
use crate::OwnedIsolate;

use std::borrow::Borrow;
use std::convert::TryFrom;
use std::mem::MaybeUninit;
use std::ops::Deref;
use std::ptr::null;

extern "C" {
  fn v8__SnapshotCreator__CONSTRUCT(
    buf: *mut MaybeUninit<SnapshotCreatorInner>,
    external_references: *const intptr_t,
    existing_blob: *const raw::StartupData,
  );
  fn v8__SnapshotCreator__DESTRUCT(this: *mut SnapshotCreatorInner);
  fn v8__SnapshotCreator__GetIsolate(
    this: *const SnapshotCreatorInner,
  ) -> *mut Isolate;
  fn v8__SnapshotCreator__CreateBlob(
    this: *mut SnapshotCreatorInner,
    function_code_handling: FunctionCodeHandling,
  ) -> StartupData;
  fn v8__SnapshotCreator__SetDefaultContext(
    this: *mut SnapshotCreatorInner,
    context: *const Context,
  );
  fn v8__SnapshotCreator__AddContext(
    this: *mut SnapshotCreatorInner,
    context: *const Context,
  ) -> usize;
  fn v8__SnapshotCreator__AddData_to_isolate(
    this: *mut SnapshotCreatorInner,
    data: *const Data,
  ) -> usize;
  fn v8__SnapshotCreator__AddData_to_context(
    this: *mut SnapshotCreatorInner,
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

impl Drop for StartupData {
  fn drop(&mut self) {
    unsafe { v8__StartupData__DESTRUCT(self) }
  }
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

#[repr(C)]
#[derive(Debug)]
pub enum FunctionCodeHandling {
  Clear,
  Keep,
}

/// Helper class to create a snapshot data blob.
#[repr(C)]
#[derive(Debug)]
struct SnapshotCreatorInner([usize; 1]);

/// Helper class to create a snapshot data blob.
#[derive(Debug)]
pub struct SnapshotCreator {
  inner: SnapshotCreatorInner,
  isolate: Option<OwnedIsolate>,
}

impl SnapshotCreator {
  /// Create an isolate, and set it up for serialization.
  /// The isolate is created from scratch.
  pub fn new(external_references: Option<&'static ExternalReferences>) -> Self {
    Self::new_impl(external_references, None::<&[u8]>)
  }

  /// Create an isolate, and set it up for serialization.
  /// The isolate is created from scratch.
  #[inline(always)]
  pub fn from_existing_snapshot(
    existing_snapshot_blob: impl Allocated<[u8]>,
    external_references: Option<&'static ExternalReferences>,
  ) -> Self {
    Self::new_impl(external_references, Some(existing_snapshot_blob))
  }

  /// Create and enter an isolate, and set it up for serialization.
  /// The isolate is created from scratch.
  #[inline(always)]
  fn new_impl(
    external_references: Option<&'static ExternalReferences>,
    existing_snapshot_blob: Option<impl Allocated<[u8]>>,
  ) -> Self {
    let mut snapshot_creator_inner: MaybeUninit<SnapshotCreatorInner> =
      MaybeUninit::uninit();
    let external_references_ptr = if let Some(er) = external_references {
      er.as_ptr()
    } else {
      std::ptr::null()
    };

    let snapshot_blob_ptr;
    let snapshot_allocations;
    if let Some(snapshot_blob) = existing_snapshot_blob {
      let data = Allocation::of(snapshot_blob);
      let header = Allocation::of(raw::StartupData::boxed_header(&data));
      snapshot_blob_ptr = &*header as *const _;
      snapshot_allocations = Some((header, data));
    } else {
      snapshot_blob_ptr = null();
      snapshot_allocations = None;
    }

    let mut snapshot_creator_inner = unsafe {
      v8__SnapshotCreator__CONSTRUCT(
        &mut snapshot_creator_inner,
        external_references_ptr,
        snapshot_blob_ptr,
      );
      snapshot_creator_inner.assume_init()
    };

    let isolate = unsafe {
      let isolate_ptr =
        v8__SnapshotCreator__GetIsolate(snapshot_creator_inner);
      let mut owned_isolate = OwnedIsolate::new(isolate_ptr);
      ScopeData::new_root(&mut owned_isolate);
      owned_isolate.create_annex(Box::new(snapshot_allocations));
      owned_isolate
    };

    Self {
      inner: snapshot_creator_inner,
      isolate: Some(isolate),
    }
  }
}

impl Drop for SnapshotCreator {
  fn drop(&mut self) {
    // `SnapshotCreatorInner` owns the isolate and will drop it when calling
    // `v8__SnapshotCreator__DESTRUCT()`, so let's just forget it here.
    std::mem::forget(self.isolate.take().unwrap());
    unsafe { v8__SnapshotCreator__DESTRUCT(&mut self.inner) };
  }
}

impl SnapshotCreator {
  /// Set the default context to be included in the snapshot blob.
  /// The snapshot will not contain the global proxy, and we expect one or a
  /// global object template to create one, to be provided upon deserialization.
  #[inline(always)]
  pub fn set_default_context(&mut self, context: Local<Context>) {
    unsafe {
      v8__SnapshotCreator__SetDefaultContext(&mut self.inner, &*context)
    };
  }

  /// Add additional context to be included in the snapshot blob.
  /// The snapshot will include the global proxy.
  ///
  /// Returns the index of the context in the snapshot blob.
  #[inline(always)]
  pub fn add_context(&mut self, context: Local<Context>) -> usize {
    unsafe { v8__SnapshotCreator__AddContext(&mut self.inner, &*context) }
  }

  /// Attach arbitrary `v8::Data` to the isolate snapshot, which can be
  /// retrieved via `HandleScope::get_context_data_from_snapshot_once()` after
  /// deserialization. This data does not survive when a new snapshot is created
  /// from an existing snapshot.
  #[inline(always)]
  pub fn add_isolate_data<T>(&mut self, data: Local<T>) -> usize
  where
    for<'l> Local<'l, T>: Into<Local<'l, Data>>,
  {
    unsafe {
      v8__SnapshotCreator__AddData_to_isolate(&mut self.inner, &*data.into())
    }
  }

  /// Attach arbitrary `v8::Data` to the context snapshot, which can be
  /// retrieved via `HandleScope::get_context_data_from_snapshot_once()` after
  /// deserialization. This data does not survive when a new snapshot is
  /// created from an existing snapshot.
  #[inline(always)]
  pub fn add_context_data<T>(
    &mut self,
    context: Local<Context>,
    data: Local<T>,
  ) -> usize
  where
    for<'l> Local<'l, T>: Into<Local<'l, Data>>,
  {
    unsafe {
      v8__SnapshotCreator__AddData_to_context(
        &mut self.inner,
        &*context,
        &*data.into(),
      )
    }
  }

  /// Creates a snapshot data blob.
  /// This must not be called from within a handle scope.
  #[inline(always)]
  pub fn create_blob(
    &mut self,
    isolate: OwnedIsolate,
    function_code_handling: FunctionCodeHandling,
  ) -> Option<StartupData> {
    assert!(self.isolate.replace(isolate).is_none());
    {
      ScopeData::get_root_mut(self.isolate.as_deref_mut().unwrap());
    }
    let blob = unsafe {
      v8__SnapshotCreator__CreateBlob(&mut self.inner, function_code_handling)
    };
    if blob.data.is_null() {
      debug_assert!(blob.raw_size == 0);
      None
    } else {
      debug_assert!(blob.raw_size > 0);
      Some(blob)
    }
  }

  /// This method panics if called more than once.
  #[inline(always)]
  pub fn get_owned_isolate(&mut self) -> OwnedIsolate {
    assert!(self.isolate.is_some(), "Isolate has already been taken");
    self.isolate.take().unwrap()
  }
}
