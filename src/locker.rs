// Copyright 2018-2019 the Deno authors. All rights reserved. MIT license.
use crate::InIsolate;
use crate::Isolate;
use std::mem::MaybeUninit;

extern "C" {
  fn v8__Locker__CONSTRUCT(buf: &mut MaybeUninit<Locker>, isolate: &Isolate);
  fn v8__Locker__DESTRUCT(this: &mut Locker);
}

#[repr(C)]
/// v8::Locker is a scoped lock object. While it's active, i.e. between its
/// construction and destruction, the current thread is allowed to use the locked
/// isolate. V8 guarantees that an isolate can be locked by at most one thread at
/// any time. In other words, the scope of a v8::Locker is a critical section.
pub struct Locker {
  has_lock_: bool,
  top_level: bool,
  isolate: *mut Isolate,
}

impl Locker {
  /// Initialize Locker for a given Isolate.
  pub fn new(isolate: &Isolate) -> Self {
    let mut buf = MaybeUninit::<Self>::uninit();
    unsafe {
      v8__Locker__CONSTRUCT(&mut buf, isolate);
      buf.assume_init()
    }
  }
}

impl InIsolate for Locker {
  fn isolate(&mut self) -> &mut Isolate {
    unsafe { &mut *self.isolate }
  }
}

impl Drop for Locker {
  fn drop(&mut self) {
    unsafe { v8__Locker__DESTRUCT(self) }
  }
}
