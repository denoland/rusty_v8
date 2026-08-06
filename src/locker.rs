// Copyright 2019-2026 the Deno authors. All rights reserved. MIT license.

//! Support for sharing an isolate between threads, one thread at a time,
//! via the `v8::Locker` API.

use std::marker::PhantomData;
use std::ops::Deref;
use std::ops::DerefMut;
use std::ptr::NonNull;

use crate::Isolate;
use crate::isolate::RealIsolate;

unsafe extern "C" {
  fn v8__Locker__CONSTRUCT(buf: *mut RawLocker, isolate: *mut RealIsolate);
  fn v8__Locker__DESTRUCT(this: *mut RawLocker);
  pub(crate) fn v8__Locker__IsLocked(isolate: *const RealIsolate) -> bool;
  fn v8__Isolate__Enter(isolate: *mut RealIsolate);
  fn v8__Isolate__Exit(isolate: *mut RealIsolate);
  fn v8__Isolate__GetCurrent() -> *mut RealIsolate;
}

/// Raw storage for a `v8::Locker`. Its size is checked by a static_assert
/// in binding.cc. It is not address-sensitive (it holds two flags and an
/// isolate pointer), but it lives in a `Box` anyway so its address stays
/// stable for the C++ destructor no matter how the Rust guard moves.
#[repr(C)]
pub(crate) struct RawLocker([usize; 2]);

/// An isolate that can be used from multiple threads, one at a time.
///
/// Created with [`crate::OwnedIsolate::into_shared`]. All access goes
/// through [`SharedIsolate::lock`], which acquires the isolate's
/// `v8::Locker`, enters the isolate on the current thread, and yields a
/// [`Locker`] guard that dereferences to [`Isolate`].
///
/// Limitations (enforced by panics):
/// - [`crate::Weak`] handles are not supported on shared isolates.
/// - Snapshot-creator isolates cannot be shared.
/// - Cloning a [`crate::Global`] belonging to a shared isolate requires
///   holding the lock on the current thread.
///
/// cppgc heaps attached to shared isolates are unsupported and unsound.
///
/// [`crate::Global`]s may be dropped on any thread at any time: if the
/// dropping thread holds the lock the handle is released immediately,
/// otherwise the release is deferred until the next [`SharedIsolate::lock`]
/// call (or isolate teardown).
#[derive(Debug)]
pub struct SharedIsolate {
  cxx_isolate: NonNull<RealIsolate>,
}

unsafe impl Send for SharedIsolate {}
unsafe impl Sync for SharedIsolate {}

impl SharedIsolate {
  pub(crate) fn new(cxx_isolate: NonNull<RealIsolate>) -> Self {
    Self { cxx_isolate }
  }

  pub(crate) fn as_real_ptr(&self) -> *mut RealIsolate {
    self.cxx_isolate.as_ptr()
  }

  /// Acquire the isolate's lock and enter it on the current thread,
  /// blocking until any other thread holding the lock releases it.
  ///
  /// # Panics
  ///
  /// Panics if the current thread already holds the lock. `v8::Locker` is
  /// recursive, but two live guards would hand out aliasing `&mut Isolate`
  /// references, so recursive locking is forbidden here.
  pub fn lock(&self) -> Locker<'_> {
    Locker::new(self)
  }
}

impl Drop for SharedIsolate {
  fn drop(&mut self) {
    // Ownership guarantees no outstanding `Locker` (they borrow `self`),
    // but other threads may still be dropping `Global`s concurrently:
    // drain the deferred queue and close it under the lock, then tear
    // down the same way `OwnedIsolate::drop` does.
    unsafe {
      let mut isolate = Isolate::from_non_null(self.cxx_isolate);
      let ptr = self.cxx_isolate.as_ptr();
      let mut raw = Box::new(RawLocker([0; 2]));
      v8__Locker__CONSTRUCT(&mut *raw, ptr);
      isolate
        .global_liveness()
        .as_ref()
        .close_deferred_global_drops();
      v8__Locker__DESTRUCT(&mut *raw);
      let (annex_ptr, _create_param_allocations) =
        isolate.prepare_annex_for_dispose();
      Isolate::run_remaining_guaranteed_finalizers(annex_ptr);
      crate::Platform::notify_isolate_shutdown(
        &crate::V8::get_current_platform(),
        &isolate,
      );
      isolate.dispose();
      Isolate::finish_annex_dispose(annex_ptr);
    }
  }
}

/// A guard that holds the `v8::Locker` for a [`SharedIsolate`] and keeps
/// the isolate entered on the current thread. Dereferences to [`Isolate`];
/// construct scopes with e.g. `HandleScope::new(&mut *locker)`.
pub struct Locker<'s> {
  raw: Box<RawLocker>,
  cxx_isolate: NonNull<RealIsolate>,
  _shared: PhantomData<&'s SharedIsolate>,
}

impl<'s> Locker<'s> {
  fn new(shared: &'s SharedIsolate) -> Self {
    let ptr = shared.as_real_ptr();
    unsafe {
      assert!(
        !v8__Locker__IsLocked(ptr),
        "attempted to lock an isolate that is already locked by this thread"
      );
      let mut raw = Box::new(RawLocker([0; 2]));
      v8__Locker__CONSTRUCT(&mut *raw, ptr);
      v8__Isolate__Enter(ptr);
      let locker = Self {
        raw,
        cxx_isolate: shared.cxx_isolate,
        _shared: PhantomData,
      };
      // Release Globals that were dropped by threads not holding the lock.
      locker
        .global_liveness()
        .as_ref()
        .drain_deferred_global_drops();
      locker
    }
  }
}

impl Drop for Locker<'_> {
  fn drop(&mut self) {
    unsafe {
      assert!(
        std::ptr::eq(self.cxx_isolate.as_ptr(), v8__Isolate__GetCurrent()),
        "Locker dropped while its isolate was not the entered one; lockers \
         must be dropped in reverse order of creation"
      );
      v8__Isolate__Exit(self.cxx_isolate.as_ptr());
      v8__Locker__DESTRUCT(&mut *self.raw);
    }
  }
}

impl Deref for Locker<'_> {
  type Target = Isolate;
  fn deref(&self) -> &Isolate {
    unsafe { Isolate::from_raw_ref(&self.cxx_isolate) }
  }
}

impl DerefMut for Locker<'_> {
  fn deref_mut(&mut self) -> &mut Isolate {
    unsafe {
      std::mem::transmute::<&mut NonNull<RealIsolate>, &mut Isolate>(
        &mut self.cxx_isolate,
      )
    }
  }
}
