use crate::value::Value;
use std::marker::PhantomData;
use std::ops::Deref;
use std::ops::DerefMut;
use std::ptr::NonNull;

#[repr(C)]
pub struct Local<'sc, T>(NonNull<T>, PhantomData<&'sc ()>);

impl<'sc, T> Copy for Local<'sc, T> {}

impl<'sc, T> Clone for Local<'sc, T> {
  fn clone(&self) -> Self {
    Self(self.0, self.1)
  }
}

impl<'sc, T> Local<'sc, T> {
  pub unsafe fn from_raw(ptr: *mut T) -> Option<Self> {
    Some(Self(NonNull::new(ptr)?, PhantomData))
  }
}

impl<'sc, T> Deref for Local<'sc, T> {
  type Target = T;
  fn deref(&self) -> &T {
    unsafe { self.0.as_ref() }
  }
}

impl<'sc, T> DerefMut for Local<'sc, T> {
  fn deref_mut(&mut self) -> &mut T {
    unsafe { self.0.as_mut() }
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

#[test]
fn test_size_of_local() {
  use std::mem::size_of;
  assert_eq!(size_of::<Local<Value>>(), size_of::<*const Value>());
  assert_eq!(size_of::<Option<Local<Value>>>(), size_of::<*const Value>());
}
