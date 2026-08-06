// Copyright 2019-2026 the Deno authors. All rights reserved. MIT license.

//! Support for sharing an isolate between threads, one thread at a time,
//! via the `v8::Locker` API.

use std::cell::RefCell;
use std::marker::PhantomData;
use std::ops::Deref;
use std::ops::DerefMut;
use std::ptr::NonNull;

use crate::Isolate;
use crate::isolate::RealIsolate;

unsafe extern "C" {
  fn v8__Locker__CONSTRUCT(buf: *mut RawLocker, isolate: *mut RealIsolate);
  fn v8__Locker__DESTRUCT(this: *mut RawLocker);
  fn v8__Isolate__Enter(isolate: *mut RealIsolate);
  fn v8__Isolate__Exit(isolate: *mut RealIsolate);
  fn v8__Isolate__GetCurrent() -> *mut RealIsolate;
}

thread_local! {
  /// Isolates whose `v8::Locker` is held by this thread, innermost last.
  /// Maintained by `Locker::new`/`Drop` so `thread_holds_lock` is a TLS
  /// read instead of an FFI call into `v8::Locker::IsLocked` — it sits
  /// on the hot path of every `Global` clone/drop/eq/hash for shared
  /// isolates.
  static LOCKED_ISOLATES: RefCell<Vec<*mut RealIsolate>> =
    const { RefCell::new(Vec::new()) };
}

/// Whether the current thread holds the `v8::Locker` for `isolate` (via
/// [`SharedIsolate::lock`]).
pub(crate) fn thread_holds_lock(isolate: *mut RealIsolate) -> bool {
  LOCKED_ISOLATES.with(|v| v.borrow().contains(&isolate))
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
/// - [`crate::Weak`] handles are not supported on shared isolates, and an
///   isolate with live weaks or pending finalizers cannot be shared.
/// - Snapshot-creator isolates and isolates with a cppgc heap attached
///   cannot be shared.
/// - Cloning a [`crate::Global`] belonging to a shared isolate requires
///   holding the lock on the current thread.
///
/// [`crate::Global`]s are `Send` and may be dropped on any thread at any
/// time: if the dropping thread holds the lock the handle is released
/// immediately, otherwise the release is deferred until the next
/// [`SharedIsolate::lock`] call (or isolate teardown).
///
/// # Cost
///
/// [`SharedIsolate::lock`] is not uniformly cheap: its cost scales with how
/// often the *entering thread changes*. V8's `ThreadManager` archives an
/// isolate's per-thread state when a different thread takes the lock and
/// restores it on the way back in, so a sequence of locks from one thread is
/// far cheaper than the same sequence alternating between two.
///
/// This matters for work-stealing executors, which are free to run each of
/// an isolate's turns on a different worker and so maximise the migration.
/// A measured case: an embedder serving a trivial request per lock lost
/// about 9% of its throughput moving from one worker thread to twelve, with
/// the *same* number of locks in both — the loss was migration alone. If an
/// embedder can keep consecutive locks of one isolate on one thread, or run
/// fewer workers, it is worth doing.
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
    // so no thread can touch the isolate concurrently: `Global` droppers
    // on other threads only push onto the mutex-protected deferred
    // queue, which `prepare_annex_for_dispose` drains and closes. Tear
    // down the same way `OwnedIsolate::drop` does.
    unsafe {
      let mut isolate = Isolate::from_non_null(self.cxx_isolate);
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
        !thread_holds_lock(ptr),
        "attempted to lock an isolate that is already locked by this thread"
      );
      let mut raw = Box::new(RawLocker([0; 2]));
      v8__Locker__CONSTRUCT(&mut *raw, ptr);
      v8__Isolate__Enter(ptr);
      LOCKED_ISOLATES.with(|v| v.borrow_mut().push(ptr));
      let locker = Self {
        raw,
        cxx_isolate: shared.cxx_isolate,
        _shared: PhantomData,
      };
      // Release Globals that were dropped by threads not holding the lock.
      locker
        .global_liveness()
        .as_ref()
        .maybe_drain_deferred_global_drops();
      locker
    }
  }
}

impl Drop for Locker<'_> {
  fn drop(&mut self) {
    unsafe {
      // Final drain while we still hold the lock, so cells dropped by
      // other threads during this lock don't sit in the queue (keeping
      // their JS objects alive) until the next acquisition.
      self
        .global_liveness()
        .as_ref()
        .maybe_drain_deferred_global_drops();
      assert!(
        std::ptr::eq(self.cxx_isolate.as_ptr(), v8__Isolate__GetCurrent()),
        "Locker dropped while its isolate was not the entered one; lockers \
         must be dropped in reverse order of creation"
      );
      v8__Isolate__Exit(self.cxx_isolate.as_ptr());
      v8__Locker__DESTRUCT(&mut *self.raw);
      // Position-independent removal: a false positive in
      // `thread_holds_lock` would let `Global::drop` reset a cell
      // without the lock, so the shadow must stay correct even if the
      // LIFO invariant is ever violated.
      LOCKED_ISOLATES.with(|v| {
        let mut v = v.borrow_mut();
        let idx = v
          .iter()
          .rposition(|p| *p == self.cxx_isolate.as_ptr())
          .expect("locked-isolate shadow out of sync");
        v.swap_remove(idx);
      });
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
    unsafe { Isolate::from_raw_ref_mut(&mut self.cxx_isolate) }
  }
}
