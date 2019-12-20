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
pub struct Locker([usize; 2]);

impl Locker {
  pub fn new(isolate: &Isolate) -> Self {
    let mut buf = MaybeUninit::<Self>::uninit();
    unsafe {
      v8__Locker__CONSTRUCT(&mut buf, isolate);
      buf.assume_init()
    }
  }
}

impl Drop for Locker {
  fn drop(&mut self) {
    unsafe { v8__Locker__DESTRUCT(self) }
  }
}
