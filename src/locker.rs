use std::ops::{Deref, DerefMut};

use crate::isolate::Isolate;
use crate::scope::data::ScopeData;

/// A handle to a shared isolate, allowing access to the isolate in a thread safe way.
///
/// Unlike V8 isolates, these do not currently support re-entrancy.
/// Do not create multiple lockers to the same isolate in the same thread.
#[derive(Debug)]
pub struct Locker<'a> {
  _lock: raw::Locker,
  // We maintain a mut reference to ensure we have exclusive ownership of the isolate during the lock.
  locked: &'a mut Isolate,
}

impl<'a> Locker<'a> {
  /// Claims the isolate, this should only be used from a shared isolate.
  pub(crate) fn new(isolate: &Isolate) -> Self {
    let const_isolate = isolate as *const Isolate;
    let mut_isolate = const_isolate as *mut Isolate;
    let s = unsafe {
      Self {
        _lock: raw::Locker::new(isolate),
        locked: &mut *mut_isolate,
      }
    };
    ScopeData::new_root(s.locked);
    unsafe { s.locked.enter() };
    s
  }

  /// Returns a reference to the locked isolate.
  pub fn isolate(&self) -> &Isolate {
    self.locked
  }

  /// Returns a mutable reference to the locked isolate.
  pub fn isolate_mut(&mut self) -> &mut Isolate {
    self.locked
  }

  /// Returns if the isolate is locked by the current thread.
  pub fn is_locked(isolate: &Isolate) -> bool {
    raw::Locker::is_locked(isolate)
  }
}

impl<'a> Drop for Locker<'a> {
  fn drop(&mut self) {
    // A new locker automatically enters the isolate, so be sure to exit the isolate when the locker is exited.
    unsafe { self.exit() };
    ScopeData::drop_root(self);
  }
}

impl<'a> Deref for Locker<'a> {
  type Target = Isolate;
  fn deref(&self) -> &Self::Target {
    self.isolate()
  }
}

impl<'a> DerefMut for Locker<'a> {
  fn deref_mut(&mut self) -> &mut Self::Target {
    self.isolate_mut()
  }
}

impl<'a> AsMut<Isolate> for Locker<'a> {
  fn as_mut(&mut self) -> &mut Isolate {
    self
  }
}

mod raw {
  use std::mem::MaybeUninit;

  use crate::Isolate;

  #[repr(C)]
  #[derive(Debug)]
  pub(super) struct Locker([u8; crate::binding::v8__Locker__SIZE]);

  impl Locker {
    pub fn new(isolate: &Isolate) -> Self {
      unsafe {
        let mut this = MaybeUninit::<Self>::uninit();
        v8__Locker__CONSTRUCT(this.as_mut_ptr(), isolate);
        // v8-locker.h disallows copying and assigning, but it does not disallow moving so this is hopefully safe.
        this.assume_init()
      }
    }

    pub fn is_locked(isolate: &Isolate) -> bool {
      unsafe { v8__Locker__IsLocked(isolate) }
    }
  }

  impl Drop for Locker {
    fn drop(&mut self) {
      unsafe { v8__Locker__DESTRUCT(self) }
    }
  }

  extern "C" {
    fn v8__Locker__CONSTRUCT(locker: *mut Locker, isolate: *const Isolate);
    fn v8__Locker__DESTRUCT(locker: *mut Locker);
    fn v8__Locker__IsLocked(isolate: *const Isolate) -> bool;
  }
}
