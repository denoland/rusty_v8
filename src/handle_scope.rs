use std::mem::MaybeUninit;

use crate::isolate::Isolate;
use crate::scope::Entered;
use crate::scope::Scope;
use crate::scope::Scoped;
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
pub struct HandleScope([usize; 3]);

impl HandleScope {
  pub fn new<'sc>(
    isolate: &'sc mut impl AsMut<Entered<'sc, Isolate>>,
  ) -> Scope<'sc, Self> {
    Scope::new(isolate.entered().as_mut())
  }
}

unsafe impl<'s> Scoped<'s> for HandleScope {
  type Args = &'s mut Isolate;

  fn enter_scope(buf: &mut MaybeUninit<Self>, isolate: &mut Isolate) {
    unsafe { v8__HandleScope__CONSTRUCT(buf, isolate) };
  }
}

impl Drop for HandleScope {
  fn drop(&mut self) {
    unsafe { v8__HandleScope__DESTRUCT(self) }
  }
}

impl AsRef<HandleScope> for HandleScope {
  fn as_ref(&self) -> &Self {
    self
  }
}

impl AsMut<HandleScope> for HandleScope {
  fn as_mut(&mut self) -> &mut Self {
    self
  }
}

impl AsRef<Isolate> for HandleScope {
  fn as_ref(&self) -> &Isolate {
    unsafe { v8__HandleScope__GetIsolate(self) }
  }
}

impl AsMut<Isolate> for HandleScope {
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
    isolate: &'sc mut impl AsMut<Entered<'sc, Isolate>>,
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
