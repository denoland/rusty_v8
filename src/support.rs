use std::marker::PhantomData;
use std::mem::replace;
use std::mem::size_of;
use std::mem::transmute;
use std::ops::Deref;
use std::ops::DerefMut;
use std::ptr::NonNull;

// TODO use libc::intptr_t when stable.
// https://doc.rust-lang.org/1.7.0/libc/type.intptr_t.html
#[allow(non_camel_case_types)]
pub type intptr_t = isize;

pub use std::os::raw::c_char as char;
pub use std::os::raw::c_int as int;
pub use std::os::raw::c_long as long;

pub type Opaque = [usize; 0];

pub trait Delete
where
  Self: Sized + 'static,
{
  fn delete(&'static mut self) -> ();
}

/// Pointer to object allocated on the C++ heap. The pointer may be null.
#[repr(transparent)]
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

  pub fn unwrap(self) -> UniqueRef<T> {
    let p = self.into_raw();
    assert!(!p.is_null());
    unsafe { UniqueRef::from_raw(p) }
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

/// Pointer to object allocated on the C++ heap. The pointer may not be null.
#[repr(transparent)]
pub struct UniqueRef<T>(&'static mut T)
where
  T: Delete;

impl<T> UniqueRef<T>
where
  T: Delete,
{
  pub fn new(r: &'static mut T) -> Self {
    Self(r)
  }

  pub unsafe fn from_raw(p: *mut T) -> Self {
    transmute(NonNull::new(p))
  }

  pub fn into_raw(self) -> *mut T {
    unsafe { transmute(self) }
  }
}

impl<T> Deref for UniqueRef<T>
where
  T: Delete,
{
  type Target = &'static mut T;
  fn deref(&self) -> &Self::Target {
    &self.0
  }
}

impl<T> DerefMut for UniqueRef<T>
where
  T: Delete,
{
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.0
  }
}

impl<T> Drop for UniqueRef<T>
where
  T: Delete,
{
  fn drop(&mut self) {
    let inner = replace(&mut self.0, unsafe {
      transmute(NonNull::<&'static mut T>::dangling())
    });
    Delete::delete(inner)
  }
}

pub trait Shared
where
  Self: Sized + 'static,
{
  fn deref(shared_ptr: *const SharedRef<Self>) -> *mut Self;
  fn reset(shared_ptr: *mut SharedRef<Self>);
  fn use_count(shared_ptr: *const SharedRef<Self>) -> long;
}

/// Wrapper around a C++ shared_ptr. The shared_ptr is assumed to contain a
/// value and not be null.
#[repr(C)]
pub struct SharedRef<T>([*mut Opaque; 2], PhantomData<T>)
where
  T: Shared;

impl<T> SharedRef<T>
where
  T: Shared,
{
  pub fn use_count(&self) -> long {
    <T as Shared>::use_count(self)
  }
}

unsafe impl<T> Send for SharedRef<T> where T: Shared + Send {}

impl<T> Deref for SharedRef<T>
where
  T: Shared,
{
  // TODO: Maybe this should deref to UnsafeCell<T>?
  type Target = T;
  fn deref(&self) -> &T {
    unsafe { &*<T as Shared>::deref(self) }
  }
}

impl<T> DerefMut for SharedRef<T>
where
  T: Shared,
{
  fn deref_mut(&mut self) -> &mut T {
    unsafe { &mut *<T as Shared>::deref(self) }
  }
}

impl<T> Drop for SharedRef<T>
where
  T: Shared,
{
  fn drop(&mut self) {
    <T as Shared>::reset(self);
  }
}

#[repr(C)]
#[derive(PartialEq)]
pub enum MaybeBool {
  JustFalse = 0,
  JustTrue = 1,
  Nothing = 2,
}

impl Into<Option<bool>> for MaybeBool {
  fn into(self) -> Option<bool> {
    match self {
      MaybeBool::JustFalse => Some(false),
      MaybeBool::JustTrue => Some(true),
      MaybeBool::Nothing => None,
    }
  }
}

#[derive(Copy, Clone)]
#[repr(transparent)]
pub struct CxxVTable(pub *const Opaque);

#[derive(Copy, Clone)]
pub struct RustVTable<DynT>(pub *const Opaque, pub PhantomData<DynT>);

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

#[repr(C)]
#[derive(Default)]
pub struct Maybe<T> {
  has_value: bool,
  value: T,
}

impl<T> Into<Option<T>> for Maybe<T> {
  fn into(self) -> Option<T> {
    if self.has_value {
      Some(self.value)
    } else {
      None
    }
  }
}
