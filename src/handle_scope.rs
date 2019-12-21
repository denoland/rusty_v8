use std::marker::PhantomData;
use std::mem::MaybeUninit;

use crate::isolate::Isolate;

extern "C" {
  fn v8__HandleScope__CONSTRUCT(
    buf: &mut MaybeUninit<HandleScope>,
    isolate: &Isolate,
  );
  fn v8__HandleScope__DESTRUCT(this: &mut HandleScope);
  fn v8__HandleScope__GetIsolate<'sc>(
    this: &'sc HandleScope,
  ) -> &'sc mut Isolate;
}

#[repr(C)]
pub struct HandleScope<'sc>([usize; 3], PhantomData<&'sc mut ()>);

impl<'sc> HandleScope<'sc> {
  pub fn enter(
    isolate: &mut impl AsMut<Isolate>,
    mut f: impl FnMut(&mut HandleScope<'_>) -> (),
  ) {
    let isolate = isolate.as_mut();
    let mut scope: MaybeUninit<Self> = MaybeUninit::uninit();
    unsafe { v8__HandleScope__CONSTRUCT(&mut scope, isolate) };
    let scope = unsafe { &mut *(scope.as_mut_ptr()) };
    f(scope);

    unsafe { v8__HandleScope__DESTRUCT(scope) };
  }
}

impl<'sc> AsRef<HandleScope<'sc>> for HandleScope<'sc> {
  fn as_ref(&self) -> &Self {
    self
  }
}

impl<'sc> AsMut<HandleScope<'sc>> for HandleScope<'sc> {
  fn as_mut(&mut self) -> &mut Self {
    self
  }
}

impl<'sc> AsRef<Isolate> for HandleScope<'sc> {
  fn as_ref(&self) -> &Isolate {
    unsafe { v8__HandleScope__GetIsolate(self) }
  }
}

impl<'sc> AsMut<Isolate> for HandleScope<'sc> {
  fn as_mut(&mut self) -> &mut Isolate {
    unsafe { v8__HandleScope__GetIsolate(self) }
  }
}
