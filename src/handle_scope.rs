// Copyright 2019-2020 the Deno authors. All rights reserved. MIT license.

use std::marker::PhantomData;

use crate::isolate::Isolate;
use crate::scope::Scope;
use crate::scope::ScopeDefinition;
use crate::scope_traits::ToLocalOrReturnsLocal;
use crate::InIsolate;
use crate::Local;
use crate::Value;

extern "C" {
  fn v8__HandleScope__CONSTRUCT(buf: *mut CxxHandleScope, isolate: &Isolate);
  fn v8__HandleScope__DESTRUCT(this: &mut CxxHandleScope);
  fn v8__EscapableHandleScope__CONSTRUCT(
    buf: *mut CxxEscapableHandleScope,
    isolate: &Isolate,
  );
  fn v8__EscapableHandleScope__DESTRUCT(this: &mut CxxEscapableHandleScope);
  fn v8__EscapableHandleScope__Escape(
    this: &mut CxxEscapableHandleScope,
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
pub struct HandleScope<P>(CxxHandleScope, PhantomData<P>);

#[repr(C)]
pub(crate) struct CxxHandleScope([usize; 3]);

impl<'s, P> HandleScope<P>
where
  P: InIsolate,
{
  pub fn new(parent: &'s mut P) -> Scope<'s, Self> {
    Scope::new(parent.isolate())
  }
}

impl<'s, P> HandleScope<P> {
  pub(crate) fn inner(&self) -> &CxxHandleScope {
    &self.0
  }
}

unsafe impl<'s, P> ScopeDefinition<'s> for HandleScope<P> {
  type Parent = P;
  type Args = &'s mut Isolate;
  unsafe fn enter_scope(ptr: *mut Self, isolate: &mut Isolate) {
    v8__HandleScope__CONSTRUCT(&mut (*ptr).0, isolate);
  }
}

impl<P> Drop for HandleScope<P> {
  fn drop(&mut self) {
    unsafe { v8__HandleScope__DESTRUCT(&mut self.0) }
  }
}

/// A HandleScope which first allocates a handle in the current scope
/// which will be later filled with the escape value.
pub struct EscapableHandleScope<P>(CxxEscapableHandleScope, PhantomData<P>);

#[repr(C)]
pub(crate) struct CxxEscapableHandleScope([usize; 4]);

impl<'p: 's, 's, P> EscapableHandleScope<P>
where
  P: ToLocalOrReturnsLocal<'p>,
{
  pub fn new(parent: &'s mut P) -> Scope<'s, Self> {
    Scope::new(parent.isolate())
  }
}

impl<P> EscapableHandleScope<P> {
  /// Pushes the value into the previous scope and returns a handle to it.
  /// Cannot be called twice.
  pub fn escape<'parent, T>(&mut self, value: Local<T>) -> Local<'parent, T> {
    unsafe {
      Local::from_raw(v8__EscapableHandleScope__Escape(
        &mut self.0,
        value.as_ptr() as *mut Value,
      ) as *mut T)
    }
    .unwrap()
  }

  pub(crate) fn inner(&self) -> &CxxEscapableHandleScope {
    &self.0
  }
}

unsafe impl<'s, P> ScopeDefinition<'s> for EscapableHandleScope<P> {
  type Parent = P;
  type Args = &'s mut Isolate;
  unsafe fn enter_scope(ptr: *mut Self, isolate: &mut Isolate) {
    v8__EscapableHandleScope__CONSTRUCT(&mut (*ptr).0, isolate);
  }
}

impl<P> Drop for EscapableHandleScope<P> {
  fn drop(&mut self) {
    unsafe { v8__EscapableHandleScope__DESTRUCT(&mut self.0) }
  }
}
