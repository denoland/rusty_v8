use std::cell::UnsafeCell;
use std::marker::PhantomData;
use std::mem::forget;
use std::mem::size_of;
use std::mem::transmute;
use std::ops::Deref;
use std::ops::DerefMut;
use std::ptr::null_mut;
use std::ptr::NonNull;

// TODO use libc::intptr_t when stable.
// https://doc.rust-lang.org/1.7.0/libc/type.intptr_t.html
#[allow(non_camel_case_types)]
pub type intptr_t = isize;

pub use std::os::raw::c_char as char;
pub use std::os::raw::c_int as int;
pub use std::os::raw::c_long as long;

pub type Opaque = [u8; 0];

pub trait Delete
where
  Self: Sized,
{
  fn delete(&mut self) -> ();
}

/// Pointer to object allocated on the C++ heap. The pointer may be null.
#[repr(transparent)]
pub struct UniquePtr<T>(Option<UniqueRef<T>>)
where
  T: Delete;

impl<T> UniquePtr<T>
where
  T: Delete,
{
  pub fn null() -> Self {
    Self(None)
  }

  pub unsafe fn from_raw(ptr: *mut T) -> Self {
    Self(UniqueRef::try_from_raw(ptr))
  }

  pub fn into_raw(self) -> *mut T {
    self
      .0
      .map(|unique_ref| unique_ref.into_raw())
      .unwrap_or_else(null_mut)
  }

  pub fn unwrap(self) -> UniqueRef<T> {
    self.0.unwrap()
  }

  unsafe fn _static_assert_has_pointer_repr() {
    let dummy: fn() -> Self = || unimplemented!();
    let _ptr: *mut T = transmute(dummy());
    let _ref: &mut T = transmute(dummy());
  }
}

impl<T> From<UniqueRef<T>> for UniquePtr<T>
where
  T: Delete,
{
  fn from(unique_ref: UniqueRef<T>) -> Self {
    Self(Some(unique_ref))
  }
}

impl<T> Deref for UniquePtr<T>
where
  T: Delete,
{
  type Target = Option<UniqueRef<T>>;
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

/// Pointer to object allocated on the C++ heap. The pointer may not be null.
#[repr(transparent)]
pub struct UniqueRef<T>(NonNull<T>)
where
  T: Delete;

impl<T> UniqueRef<T>
where
  T: Delete,
{
  pub fn make_shared(self) -> SharedRef<T>
  where
    T: Shared,
  {
    self.into()
  }

  pub unsafe fn from_raw(ptr: *mut T) -> Self {
    Self::try_from_raw(ptr).unwrap()
  }

  pub fn into_raw(self) -> *mut T {
    let ptr = self.0.as_ptr();
    forget(self);
    ptr
  }

  unsafe fn try_from_raw(ptr: *mut T) -> Option<Self> {
    NonNull::new(ptr).map(Self)
  }

  unsafe fn _static_assert_has_pointer_repr() {
    let dummy: fn() -> Self = || unimplemented!();
    let _ptr: *mut T = transmute(dummy());
    let _ref: &mut T = transmute(dummy());
  }
}

impl<T> Deref for UniqueRef<T>
where
  T: Delete,
{
  type Target = T;
  fn deref(&self) -> &Self::Target {
    unsafe { self.0.as_ref() }
  }
}

impl<T> DerefMut for UniqueRef<T>
where
  T: Delete,
{
  fn deref_mut(&mut self) -> &mut Self::Target {
    unsafe { self.0.as_mut() }
  }
}

impl<T> Drop for UniqueRef<T>
where
  T: Delete,
{
  fn drop(&mut self) {
    Delete::delete(&mut **self)
  }
}

pub trait Shared
where
  Self: Delete + 'static,
{
  fn clone(shared_ptr: *const SharedRef<Self>) -> SharedRef<Self>;
  fn from_unique(unique: UniqueRef<Self>) -> SharedRef<Self>;
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

unsafe impl<T> Send for SharedRef<T> where T: Shared + Send {}

impl<T> SharedRef<T>
where
  T: Shared,
{
  pub fn use_count(&self) -> long {
    <T as Shared>::use_count(self)
  }
}

impl<T> Clone for SharedRef<T>
where
  T: Shared,
{
  fn clone(&self) -> Self {
    <T as Shared>::clone(self)
  }
}

impl<T> From<UniqueRef<T>> for SharedRef<T>
where
  T: Delete + Shared,
{
  fn from(unique: UniqueRef<T>) -> Self {
    <T as Shared>::from_unique(unique)
  }
}

impl<T> Deref for SharedRef<T>
where
  T: Shared,
{
  type Target = UnsafeCell<T>;
  fn deref(&self) -> &Self::Target {
    unsafe { &*(<T as Shared>::deref(self) as *const UnsafeCell<T>) }
  }
}

impl<T> DerefMut for SharedRef<T>
where
  T: Shared,
{
  fn deref_mut(&mut self) -> &mut Self::Target {
    unsafe { &mut *(<T as Shared>::deref(self) as *mut UnsafeCell<T>) }
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
#[derive(Debug, PartialEq)]
pub(crate) enum MaybeBool {
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

pub trait UnitType
where
  Self: Copy + Sized,
{
  #[inline(always)]
  fn get() -> Self {
    UnitValue::<Self>::get()
  }
}

impl<T> UnitType for T where T: Copy + Sized {}

#[derive(Copy, Clone)]
struct UnitValue<T>(PhantomData<T>)
where
  Self: Sized;

impl<T> UnitValue<T>
where
  Self: Copy + Sized,
{
  const SELF: Self = Self::new_checked();

  const fn new_checked() -> Self {
    // Statically assert that T is indeed a unit type.
    let size_must_be_0 = size_of::<T>();
    let s = Self(PhantomData::<T>);
    [s][size_must_be_0]
  }

  #[inline(always)]
  fn get_checked(self) -> T {
    // This run-time check serves just as a backup for the compile-time
    // check when Self::SELF is initialized.
    assert_eq!(size_of::<T>(), 0);
    unsafe { std::mem::MaybeUninit::<T>::zeroed().assume_init() }
  }

  #[inline(always)]
  pub fn get() -> T {
    // Accessing the Self::SELF is necessary to make the compile-time type check
    // work.
    Self::SELF.get_checked()
  }
}

pub struct DefaultTag;
pub struct IdenticalConversionTag;

pub trait MapFnFrom<F, Tag = DefaultTag>
where
  F: UnitType,
  Self: Sized,
{
  fn mapping() -> Self;

  #[inline(always)]
  fn map_fn_from(_: F) -> Self {
    Self::mapping()
  }
}

impl<F> MapFnFrom<F, IdenticalConversionTag> for F
where
  Self: UnitType,
{
  #[inline(always)]
  fn mapping() -> Self {
    Self::get()
  }
}

pub trait MapFnTo<T, Tag = DefaultTag>
where
  Self: UnitType,
  T: Sized,
{
  fn mapping() -> T;

  #[inline(always)]
  fn map_fn_to(self) -> T {
    Self::mapping()
  }
}

impl<F, T, Tag> MapFnTo<T, Tag> for F
where
  Self: UnitType,
  T: MapFnFrom<F, Tag>,
{
  #[inline(always)]
  fn mapping() -> T {
    T::map_fn_from(F::get())
  }
}

pub trait CFnFrom<F>
where
  Self: Sized,
  F: UnitType,
{
  fn mapping() -> Self;

  #[inline(always)]
  fn c_fn_from(_: F) -> Self {
    Self::mapping()
  }
}

macro_rules! impl_c_fn_from {
  ($($arg:ident: $ty:ident),*) => {
    impl<F, R, $($ty),*> CFnFrom<F> for extern "C" fn($($ty),*) -> R
    where
      F: UnitType + Fn($($ty),*) -> R,
    {
      #[inline(always)]
      fn mapping() -> Self {
        extern "C" fn c_fn<F, R, $($ty),*>($($arg: $ty),*) -> R
        where
          F: UnitType + Fn($($ty),*) -> R,
        {
          (F::get())($($arg),*)
        };
        c_fn::<F, R, $($ty),*>
      }
    }
  };
}

impl_c_fn_from!();
impl_c_fn_from!(a0: A0);
impl_c_fn_from!(a0: A0, a1: A1);
impl_c_fn_from!(a0: A0, a1: A1, a2: A2);
impl_c_fn_from!(a0: A0, a1: A1, a2: A2, a3: A3);
impl_c_fn_from!(a0: A0, a1: A1, a2: A2, a3: A3, a4: A4);
impl_c_fn_from!(a0: A0, a1: A1, a2: A2, a3: A3, a4: A4, a5: A5);
impl_c_fn_from!(a0: A0, a1: A1, a2: A2, a3: A3, a4: A4, a5: A5, a6: A6);

pub trait ToCFn<T>
where
  Self: UnitType,
  T: Sized,
{
  fn mapping() -> T;

  #[inline(always)]
  fn to_c_fn(self) -> T {
    Self::mapping()
  }
}

impl<F, T> ToCFn<T> for F
where
  Self: UnitType,
  T: CFnFrom<F>,
{
  #[inline(always)]
  fn mapping() -> T {
    T::c_fn_from(F::get())
  }
}
