use std::marker::PhantomData;
use std::mem::size_of;
use std::mem::transmute;
use std::ops::Deref;
use std::ops::DerefMut;

pub use std::os::raw::c_int as int;

pub type Opaque = [usize; 0];

pub trait Delete
where
  Self: Sized + 'static,
{
  fn delete(&'static mut self) -> ();
}

/// Pointer to object allocated on the C++ heap.
#[repr(transparent)]
#[derive(Debug)]
pub struct UniquePtr<T>(Option<&'static mut T>)
where
  T: Delete;

impl<T> UniquePtr<T>
where
  T: Delete,
{
  pub fn null() -> Self {
    Self(None)
  }

  pub fn new(r: &'static mut T) -> Self {
    Self(Some(r))
  }

  pub unsafe fn from_raw(p: *mut T) -> Self {
    transmute(p)
  }
}

impl<T> Deref for UniquePtr<T>
where
  T: Delete,
{
  type Target = Option<&'static mut T>;
  fn deref(&self) -> &Self::Target {
    &self.0
  }
}

impl<T> DerefMut for UniquePtr<T>
where
  T: Delete,
{
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.0
  }
}

impl<T> Drop for UniquePtr<T>
where
  T: Delete,
{
  fn drop(&mut self) {
    self.0.take().map(Delete::delete);
  }
}

#[derive(Copy, Clone, Debug)]
#[repr(transparent)]
pub struct CxxVTable(pub *const Opaque);

#[derive(Copy, Clone, Debug)]
pub struct RustVTable<DynT>(pub *const Opaque, pub PhantomData<DynT>);

#[derive(Debug)]
pub struct FieldOffset<F>(usize, PhantomData<F>);

unsafe impl<F> Send for FieldOffset<F> where F: Send {}
unsafe impl<F> Sync for FieldOffset<F> where F: Sync {}

impl<F> Copy for FieldOffset<F> {}

impl<F> Clone for FieldOffset<F> {
  fn clone(&self) -> Self {
    Self(self.0, self.1)
  }
}

impl<F> FieldOffset<F> {
  pub fn from_ptrs<E>(embedder_ptr: *const E, field_ptr: *const F) -> Self {
    let embedder_addr = embedder_ptr as usize;
    let field_addr = field_ptr as usize;
    assert!(field_addr >= embedder_addr);
    assert!((field_addr + size_of::<F>()) <= (embedder_addr + size_of::<E>()));
    Self(embedder_addr - field_addr, PhantomData)
  }

  pub unsafe fn to_embedder<E>(self, field: &F) -> &E {
    (((field as *const _ as usize) - self.0) as *const E)
      .as_ref()
      .unwrap()
  }

  pub unsafe fn to_embedder_mut<E>(self, field: &mut F) -> &mut E {
    (((field as *mut _ as usize) - self.0) as *mut E)
      .as_mut()
      .unwrap()
  }
}
