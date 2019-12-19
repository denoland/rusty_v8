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
