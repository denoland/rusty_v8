use std::marker::PhantomData;
use std::ops::Deref;
use std::ops::DerefMut;
use std::ptr::NonNull;

pub struct Local<'sc, T>(NonNull<T>, PhantomData<&'sc ()>);

impl<'sc, T> Copy for Local<'sc, T> {}

impl<'sc, T> Clone for Local<'sc, T> {
  fn clone(&self) -> Self {
    Self(self.0, self.1)
  }
}

impl<'sc, T> Local<'sc, T> {
  pub(crate) unsafe fn from_raw(ptr: *mut T) -> Option<Self> {
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
