use std::marker::PhantomData;
use std::mem::MaybeUninit;
use std::ops::Deref;
use std::ops::DerefMut;

use crate::isolate::Isolate;
use crate::isolate::LockedIsolate;
use crate::isolate::UnlockedIsolate;

// class Locker {
//  public:
//    explicit Locker(Isolate* isolate);
//    ~Locker();
//    static bool IsLocked(Isolate* isolate);
//    static bool IsActive();
// }

extern "C" {
  fn v8__Locker__CONSTRUCT(
    buf: &mut MaybeUninit<Locker>,
    isolate: &UnlockedIsolate,
  );
  fn v8__Locker__DESTRUCT(this: &mut Locker);
}

#[repr(C)]
pub struct Locker<'a> {
  has_lock: bool,
  top_level: bool,
  isolate: &'a mut LockedIsolate,
  phantom: PhantomData<&'a Isolate>,
}

impl<'a> Locker<'a> {
  fn new(isolate: &UnlockedIsolate) -> Self {
    let mut buf = MaybeUninit::<Self>::uninit();
    unsafe {
      v8__Locker__CONSTRUCT(&mut buf, isolate);
      buf.assume_init()
    }
  }
}

impl<'a> Drop for Locker<'a> {
  fn drop(&mut self) {
    unsafe { v8__Locker__DESTRUCT(self) }
  }
}

impl<'a> Deref for Locker<'a> {
  type Target = LockedIsolate;
  fn deref(&self) -> &LockedIsolate {
    self.isolate
  }
}

impl<'a> DerefMut for Locker<'a> {
  fn deref_mut(&mut self) -> &mut LockedIsolate {
    self.isolate
  }
}
