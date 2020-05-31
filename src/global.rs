use std::mem::transmute;
use std::ptr::NonNull;

use crate::Data;
use crate::InIsolate;
use crate::Isolate;
use crate::IsolateHandle;
use crate::Local;
use crate::ToLocal;

extern "C" {
  fn v8__Local__New(isolate: *mut Isolate, other: *const Data) -> *const Data;

  fn v8__Global__New(isolate: *mut Isolate, other: *const Data) -> *const Data;

  fn v8__Global__Reset__0(this: *mut *const Data);

  fn v8__Global__Reset__2(
    this: *mut *const Data,
    isolate: *mut Isolate,
    other: *const *const Data,
  );
}

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
#[repr(C)]
pub struct Global<T> {
  value: Option<NonNull<T>>,
  isolate_handle: Option<IsolateHandle>,
}

impl<T> Global<T> {
  /// Construct a Global with no storage cell.
  pub fn new() -> Self {
    Self {
      value: None,
      isolate_handle: None,
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
      isolate_handle: other_value.map(|_| isolate.thread_safe_handle()),
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
    self
      .value
      .map(|g| g.as_ptr() as *const Data)
      .and_then(|g| unsafe {
        scope.to_local(|scope| v8__Local__New(scope.isolate(), g) as *const T)
      })
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
          &mut *(target as *mut Option<NonNull<T>> as *mut *const Data),
        )
      },
      (target, source) => unsafe {
        v8__Global__Reset__2(
          &mut *(target as *mut Option<NonNull<T>> as *mut *const Data),
          isolate,
          &*(source as *const Option<NonNull<T>> as *const *const Data),
        )
      },
    }
    self.isolate_handle = other_value.map(|_| isolate.thread_safe_handle());
  }

  /// If non-empty, destroy the underlying storage cell
  /// IsEmpty() will return true after this call.
  pub fn reset(&mut self, scope: &mut impl InIsolate) {
    self.set(scope, None);
  }

  fn check_isolate(&self, isolate: &mut Isolate) {
    match self.value {
      None => assert!(self.isolate_handle.is_none()),
      Some(_) => assert_eq!(
        unsafe { self.isolate_handle.as_ref().unwrap().get_isolate_ptr() },
        isolate as *mut _
      ),
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
    match &mut self.value {
      None => {
        // This global handle is empty.
        assert!(self.isolate_handle.is_none())
      }
      Some(_)
        if unsafe {
          self
            .isolate_handle
            .as_ref()
            .unwrap()
            .get_isolate_ptr()
            .is_null()
        } =>
      {
        // This global handle is associated with an Isolate that has already
        // been disposed.
      }
      addr @ Some(_) => unsafe {
        // Destroy the storage cell that contains the contents of this Global.
        v8__Global__Reset__0(
          &mut *(addr as *mut Option<NonNull<T>> as *mut *const Data),
        )
      },
    }
  }
}

pub trait AnyHandle<T> {
  fn read(self, isolate: &mut Isolate) -> Option<NonNull<T>>;
}

impl<'sc, T> AnyHandle<T> for Local<'sc, T> {
  fn read(self, _isolate: &mut Isolate) -> Option<NonNull<T>> {
    Some(self.as_non_null())
  }
}

impl<'sc, T> AnyHandle<T> for Option<Local<'sc, T>> {
  fn read(self, _isolate: &mut Isolate) -> Option<NonNull<T>> {
    self.map(|local| local.as_non_null())
  }
}

impl<'sc, T> AnyHandle<T> for &Global<T> {
  fn read(self, isolate: &mut Isolate) -> Option<NonNull<T>> {
    self.check_isolate(isolate);
    self.value
  }
}
