use std::marker::PhantomData;
use std::mem::size_of;
use std::mem::transmute;
use std::mem::MaybeUninit;
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

  pub fn into_raw(self) -> *mut T {
    unsafe { transmute(self) }
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
    if let Some(v) = self.0.take() {
      Delete::delete(v)
    }
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
    Self(field_addr - embedder_addr, PhantomData)
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

pub(crate) trait ConstructOnStack
where
  Self: Sized,
{
  type Args;

  // The `buf` parameter represents a pointer to uninitialized memory.
  fn construct(buf: &mut MaybeUninit<Self>, args: &Self::Args);
  fn destruct(buf: &mut Self);
}

pub(crate) struct StackOnly<T>(MaybeUninit<T>)
where
  T: ConstructOnStack;

impl<T> StackOnly<T>
where
  T: ConstructOnStack,
{
  unsafe fn uninit() -> Self {
    Self(MaybeUninit::<T>::uninit())
  }

  unsafe fn init(&mut self, args: &T::Args) {
    T::construct(&mut self.0, args);
  }
}

impl<T> Deref for StackOnly<T>
where
  T: ConstructOnStack,
{
  type Target = T;
  fn deref(&self) -> &T {
    unsafe { &*(self as *const StackOnly<T> as *const T) }
  }
}

impl<T> DerefMut for StackOnly<T>
where
  T: ConstructOnStack,
{
  fn deref_mut(&mut self) -> &mut T {
    unsafe { &mut *(self as *mut StackOnly<T> as *mut T) }
  }
}

impl<T> Drop for StackOnly<T>
where
  T: ConstructOnStack,
{
  fn drop(&mut self) {
    T::destruct(&mut *self)
  }
}
