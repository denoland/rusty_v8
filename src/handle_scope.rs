// Copyright 2019-2020 the Deno authors. All rights reserved. MIT license.

use crate::Isolate;
use crate::Local;
use crate::Value;
use std::mem::MaybeUninit;

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

  fn v8__HandleScope__GetIsolate(self_: &HandleScope) -> *mut Isolate;

/*
fn v8__Context__GetIsolate(self_: &Context) -> *mut Isolate;
fn v8__EscapableHandleScope__GetIsolate(
  self_: &EscapableHandleScope,
) -> *mut Isolate;
fn v8__FunctionCallbackInfo__GetIsolate(
  self_: &FunctionCallbackInfo,
) -> *mut Isolate;
fn v8__Message__GetIsolate(self_: &Message) -> *mut Isolate;
fn v8__Object__GetIsolate(self_: &Object) -> *mut Isolate;
fn v8__PropertyCallbackInfo__GetIsolate(
  self_: &PropertyCallbackInfo,
) -> *mut Isolate;
*/
}

#[repr(C)]
pub struct Scope(Isolate);

impl Scope {
  pub fn isolate(&mut self) -> &mut Isolate {
    self
  }

  pub unsafe fn to_local<T>(&mut self, ptr: *mut T) -> Option<Local<T>> {
    crate::Local::<T>::from_raw(ptr)
  }
}

use std::ops::Deref;
use std::ops::DerefMut;

impl Deref for Scope {
  type Target = Isolate;
  fn deref(&self) -> &Isolate {
    &self.0
  }
}

impl DerefMut for Scope {
  fn deref_mut(&mut self) -> &mut Isolate {
    &mut self.0
  }
}

impl Deref for HandleScope {
  type Target = Scope;
  fn deref(&self) -> &Scope {
    todo!()
  }
}

impl DerefMut for HandleScope {
  fn deref_mut(&mut self) -> &mut Scope {
    unsafe { &mut *(v8__HandleScope__GetIsolate(self) as *mut Scope) }
  }
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

impl HandleScope {
  pub fn new<F>(isolate: &mut Isolate, f: F)
  where
    F: FnOnce(&mut Scope),
  {
    assert_eq!(
      std::mem::size_of::<HandleScope>(),
      std::mem::size_of::<MaybeUninit<HandleScope>>()
    );
    let mut hs: MaybeUninit<HandleScope> = MaybeUninit::uninit();
    unsafe {
      v8__HandleScope__CONSTRUCT(hs.as_mut_ptr(), isolate);
    }
    let mut hs = unsafe { hs.assume_init() };
    let scope: &mut Scope = &mut hs;
    f(scope);
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

impl EscapableHandleScope {
  pub fn new<F>(_isolate: &mut Isolate, _f: F)
  where
    F: FnOnce(&mut Self),
  {
    todo!()
    /*
    EscapableHandleScope hs;
    unsafe { v8__EscapableHandleScope__CONSTRUCT(&hs, isolate); }
    f(hs);
    */
  }
}

impl Drop for EscapableHandleScope {
  fn drop(&mut self) {
    unsafe { v8__EscapableHandleScope__DESTRUCT(self) }
  }
}
