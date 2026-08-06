// Copyright 2019-2026 the Deno authors. All rights reserved. MIT license.

//! Support for sharing an isolate between threads, one thread at a time,
//! via the `v8::Locker` API.

use std::marker::PhantomData;
use std::ops::Deref;
use std::ops::DerefMut;
use std::ptr::NonNull;

use crate::Isolate;
use crate::IsolateHandle;
use crate::isolate::RealIsolate;

unsafe extern "C" {
  fn v8__Locker__CONSTRUCT(buf: *mut RawLocker, isolate: *mut RealIsolate);
  fn v8__Locker__DESTRUCT(this: *mut RawLocker);
  pub(crate) fn v8__Locker__IsLocked(isolate: *const RealIsolate) -> bool;
  fn v8__Unlocker__CONSTRUCT(buf: *mut RawUnlocker, isolate: *mut RealIsolate);
  fn v8__Unlocker__DESTRUCT(this: *mut RawUnlocker);
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

/// Raw storage for a `v8::Unlocker`. Size checked by a static_assert in
/// binding.cc. Boxed for the same reason as [`RawLocker`].
#[repr(C)]
pub(crate) struct RawUnlocker([usize; 1]);

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
/// [`crate::Global`]s may be dropped on any thread at any time: if the
/// dropping thread holds the lock the handle is released immediately,
/// otherwise the release is deferred until the next [`SharedIsolate::lock`]
/// call (or isolate teardown).
///
/// # Blocking under the lock
///
/// The lock is held for as long as its [`Locker`] guard lives, and
/// [`SharedIsolate::lock`] has no timeout and no `try_lock`. Anything that
/// blocks while holding it — a Rust callback doing I/O, a long computation,
/// JS that parks — blocks *every* other thread that wants this isolate, and
/// a thread that blocks forever wedges the isolate permanently.
///
/// Use [`Locker::unlock`] to release the lock around such work so other
/// threads can make progress in the meantime.
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

  /// A handle usable from any thread without holding the lock, for
  /// [`IsolateHandle::terminate_execution`] and
  /// [`IsolateHandle::request_interrupt`].
  ///
  /// Obtaining one through a [`Locker`] would require the very lock the
  /// runaway thread is holding, so take it from here instead.
  pub fn thread_safe_handle(&self) -> IsolateHandle {
    // SAFETY: `thread_safe_handle` only reads the annex behind a mutex; it
    // does not touch V8, so it needs neither the lock nor entry.
    unsafe { Isolate::from_raw_ref(&self.cxx_isolate) }.thread_safe_handle()
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

  /// Release the isolate's lock for the duration of `f` so other threads
  /// can lock and use the isolate, then reacquire it before returning.
  ///
  /// Wrap anything that blocks or runs long without needing the isolate —
  /// I/O in a callback, waiting on a channel, a heavy pure-Rust
  /// computation — so it doesn't hold every other thread off the isolate.
  ///
  /// The isolate must not be touched inside `f`. The `&mut self` borrow
  /// enforces that for anything reached through this guard: scopes and
  /// `&Isolate` references derived from it are already dead by the time
  /// `unlock` can be called, and handle operations that would need the
  /// lock (e.g. cloning a [`crate::Global`]) panic while it is released.
  ///
  /// Handles created before the call stay valid — `v8::Unlocker` archives
  /// this thread's isolate state and restores it on the way back in — but
  /// another thread may run JS and collect garbage in the window, so
  /// nothing observed beforehand can be assumed unchanged afterwards.
  ///
  /// # Panics
  ///
  /// Panics if another isolate has been entered on top of this one, since
  /// unlocking would then release the wrong isolate's hold on this thread.
  pub fn unlock<R>(&mut self, f: impl FnOnce() -> R) -> R {
    let ptr = self.cxx_isolate.as_ptr();
    unsafe {
      assert!(
        std::ptr::eq(ptr, v8__Isolate__GetCurrent()),
        "Locker::unlock called while another isolate was entered on top of \
         this one"
      );
      // Release what other threads queued while we held the lock; they
      // can't do it themselves, and we're about to stop being able to.
      self
        .global_liveness()
        .as_ref()
        .drain_deferred_global_drops();
    }
    let mut raw = Box::new(RawUnlocker([0; 1]));
    // The `v8::Unlocker` constructor exits the isolate and releases the
    // lock; its destructor reacquires and re-enters. It runs through a
    // guard so that an unwind out of `f` still restores both before this
    // `Locker`'s own `Drop` runs — that would otherwise exit an isolate
    // this thread neither holds nor has entered.
    unsafe { v8__Unlocker__CONSTRUCT(&mut *raw, ptr) };
    let _relock = RelockGuard(raw);
    f()
  }
}

/// Reacquires the lock and re-enters the isolate when the unlock window
/// ends, on the normal path and while unwinding alike.
struct RelockGuard(Box<RawUnlocker>);

impl Drop for RelockGuard {
  fn drop(&mut self) {
    unsafe { v8__Unlocker__DESTRUCT(&mut *self.0) };
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
        .drain_deferred_global_drops();
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
    unsafe { Isolate::from_raw_ref_mut(&mut self.cxx_isolate) }
  }
}
