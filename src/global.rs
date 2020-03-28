use std::mem::transmute;
use std::ptr::NonNull;

use crate::InIsolate;
use crate::Isolate;
use crate::IsolateHandle;
use crate::Local;
use crate::ToLocal;
use crate::Value;
use std::ffi::c_void;
use std::rc::Rc;

extern "C" {
  fn v8__Local__New(isolate: *mut Isolate, other: *mut Value) -> *mut Value;

  fn v8__Global__New(isolate: *mut Isolate, other: *mut Value) -> *mut Value;

  fn v8__Global__SetWeak(
    this: &mut *mut Value,
    parameter: NonNull<c_void>,
    callback: extern "C" fn(NonNull<c_void>, NonNull<Isolate>),
  );

  fn v8__Global__IsWeak(this: &mut *mut Value) -> bool;

  fn v8__Global__ClearWeak(this: &mut *mut Value);

  fn v8__Global__Reset__0(this: &mut *mut Value);

  fn v8__Global__Reset__2(
    this: &mut *mut Value,
    isolate: *mut Isolate,
    other: &*mut Value,
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
  weakable: Option<Rc<dyn Weakable<T>>>,
}

pub type WeakCallback<T> = extern "C" fn(NonNull<T>, NonNull<Isolate>);

/// Used to retrieve the callback and argument for weakened `Global` pointers.
pub unsafe trait Weakable<T> {
  /// Get the argument to be sent to the callback.
  /// Called upon `Global::set_weak`.
  fn get(self: Rc<Self>, global: &Global<T>) -> NonNull<c_void>;

  /// Called after a `Global` is no longer weak due to calling
  /// `Global::clear_weak`.
  fn clear(&self, global: &Global<T>);

  /// Get the callback function to be called upon the deallocation
  /// of the object pointed to by the `Global` handle.
  fn get_callback(&self, global: &Global<T>) -> WeakCallback<c_void>;
}

/// Default `Weakable` implementation for `Global`.
/// This implementation only sets the `Global`'s stored `value` and
/// `isolate_handle` to None.
/// It is equivalent to calling `Global::set_isolate(_, None) upon deallocation.
pub struct DefaultWeakable {}

unsafe impl<T> Weakable<T> for DefaultWeakable {
  fn get(self: Rc<DefaultWeakable>, global: &Global<T>) -> NonNull<c_void> {
    unsafe {
      NonNull::new_unchecked(global as *const Global<T> as *mut libc::c_void)
    }
  }

  fn clear(&self, _global: &Global<T>) {}

  fn get_callback(&self, _global: &Global<T>) -> WeakCallback<c_void> {
    global_weak_callback
  }
}

impl<T> Global<T> {
  /// Construct a Global with no storage cell.
  pub fn new() -> Self {
    Self {
      value: None,
      isolate_handle: None,
      weakable: None,
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
      isolate_handle: other_value.map(|_| IsolateHandle::new(isolate)),
      weakable: None,
    }
  }

  pub fn set_weakable(&mut self, weakable: Rc<dyn Weakable<T>>) {
    self.weakable = Some(weakable);
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
    let isolate = scope.isolate();
    self.get_isolate(isolate)
  }

  /// Construct a Local<T> from this global handle.
  pub fn get_isolate<'sc>(
    &self,
    isolate: &mut Isolate,
  ) -> Option<Local<'sc, T>> {
    self.check_isolate(isolate);
    self
      .value
      .map(|g| g.as_ptr() as *mut Value)
      .map(|g| unsafe { v8__Local__New(isolate, g) })
      .and_then(|l| unsafe { Local::from_raw(l as *mut T) })
  }

  /// If non-empty, destroy the underlying storage cell
  /// and create a new one with the contents of other if other is non empty.
  pub fn set(&mut self, scope: &mut impl InIsolate, other: impl AnyHandle<T>) {
    let isolate = scope.isolate();
    self.set_isolate(isolate, other)
  }

  /// If non-empty, destroy the underlying storage cell
  /// and create a new one with the contents of other if other is non empty.
  pub fn set_isolate(
    &mut self,
    isolate: &mut Isolate,
    other: impl AnyHandle<T>,
  ) {
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
    self.isolate_handle = other_value.map(|_| IsolateHandle::new(isolate));
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

  /// Turns this handle into a weak phantom handle without
  /// finalization callback.
  /// The handle will be reset automatically when the garbage collector detects
  /// that the object is no longer reachable.
  pub fn set_weak(&mut self) {
    if self.value.is_none() {
      return;
    }
    if self.weakable.is_none() {
      self.weakable = Some(Rc::new(DefaultWeakable {}));
    }
    if self.is_weak() {
      self.clear_weak();
    }
    let weakable = self.weakable.as_ref().unwrap();
    unsafe {
      v8__Global__SetWeak(
        &mut *(&mut self.value as *mut Option<NonNull<T>> as *mut *mut Value),
        weakable.clone().get(self),
        weakable.get_callback(self),
      )
    };
  }

  /// Returns true if the handle's reference is weak.
  pub fn is_weak(&mut self) -> bool {
    if self.value.is_none() {
      return false;
    }

    unsafe {
      v8__Global__IsWeak(
        &mut *(&mut self.value as *mut Option<NonNull<T>> as *mut *mut Value),
      )
    }
  }

  pub fn clear_weak(&mut self) {
    if self.value.is_none() || !self.is_weak() {
      return;
    }
    unsafe {
      v8__Global__ClearWeak(
        &mut *(&mut self.value as *mut Option<NonNull<T>> as *mut *mut Value),
      )
    };
    self.weakable.as_ref().unwrap().clear(self);
  }
}

extern "C" fn global_weak_callback(
  value: NonNull<c_void>,
  mut isolate: NonNull<Isolate>,
) {
  let this = unsafe {
    (&value as *const NonNull<c_void> as *mut NonNull<Global<()>>)
      .as_mut()
      .unwrap()
      .as_mut()
  };
  let isolate = unsafe { isolate.as_mut() };
  this.set_isolate(isolate, None);
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
          &mut *(addr as *mut Option<NonNull<T>> as *mut *mut Value),
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
