use std::marker::PhantomData;
use std::mem::MaybeUninit;

use crate::isolate::Isolate;
use crate::scope::{Scope, Scoped};
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
  fn v8__EscapableHandleScope__GetIsolate<'sc>(
    this: &'sc EscapableHandleScope,
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

/// A HandleScope which first allocates a handle in the current scope
/// which will be later filled with the escape value.
#[repr(C)]
pub struct EscapableHandleScope([usize; 4]);

impl EscapableHandleScope {
  pub fn new<'sc>(
    isolate: &'sc mut impl AsMut<crate::scope::Entered<'sc, Isolate>>,
  ) -> Scope<'sc, Self> {
    Scope::new(&mut **(isolate.as_mut()))
  }

  /// Pushes the value into the previous scope and returns a handle to it.
  /// Cannot be called twice.
  pub fn escape<'parent>(
    &mut self,
    value: Local<Value>,
  ) -> Local<'parent, Value> {
    unsafe {
      Local::from_raw(v8__EscapableHandleScope__Escape(self, value.as_ptr()))
        .unwrap()
    }
  }
}

unsafe impl<'s> Scoped<'s> for EscapableHandleScope {
  type Args = &'s mut Isolate;

  fn enter_scope(buf: &mut MaybeUninit<Self>, isolate: &mut Isolate) {
    unsafe { v8__EscapableHandleScope__CONSTRUCT(buf, isolate) };
  }
}

impl Drop for EscapableHandleScope {
  fn drop(&mut self) {
    unsafe { v8__EscapableHandleScope__DESTRUCT(self) }
  }
}

impl AsRef<EscapableHandleScope> for EscapableHandleScope {
  fn as_ref(&self) -> &Self {
    self
  }
}

impl AsMut<EscapableHandleScope> for EscapableHandleScope {
  fn as_mut(&mut self) -> &mut Self {
    self
  }
}

impl AsRef<Isolate> for EscapableHandleScope {
  fn as_ref(&self) -> &Isolate {
    unsafe { v8__EscapableHandleScope__GetIsolate(self) }
  }
}

impl AsMut<Isolate> for EscapableHandleScope {
  fn as_mut(&mut self) -> &mut Isolate {
    unsafe { v8__EscapableHandleScope__GetIsolate(self) }
  }
}
