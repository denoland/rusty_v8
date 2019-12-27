use std::mem::MaybeUninit;

use crate::isolate::Isolate;
use crate::scope::Entered;
use crate::scope::Scope;
use crate::scope::Scoped;
use crate::InIsolate;
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
  pub fn new(scope: &mut impl InIsolate) -> Scope<Self> {
    Scope::new(scope.isolate())
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

/// A HandleScope which first allocates a handle in the current scope
/// which will be later filled with the escape value.
#[repr(C)]
pub struct EscapableHandleScope([usize; 4]);

impl EscapableHandleScope {
  pub fn new(scope: &mut impl InIsolate) -> Scope<Self> {
    Scope::new(scope.isolate())
  }

  /// Pushes the value into the previous scope and returns a handle to it.
  /// Cannot be called twice.
  pub fn escape<'parent, T>(&mut self, value: Local<T>) -> Local<'parent, T> {
    unsafe {
      Local::from_raw(v8__EscapableHandleScope__Escape(
        self,
        value.as_ptr() as *mut Value,
      ) as *mut T)
    }
    .unwrap()
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

impl<'s> InIsolate for Entered<'s, HandleScope> {
  fn isolate(&mut self) -> &mut Isolate {
    unsafe { v8__HandleScope__GetIsolate(self) }
  }
}

impl<'s> InIsolate for Entered<'s, EscapableHandleScope> {
  fn isolate(&mut self) -> &mut Isolate {
    unsafe { v8__EscapableHandleScope__GetIsolate(self) }
  }
}

pub trait ToLocal<'sc>: InIsolate {
  unsafe fn to_local<T>(&mut self, ptr: *mut T) -> Option<Local<'sc, T>> {
    crate::Local::<'sc, T>::from_raw(ptr)
  }
}

impl<'s> ToLocal<'s> for crate::scope::Entered<'s, HandleScope> {}
impl<'s> ToLocal<'s> for crate::scope::Entered<'s, EscapableHandleScope> {}
