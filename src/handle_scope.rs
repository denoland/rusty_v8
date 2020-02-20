// Copyright 2019-2020 the Deno authors. All rights reserved. MIT license.

use crate::isolate::Isolate;
use crate::scope::Scope;
use crate::scope::ScopeDefinition;
use crate::scope_traits::ToLocalOrReturnsLocal;
use crate::InIsolate;
use crate::Local;
use crate::Value;

extern "C" {
  fn v8__HandleScope__CONSTRUCT(buf: *mut HandleScope, isolate: *mut Isolate);
  fn v8__HandleScope__DESTRUCT(this: &mut HandleScope);
  fn v8__EscapableHandleScope__CONSTRUCT(
    buf: *mut EscapableHandleScope,
    isolate: *mut Isolate,
  );
  fn v8__EscapableHandleScope__DESTRUCT(this: &mut EscapableHandleScope);
  fn v8__EscapableHandleScope__Escape(
    this: &mut EscapableHandleScope,
    value: *mut Value,
  ) -> *mut Value;
}

/// A stack-allocated class that governs a number of local handles.
/// After a handle scope has been created, all local handles will be
/// allocated within that handle scope until either the handle scope is
/// deleted or another handle scope is created.  If there is already a
/// handle scope and a new one is created, all allocations will take
/// place in the new handle scope until it is deleted.  After that,
/// new handles will again be allocated in the original handle scope.
///
/// After the handle scope of a local handle has been deleted the
/// garbage collector will no longer track the object stored in the
/// handle and may deallocate it.  The behavior of accessing a handle
/// for which the handle scope has been deleted is undefined.
#[repr(C)]
pub struct HandleScope([usize; 3]);

impl<'s> HandleScope {
  pub fn new<P>(parent: &'s mut P) -> Scope<'s, Self, P>
  where
    P: InIsolate,
  {
    let isolate: *mut Isolate = parent.isolate();
    Scope::new(isolate, parent)
  }

  // TODO(ry) Remove this. This is a hack so we can upgrade Deno.
  pub unsafe fn new2(isolate: &Isolate) -> Scope<'s, Self> {
    Scope::new_root(isolate as *const _ as *mut Isolate)
  }
}

unsafe impl<'s> ScopeDefinition<'s> for HandleScope {
  type Args = *mut Isolate;
  unsafe fn enter_scope(buf: *mut Self, isolate: *mut Isolate) {
    v8__HandleScope__CONSTRUCT(buf, isolate);
  }
}

impl Drop for HandleScope {
  fn drop(&mut self) {
    unsafe { v8__HandleScope__DESTRUCT(self) }
  }
}

/// A HandleScope which first allocates a handle in the current scope
/// which will be later filled with the escape value.
#[repr(C)]
pub struct EscapableHandleScope([usize; 4]);

impl<'s> EscapableHandleScope {
  pub fn new<'p: 's, P>(parent: &'s mut P) -> Scope<'s, Self, P>
  where
    P: ToLocalOrReturnsLocal<'p>,
  {
    let isolate: *mut Isolate = parent.isolate();
    Scope::new(isolate, parent)
  }

  /// Pushes the value into the previous scope and returns a handle to it.
  /// Cannot be called twice.
  pub(crate) unsafe fn escape<'p, T>(
    &mut self,
    value: Local<T>,
  ) -> Local<'p, T> {
    Local::from_raw(v8__EscapableHandleScope__Escape(
      self,
      value.as_ptr() as *mut Value,
    ) as *mut T)
    .unwrap()
  }
}

unsafe impl<'s> ScopeDefinition<'s> for EscapableHandleScope {
  type Args = *mut Isolate;
  unsafe fn enter_scope(buf: *mut Self, isolate: *mut Isolate) {
    v8__EscapableHandleScope__CONSTRUCT(buf, isolate);
  }
}

impl Drop for EscapableHandleScope {
  fn drop(&mut self) {
    unsafe { v8__EscapableHandleScope__DESTRUCT(self) }
  }
}
