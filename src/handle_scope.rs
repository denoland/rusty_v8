use std::marker::PhantomData;
use std::mem::MaybeUninit;

use crate::isolate::CxxIsolate;
use crate::isolate::LockedIsolate;

extern "C" {
  fn v8__HandleScope__CONSTRUCT(
    buf: &mut MaybeUninit<HandleScope>,
    isolate: &mut CxxIsolate,
  );
  fn v8__HandleScope__DESTRUCT(this: &mut HandleScope);
  fn v8__HandleScope__GetIsolate<'sc>(
    this: &'sc HandleScope,
  ) -> &'sc mut CxxIsolate;
}

#[repr(C)]
pub struct HandleScope<'sc>([usize; 3], PhantomData<&'sc mut ()>);

impl<'sc> HandleScope<'sc> {
  pub fn enter<P>(parent: &mut P, mut f: impl FnMut(&mut HandleScope<'_>) -> ())
  where
    P: LockedIsolate,
  {
    let mut scope: MaybeUninit<Self> = MaybeUninit::uninit();
    unsafe { v8__HandleScope__CONSTRUCT(&mut scope, parent.cxx_isolate()) };
    let scope = unsafe { &mut *(scope.as_mut_ptr()) };
    f(scope);

    unsafe { v8__HandleScope__DESTRUCT(scope) };
  }
}

impl<'sc> LockedIsolate for HandleScope<'sc> {
  fn cxx_isolate(&mut self) -> &mut CxxIsolate {
    unsafe { v8__HandleScope__GetIsolate(self) }
  }
}
