use std::mem::MaybeUninit;

use crate::isolate::Isolate;

extern "C" {
  fn v8__HandleScope__CONSTRUCT(
    buf: &mut MaybeUninit<HandleScope>,
    isolate: &Isolate,
  );
  fn v8__HandleScope__DESTRUCT(this: &mut HandleScope);
  fn v8__HandleScope__GetIsolate(this: &HandleScope) -> &mut Isolate;
}

#[repr(C)]
pub struct HandleScope([usize; 3]);

impl HandleScope {
  pub fn enter(isolate: &Isolate, mut f: impl FnMut(&mut HandleScope) -> ()) {
    let mut scope: MaybeUninit<Self> = MaybeUninit::uninit();
    unsafe { v8__HandleScope__CONSTRUCT(&mut scope, isolate) };
    let scope = unsafe { &mut *(scope.as_mut_ptr()) };
    f(scope);
    unsafe { v8__HandleScope__DESTRUCT(scope) };
  }

  fn get_isolate(&self) -> &Isolate {
    unsafe { v8__HandleScope__GetIsolate(self) }
  }
}
