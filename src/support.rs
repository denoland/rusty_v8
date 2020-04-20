use std::borrow::Borrow;
use std::borrow::BorrowMut;
use std::convert::AsMut;
use std::convert::AsRef;
use std::marker::PhantomData;
use std::mem::align_of;
use std::mem::forget;
use std::mem::needs_drop;
use std::mem::size_of;
use std::mem::take;
use std::ops::Deref;
use std::ops::DerefMut;
use std::ptr::drop_in_place;
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

/// Pointer to object allocated on the C++ heap. The pointer may be null.
#[repr(transparent)]
pub struct UniquePtr<T: ?Sized>(Option<UniqueRef<T>>);

impl<T: ?Sized> UniquePtr<T> {
  pub fn is_null(&self) -> bool {
    self.0.is_none()
  }

  pub fn as_ref(&self) -> Option<&UniqueRef<T>> {
    self.0.as_ref()
  }

  pub fn as_mut(&mut self) -> Option<&mut UniqueRef<T>> {
    self.0.as_mut()
  }

  pub fn take(&mut self) -> Option<UniqueRef<T>> {
    take(&mut self.0)
  }

  pub fn unwrap(self) -> UniqueRef<T> {
    self.0.unwrap()
  }
}

impl<T> UniquePtr<T> {
  pub unsafe fn from_raw(ptr: *mut T) -> Self {
    assert_unique_ptr_layout_compatible::<Self, T>();
    Self(UniqueRef::try_from_raw(ptr))
  }

  pub fn into_raw(self) -> *mut T {
    self
      .0
      .map(|unique_ref| unique_ref.into_raw())
      .unwrap_or_else(null_mut)
  }
}

impl<T: Shared> UniquePtr<T> {
  pub fn make_shared(self) -> SharedPtr<T> {
    self.into()
  }
}

impl<T> Default for UniquePtr<T> {
  fn default() -> Self {
    assert_unique_ptr_layout_compatible::<Self, T>();
    Self(None)
  }
}

impl<T> From<UniqueRef<T>> for UniquePtr<T> {
  fn from(unique_ref: UniqueRef<T>) -> Self {
    assert_unique_ptr_layout_compatible::<Self, T>();
    Self(Some(unique_ref))
  }
}

/// Pointer to object allocated on the C++ heap. The pointer may not be null.
#[repr(transparent)]
pub struct UniqueRef<T: ?Sized>(NonNull<T>);

impl<T> UniqueRef<T> {
  unsafe fn try_from_raw(ptr: *mut T) -> Option<Self> {
    assert_unique_ptr_layout_compatible::<Self, T>();
    NonNull::new(ptr).map(Self)
  }

  pub unsafe fn from_raw(ptr: *mut T) -> Self {
    assert_unique_ptr_layout_compatible::<Self, T>();
    Self::try_from_raw(ptr).unwrap()
  }

  pub fn into_raw(self) -> *mut T {
    let ptr = self.0.as_ptr();
    forget(self);
    ptr
  }
}

impl<T: Shared> UniqueRef<T> {
  pub fn make_shared(self) -> SharedRef<T> {
    self.into()
  }
}

impl<T: ?Sized> Drop for UniqueRef<T> {
  fn drop(&mut self) {
    unsafe { drop_in_place(self.0.as_ptr()) }
  }
}

impl<T: ?Sized> Deref for UniqueRef<T> {
  type Target = T;
  fn deref(&self) -> &Self::Target {
    unsafe { self.0.as_ref() }
  }
}

impl<T: ?Sized> DerefMut for UniqueRef<T> {
  fn deref_mut(&mut self) -> &mut Self::Target {
    unsafe { self.0.as_mut() }
  }
}

impl<T: ?Sized> AsRef<T> for UniqueRef<T> {
  fn as_ref(&self) -> &T {
    &**self
  }
}

impl<T: ?Sized> AsMut<T> for UniqueRef<T> {
  fn as_mut(&mut self) -> &mut T {
    &mut **self
  }
}

impl<T: ?Sized> Borrow<T> for UniqueRef<T> {
  fn borrow(&self) -> &T {
    &**self
  }
}

impl<T: ?Sized> BorrowMut<T> for UniqueRef<T> {
  fn borrow_mut(&mut self) -> &mut T {
    &mut **self
  }
}

fn assert_unique_ptr_layout_compatible<U, T>() {
  // Assert that `U` (a `UniqueRef` or `UniquePtr`) has the same memory layout
  // as a raw C pointer.
  assert_eq!(size_of::<U>(), size_of::<*mut T>());
  assert_eq!(align_of::<U>(), align_of::<*mut T>());

  // Assert that `T` (probably) implements `Drop`. If it doesn't, a regular
  // reference should be used instead of UniquePtr/UniqueRef.
  assert!(needs_drop::<T>());
}

pub trait Shared
where
  Self: Sized,
{
  fn clone(shared_ptr: &SharedPtrBase<Self>) -> SharedPtrBase<Self>;
  fn from_unique_ptr(shared_ptr: UniquePtr<Self>) -> SharedPtrBase<Self>;
  fn get(shared_ptr: &SharedPtrBase<Self>) -> *mut Self;
  fn reset(shared_ptr: &mut SharedPtrBase<Self>);
  fn use_count(shared_ptr: &SharedPtrBase<Self>) -> long;
}

/// Private base type which is shared by the `SharedPtr` and `SharedRef`
/// implementations.
#[repr(C)]
pub struct SharedPtrBase<T: Shared>([usize; 2], PhantomData<T>);

unsafe impl<T: Shared + Sync> Send for SharedPtrBase<T> {}
unsafe impl<T: Shared + Sync> Sync for SharedPtrBase<T> {}

impl<T: Shared> Default for SharedPtrBase<T> {
  fn default() -> Self {
    Self([0usize; 2], PhantomData)
  }
}

impl<T: Shared> Drop for SharedPtrBase<T> {
  fn drop(&mut self) {
    <T as Shared>::reset(self);
  }
}

/// Wrapper around a C++ shared_ptr. A shared_ptr may be be null.
#[repr(C)]
#[derive(Default)]
pub struct SharedPtr<T: Shared>(SharedPtrBase<T>);

impl<T: Shared> SharedPtr<T> {
  pub fn is_null(&self) -> bool {
    <T as Shared>::get(&self.0).is_null()
  }

  pub fn use_count(&self) -> long {
    <T as Shared>::use_count(&self.0)
  }

  pub fn take(&mut self) -> Option<SharedRef<T>> {
    if self.is_null() {
      None
    } else {
      let base = take(&mut self.0);
      Some(SharedRef(base))
    }
  }

  pub fn unwrap(self) -> SharedRef<T> {
    assert!(!self.is_null());
    SharedRef(self.0)
  }
}

impl<T: Shared> Clone for SharedPtr<T> {
  fn clone(&self) -> Self {
    Self(<T as Shared>::clone(&self.0))
  }
}

impl<T, U> From<U> for SharedPtr<T>
where
  T: Shared,
  U: Into<UniquePtr<T>>,
{
  fn from(unique_ptr: U) -> Self {
    let unique_ptr = unique_ptr.into();
    Self(<T as Shared>::from_unique_ptr(unique_ptr))
  }
}

impl<T: Shared> From<SharedRef<T>> for SharedPtr<T> {
  fn from(mut shared_ref: SharedRef<T>) -> Self {
    Self(take(&mut shared_ref.0))
  }
}

/// Wrapper around a C++ shared_ptr. The shared_ptr is assumed to contain a
/// value and may not be null.
#[repr(C)]
pub struct SharedRef<T: Shared>(SharedPtrBase<T>);

impl<T: Shared> SharedRef<T> {
  pub fn use_count(&self) -> long {
    <T as Shared>::use_count(&self.0)
  }
}

impl<T: Shared> Clone for SharedRef<T> {
  fn clone(&self) -> Self {
    Self(<T as Shared>::clone(&self.0))
  }
}

impl<T: Shared> From<UniqueRef<T>> for SharedRef<T> {
  fn from(unique_ref: UniqueRef<T>) -> Self {
    SharedPtr::from(unique_ref).unwrap()
  }
}

impl<T: Shared> Deref for SharedRef<T> {
  type Target = T;
  fn deref(&self) -> &Self::Target {
    unsafe { &*(<T as Shared>::get(&self.0)) }
  }
}

impl<T: Shared> AsRef<T> for SharedRef<T> {
  fn as_ref(&self) -> &T {
    &**self
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
