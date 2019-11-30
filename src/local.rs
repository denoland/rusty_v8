use std::ops::Deref;

pub struct Local<'sc, T>(&'sc T);

impl<'sc, T> Local<'sc, T> {
  pub unsafe fn from_raw(ptr: *const T) -> Option<Self> {
    if ptr.is_null() {
      None
    } else {
      Some(Self(&*ptr))
    }
  }
}

impl<'sc, T> Deref for Local<'sc, T> {
  type Target = T;
  fn deref(&self) -> &T {
    &self.0
  }
}
