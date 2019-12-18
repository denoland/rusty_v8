use std::marker::PhantomData;
use std::mem::MaybeUninit;

use crate::isolate::Isolate;

// class Locker {
//  public:
//    explicit Locker(Isolate* isolate);
//    ~Locker();
//    static bool IsLocked(Isolate* isolate);
//    static bool IsActive();
// }

extern "C" {
  fn v8__Locker__CONSTRUCT(buf: &mut MaybeUninit<Locker>, isolate: &Isolate);
  fn v8__Locker__DESTRUCT(this: &mut Locker);
}

#[repr(C)]
pub struct Locker<'sc>([usize; 2], PhantomData<&'sc mut ()>);

impl<'a> Locker<'a> {
  pub fn new(isolate: &Isolate) -> Self {
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

impl<'a> LockedIsolate for Locker<'a> {
  fn cxx_isolate(&mut self) -> &mut CxxIsolate {
    self.isolate
  }
}

#[repr(transparent)]
pub struct AssumeLocked<'a>(&'a mut CxxIsolate);

impl<'a> AssumeLocked<'a> {
  pub unsafe fn new(isolate: &'a mut CxxIsolate) -> Self {
    Self(isolate)
  }
}

impl<'a> LockedIsolate for AssumeLocked<'a> {
  fn cxx_isolate(&mut self) -> &mut CxxIsolate {
    &mut self.0
  }
}
