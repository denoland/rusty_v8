use std::mem::transmute;
use std::ptr::NonNull;

use crate::InIsolate;
use crate::Isolate;
use crate::Local;
use crate::ToLocal;
use crate::Value;

extern "C" {
  fn v8__Local__New(isolate: *mut Isolate, other: *mut Value) -> *mut Value;

  fn v8__Global__New(isolate: *mut Isolate, other: *mut Value) -> *mut Value;

  fn v8__Global__Reset__0(this: &mut *mut Value);

  fn v8__Global__Reset__2(
    this: &mut *mut Value,
    isolate: *mut Isolate,
    other: &*mut Value,
  );
}

#[repr(C)]
pub struct Global<T> {
  value: Option<NonNull<T>>,
  isolate: Option<NonNull<Isolate>>,
}

unsafe impl<T> Send for Global<T> {}

/// An object reference that is independent of any handle scope. Where
/// a Local handle only lives as long as the HandleScope in which it was
/// allocated, a global handle remains valid until it is explicitly
/// disposed using reset().
///
/// A global handle contains a reference to a storage cell within
/// the V8 engine which holds an object value and which is updated by
/// the garbage collector whenever the object is moved. A new storage
/// cell can be created using the constructor or Global::set and
/// existing handles can be disposed using Global::reset.
impl<T> Global<T> {
  /// Construct a Global with no storage cell.
  pub fn new() -> Self {
    Self {
      value: None,
      isolate: None,
    }
  }

  /// Construct a new Global from an existing handle. When the existing handle
  /// is non-empty, a new storage cell is created pointing to the same object,
  /// and no flags are set.
  pub fn new_from(
    scope: &mut impl InIsolate,
    other: impl AnyHandle<T>,
  ) -> Self {
    let isolate = scope.isolate();
    let other_value = other.read(isolate);
    Self {
      value: other_value
        .map(|v| unsafe { transmute(v8__Global__New(isolate, transmute(v))) }),
      isolate: other_value.map(|_| isolate.into()),
    }
  }

  /// Returns true if this Global is empty, i.e., has not been
  /// assigned an object.
  pub fn is_empty(&self) -> bool {
    self.value.is_none()
  }

  /// Construct a Local<T> from this global handle.
  pub fn get<'sc>(
    &self,
    scope: &mut impl ToLocal<'sc>,
  ) -> Option<Local<'sc, T>> {
    self.check_isolate(scope.isolate());
    match &self.value {
      None => None,
      Some(p) => unsafe { scope.to_local(p.as_ptr()) },
    }
  }

  /// If non-empty, destroy the underlying storage cell
  /// and create a new one with the contents of other if other is non empty.
  pub fn set(&mut self, scope: &mut impl InIsolate, other: impl AnyHandle<T>) {
    let isolate = scope.isolate();
    self.check_isolate(isolate);
    let other_value = other.read(isolate);
    match (&mut self.value, &other_value) {
      (None, None) => {}
      (target, None) => unsafe {
        v8__Global__Reset__0(
          &mut *(target as *mut Option<NonNull<T>> as *mut *mut Value),
        )
      },
      (target, source) => unsafe {
        v8__Global__Reset__2(
          &mut *(target as *mut Option<NonNull<T>> as *mut *mut Value),
          isolate,
          &*(source as *const Option<NonNull<T>> as *const *mut Value),
        )
      },
    }
    self.isolate = other_value.map(|_| isolate.into());
  }

  /// If non-empty, destroy the underlying storage cell
  /// IsEmpty() will return true after this call.
  pub fn reset(&mut self, scope: &mut impl InIsolate) {
    self.set(scope, None);
  }

  fn check_isolate(&self, other: &Isolate) {
    match self.value {
      None => assert_eq!(self.isolate, None),
      Some(_) => assert_eq!(self.isolate.unwrap(), other.into()),
    }
  }
}

impl<T> Default for Global<T> {
  fn default() -> Self {
    Self::new()
  }
}

impl<T> Drop for Global<T> {
  fn drop(&mut self) {
    if !self.is_empty() {
      panic!("Global handle dropped while holding a value");
    }
  }
}

pub trait AnyHandle<T> {
  fn read(self, isolate: &Isolate) -> Option<NonNull<T>>;
}

impl<'sc, T> AnyHandle<T> for Local<'sc, T> {
  fn read(self, _isolate: &Isolate) -> Option<NonNull<T>> {
    Some(self.as_non_null())
  }
}

impl<'sc, T> AnyHandle<T> for Option<Local<'sc, T>> {
  fn read(self, _isolate: &Isolate) -> Option<NonNull<T>> {
    self.map(|local| local.as_non_null())
  }
}

impl<'sc, T> AnyHandle<T> for &Global<T> {
  fn read(self, isolate: &Isolate) -> Option<NonNull<T>> {
    self.check_isolate(isolate);
    self.value
  }
}
