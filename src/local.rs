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

pub struct MaybeLocal<'sc, T>(*mut T, PhantomData<&'sc ()>);

impl<'sc, T> Copy for MaybeLocal<'sc, T> {}

impl<'sc, T> Clone for MaybeLocal<'sc, T> {
  fn clone(&self) -> Self {
    Self(self.0, self.1)
  }
}

/// A MaybeLocal<> is a wrapper around Local<> that enforces a check whether
/// the Local<> is empty before it can be used.
///
/// If an API method returns a MaybeLocal<>, the API method can potentially fail
/// either because an exception is thrown, or because an exception is pending,
/// e.g. because a previous API call threw an exception that hasn't been caught
/// yet, or because a TerminateExecution exception was thrown. In that case, an
/// empty MaybeLocal is returned.
impl<'sc, T> MaybeLocal<'sc, T> {
  pub(crate) unsafe fn from_raw(ptr: *mut T) -> Self {
    Self(ptr, PhantomData)
  }

  pub fn is_empty(&self) -> bool {
    self.0.is_null()
  }

  /// Converts this MaybeLocal<T> to a Local<T>. If this MaybeLocal<> is empty,
  /// returns None.
  pub fn to_local(&self) -> Option<Local<'sc, T>> {
    unsafe { Local::from_raw(self.0) }
  }

  /// Converts this MaybeLocal<> to a Local<>. If this MaybeLocal<> is empty,
  /// V8 will crash the process.
  pub fn to_local_checked(&self) -> Local<'sc, T> {
    self.to_local().unwrap()
  }

  /// Converts this MaybeLocal<> to a Local<>. If this MaybeLocal<> is empty,
  /// V8 will crash the process.
  pub fn from_maybe(&self, default_value: Local<'sc, T>) -> Local<'sc, T> {
    self.to_local().unwrap_or(default_value)
  }
}
