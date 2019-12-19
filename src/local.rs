use crate::value::Value;
use std::marker::PhantomData;
use std::ops::Deref;
use std::ops::DerefMut;
use std::ptr;

pub struct Local<'sc, T>(*mut T, PhantomData<&'sc ()>);

impl<'sc, T> Copy for Local<'sc, T> {}

impl<'sc, T> Clone for Local<'sc, T> {
  fn clone(&self) -> Self {
    Self(self.0, self.1)
  }
}

impl<'sc, T> Local<'sc, T> {
  /// Creates a new empty local handle.
  pub fn empty() -> Self {
    Self(ptr::null_mut(), PhantomData)
  }

  pub unsafe fn from_raw(ptr: *mut T) -> Option<Self> {
    Some(Self(ptr, PhantomData))
  }

  /// Returns true if the handle is empty.
  pub fn is_empty(&self) -> bool {
    self.0.is_null()
  }

  /// Sets the handle to be empty. IsEmpty() will then return true.
  pub fn clear(&mut self) {
    self.0 = ptr::null_mut();
  }
}

impl<'sc, T> Deref for Local<'sc, T> {
  type Target = T;
  fn deref(&self) -> &T {
    unsafe { self.0.as_ref().unwrap() }
  }
}

impl<'sc, T> DerefMut for Local<'sc, T> {
  fn deref_mut(&mut self) -> &mut T {
    unsafe { self.0.as_mut().unwrap() }
  }
}

// TODO make it possible for targets other than Local<Value>. For example
// Local<String> should be able to be down cast to Local<Name>.
impl<'sc, T> From<Local<'sc, T>> for Local<'sc, Value>
where
  T: Deref<Target = Value>,
{
  fn from(v: Local<'sc, T>) -> Local<'sc, Value> {
    unsafe { std::mem::transmute(v) }
  }
}
