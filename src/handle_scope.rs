use std::marker::PhantomData;
use std::mem::MaybeUninit;

use crate::isolate::Isolate;
use crate::Local;
use crate::Value;

extern "C" {
  fn v8__HandleScope__CONSTRUCT(
    buf: &mut MaybeUninit<HandleScope>,
    isolate: &Isolate,
  );
  fn v8__HandleScope__DESTRUCT(this: &mut HandleScope);
  fn v8__HandleScope__GetIsolate<'sc>(
    this: &'sc HandleScope,
  ) -> &'sc mut Isolate;

  fn v8__EscapableHandleScope__CONSTRUCT(
    buf: &mut MaybeUninit<EscapableHandleScope>,
    isolate: &Isolate,
  );
  fn v8__EscapableHandleScope__DESTRUCT(this: &mut EscapableHandleScope);
  fn v8__EscapableHandleScope__Escape(
    this: &mut EscapableHandleScope,
    value: *mut Value,
  ) -> *mut Value;
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

#[repr(C)]
/// A HandleScope which first allocates a handle in the current scope
/// which will be later filled with the escape value.
pub struct EscapableHandleScope<'sc>([usize; 4], PhantomData<&'sc mut ()>);

impl<'sc> EscapableHandleScope<'sc> {
  pub fn new(isolate: &mut impl AsMut<Isolate>) -> Self {
    let isolate = isolate.as_mut();
    let mut scope: MaybeUninit<Self> = MaybeUninit::uninit();
    unsafe { v8__EscapableHandleScope__CONSTRUCT(&mut scope, isolate) };
    let scope = unsafe { scope.assume_init() };
    scope
  }

  /// Pushes the value into the previous scope and returns a handle to it.
  /// Cannot be called twice.
  pub fn escape<'parent>(
    &mut self,
    mut value: Local<'sc, Value>,
  ) -> Local<'parent, Value> {
    unsafe {
      Local::from_raw(v8__EscapableHandleScope__Escape(self, &mut *value))
        .unwrap()
    }
  }
}

impl<'sc> Drop for EscapableHandleScope<'sc> {
  fn drop(&mut self) {
    unsafe { v8__EscapableHandleScope__DESTRUCT(self) }
  }
}
