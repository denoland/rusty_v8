#![allow(dead_code)]
use crate::{
  Context, FunctionCallbackInfo, Isolate, Local, Message, Object, OwnedIsolate,
  PromiseRejectMessage, PropertyCallbackInfo, SealedLocal, Value,
  fast_api::FastApiCallbackOptions, isolate::RealIsolate,
};
use std::{
  cell::Cell,
  marker::{PhantomData, PhantomPinned},
  mem::{ManuallyDrop, offset_of},
  ops::{Deref, DerefMut},
  pin::Pin,
  ptr::NonNull,
};
pub(crate) mod raw;

#[repr(C)]
pub struct ScopeStorage<T: ScopeInit> {
  inited: bool,
  scope: ManuallyDrop<T>,
  _pinned: PhantomPinned,
}

impl<T: ScopeInit> ScopeStorage<T> {
  pub fn projected(self: Pin<&mut Self>) -> Pin<&mut T> {
    let self_mut = unsafe { self.get_unchecked_mut() };
    unsafe { Pin::new_unchecked(&mut self_mut.scope) }
  }

  pub fn new(scope: T) -> Self {
    Self {
      inited: false,
      scope: ManuallyDrop::new(scope),
      _pinned: PhantomPinned,
    }
  }

  pub fn init(mut self: Pin<&mut Self>) -> Pin<&mut T> {
    if self.inited {
      // free old, going to reuse this storage
      unsafe {
        let self_mut = self.as_mut().get_unchecked_mut();
        self_mut.drop_inner();
        self_mut.inited = false;
      }
    }

    // hold onto a pointer so we can set this after initialization. we can't use a normal
    // mutable reference because the borrow checker will seee overlapping borrows. this is
    // safe, however, because we lose our mutable reference to the storage in `init_stack`
    // as it gets projected to the inner type
    let inited_ptr =
      unsafe { &raw mut self.as_mut().get_unchecked_mut().inited };
    let ret = T::init_stack(self);
    unsafe { inited_ptr.write(true) };
    ret
  }

  pub fn init_box(mut self: Pin<Box<Self>>) -> Pin<BoxedStorage<T>> {
    if self.inited {
      // free old, going to reuse this storage
      unsafe {
        let self_mut = self.as_mut().get_unchecked_mut();
        self_mut.drop_inner();
        self_mut.inited = false;
      }
    }

    // hold onto a pointer so we can set this after initialization. we can't use a normal
    // mutable reference because the borrow checker will seee overlapping borrows. this is
    // safe, however, because we lose our mutable reference to the storage in `init_stack`
    // as it gets projected to the inner type
    let inited_ptr =
      unsafe { &raw mut self.as_mut().get_unchecked_mut().inited };
    let ret = T::init_box(self);
    unsafe { inited_ptr.write(true) };
    ret
  }
  /// SAFEFTY: `self.inited` must be true, and therefore must be pinned
  unsafe fn drop_inner(&mut self) {
    unsafe {
      eprintln!("deinit");
      T::deinit(&mut self.scope);
    }
    self.inited = false;
  }
}

impl<T: ScopeInit> Drop for ScopeStorage<T> {
  fn drop(&mut self) {
    if self.inited {
      unsafe {
        self.drop_inner();
      }
    }
  }
}

pub trait Scope: Sized + sealed::Sealed + ScopeInit {}

mod sealed {
  pub trait Sealed {}
}

/// Typestate wrapper around `ScopeStorage` that reperesents an initialized,
/// boxed scope.
#[repr(transparent)]
pub struct BoxedStorage<T: ScopeInit>(Box<ScopeStorage<T>>);

impl<T: ScopeInit> BoxedStorage<T> {
  pub fn casted(value: Pin<Box<ScopeStorage<T>>>) -> Pin<BoxedStorage<T>> {
    unsafe { std::mem::transmute::<_, Pin<BoxedStorage<T>>>(value) }
  }
}

impl<T: Scope> Deref for BoxedStorage<T> {
  type Target = T;
  fn deref(&self) -> &Self::Target {
    &*self.0.scope
  }
}

impl<T: Scope> DerefMut for BoxedStorage<T> {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut *self.0.scope
  }
}

pub trait ScopeInit: Sized {
  fn init_stack(storage: Pin<&mut ScopeStorage<Self>>) -> Pin<&mut Self>;

  fn init_box(storage: Pin<Box<ScopeStorage<Self>>>)
  -> Pin<BoxedStorage<Self>>;

  unsafe fn deinit(me: &mut Self);
}

impl<'s, C> ScopeInit for HandleScope<'s, C> {
  fn init_stack(storage: Pin<&mut ScopeStorage<Self>>) -> Pin<&mut Self> {
    let storage_mut = unsafe { storage.get_unchecked_mut() };
    unsafe {
      let isolate = storage_mut.scope.isolate;
      raw::HandleScope::init(&mut storage_mut.scope.raw_handle_scope, isolate)
    };

    let projected = &mut storage_mut.scope;

    unsafe { Pin::new_unchecked(projected) }
  }

  fn init_box(
    storage: Pin<Box<ScopeStorage<Self>>>,
  ) -> Pin<BoxedStorage<Self>> {
    let mut storage = storage;
    let storage_mut = unsafe { storage.as_mut().get_unchecked_mut() };
    unsafe {
      let isolate = storage_mut.scope.isolate;
      raw::HandleScope::init(&mut storage_mut.scope.raw_handle_scope, isolate)
    };

    BoxedStorage::casted(storage)
  }

  unsafe fn deinit(me: &mut Self) {
    eprintln!("deinit handle scope");
    unsafe { raw::v8__HandleScope__DESTRUCT(&mut me.raw_handle_scope) };
  }
}

#[repr(C)]
#[derive(Debug)]
pub struct HandleScope<'s, C = Context> {
  raw_handle_scope: raw::HandleScope,
  isolate: NonNull<RealIsolate>,
  context: Option<NonNull<Context>>,
  _phantom: PhantomData<&'s C>,
}

impl<'s, C> sealed::Sealed for HandleScope<'s, C> {}
impl<'s, C> Scope for HandleScope<'s, C> {}

pub trait GetIsolate {
  fn get_isolate_ptr(&self) -> *mut RealIsolate;
}

mod get_isolate_impls {
  use crate::PromiseRejectMessage;

  use super::*;
  impl GetIsolate for Isolate {
    fn get_isolate_ptr(&self) -> *mut RealIsolate {
      self.as_real_ptr()
    }
  }

  impl GetIsolate for OwnedIsolate {
    fn get_isolate_ptr(&self) -> *mut RealIsolate {
      self.as_real_ptr()
    }
  }

  impl GetIsolate for FunctionCallbackInfo {
    fn get_isolate_ptr(&self) -> *mut RealIsolate {
      self.get_isolate_ptr()
    }
  }

  impl<T> GetIsolate for PropertyCallbackInfo<T> {
    fn get_isolate_ptr(&self) -> *mut RealIsolate {
      self.get_isolate_ptr()
    }
  }

  impl<'s> GetIsolate for FastApiCallbackOptions<'s> {
    fn get_isolate_ptr(&self) -> *mut RealIsolate {
      self.isolate
    }
  }

  impl<'s> GetIsolate for Local<'s, Context> {
    fn get_isolate_ptr(&self) -> *mut RealIsolate {
      unsafe { raw::v8__Context__GetIsolate(&**self) }
    }
  }

  impl<'s> GetIsolate for Local<'s, Message> {
    fn get_isolate_ptr(&self) -> *mut RealIsolate {
      unsafe { raw::v8__Message__GetIsolate(&**self) }
    }
  }

  impl<'s, T: Into<Local<'s, Object>> + Copy> GetIsolate for T {
    fn get_isolate_ptr(&self) -> *mut RealIsolate {
      let object: Local<Object> = (*self).into();
      unsafe { &mut *raw::v8__Object__GetIsolate(&*object) }
    }
  }

  impl<'s> GetIsolate for PromiseRejectMessage<'s> {
    fn get_isolate_ptr(&self) -> *mut RealIsolate {
      let object: Local<Object> = self.get_promise().into();
      unsafe { raw::v8__Object__GetIsolate(&*object) }
    }
  }
}

pub trait NewHandleScope<'s> {
  type NewScope: Scope;

  fn make_new_scope(me: Self) -> Self::NewScope;
}

impl<'s, 'p: 's, C> NewHandleScope<'s> for &mut HandleScope<'p, C> {
  type NewScope = HandleScope<'s, C>;

  fn make_new_scope(me: Self) -> Self::NewScope {
    HandleScope {
      raw_handle_scope: unsafe { raw::HandleScope::uninit() },
      isolate: (*me).isolate,
      context: (*me).context,
      _phantom: PhantomData,
    }
  }
}

impl<'s> NewHandleScope<'s> for &'s mut Isolate {
  type NewScope = HandleScope<'s, ()>;

  fn make_new_scope(me: Self) -> Self::NewScope {
    HandleScope {
      raw_handle_scope: unsafe { raw::HandleScope::uninit() },
      isolate: unsafe { NonNull::new_unchecked(me.as_real_ptr()) },
      context: None,
      _phantom: PhantomData,
    }
  }
}

impl<'s> NewHandleScope<'s> for &'s mut OwnedIsolate {
  type NewScope = HandleScope<'s, ()>;

  fn make_new_scope(me: Self) -> Self::NewScope {
    HandleScope {
      raw_handle_scope: unsafe { raw::HandleScope::uninit() },
      isolate: unsafe { NonNull::new_unchecked(me.get_isolate_ptr()) },
      context: None,
      _phantom: PhantomData,
    }
  }
}

impl<'s, 'p: 's, C> NewHandleScope<'s> for &mut CallbackScope<'p, C> {
  type NewScope = HandleScope<'s, C>;

  fn make_new_scope(me: Self) -> Self::NewScope {
    HandleScope {
      raw_handle_scope: unsafe { raw::HandleScope::uninit() },
      isolate: (*me).isolate,
      context: (*me).context,
      _phantom: PhantomData,
    }
  }
}

pub(crate) struct ScopeData {
  isolate: Option<NonNull<RealIsolate>>,
  context: Cell<Option<NonNull<Context>>>,
}

impl ScopeData {
  pub(crate) fn get_isolate_ptr(&self) -> *mut RealIsolate {
    self.isolate.unwrap().as_ptr()
  }

  pub(crate) fn get_current_context(&self) -> *mut Context {
    if let Some(context) = self.context.get() {
      context.as_ptr()
    } else {
      let isolate = self.get_isolate_ptr();
      let context =
        unsafe { raw::v8__Isolate__GetCurrentContext(isolate) }.cast_mut();
      self.context.set(Some(NonNull::new(context).unwrap()));
      context
    }
  }
}

// stuff ported over-ish from scope.rs

impl<'s> HandleScope<'s> {
  pub fn new<P: NewHandleScope<'s>>(scope: P) -> ScopeStorage<P::NewScope> {
    ScopeStorage::new(P::make_new_scope(scope))
  }

  pub fn get_current_context<'a>(self: &'a Self) -> Local<'a, Context> {
    unsafe { Local::from_raw(self.context.unwrap().as_ptr()).unwrap() }
  }
}

impl<'a> Deref for HandleScope<'a, ()> {
  type Target = Isolate;
  fn deref(&self) -> &Self::Target {
    unsafe {
      std::mem::transmute::<&NonNull<RealIsolate>, &Isolate>(&self.isolate)
    }
  }
}

impl<'a> Deref for HandleScope<'a> {
  type Target = HandleScope<'a, ()>;
  fn deref(&self) -> &Self::Target {
    unsafe { std::mem::transmute(self) }
  }
}

// impl<'a, T, C> AsRef<T> for HandleScope<'a, C>
// where
//     T: ?Sized,
//     <HandleScope<'a, C> as Deref>::Target: AsRef<T>,
// {
//     fn as_ref(&self) -> &T {
//         self.deref().as_ref()
//     }
// }

impl<'a, 'b> AsRef<Pin<&'a mut HandleScope<'b, ()>>>
  for Pin<&'a mut HandleScope<'b>>
{
  fn as_ref(&self) -> &Pin<&'a mut HandleScope<'b, ()>> {
    unsafe { std::mem::transmute(self) }
  }
}

impl<'s, C> HandleScope<'s, C> {
  #[inline(always)]
  pub(crate) unsafe fn cast_local<'a, T>(
    &'a self,
    _f: impl FnOnce(&mut ScopeData) -> *const T,
  ) -> Option<Local<'a, T>> {
    let mut data = ScopeData {
      context: Cell::new(self.context),
      isolate: Some(self.isolate),
    };
    let ptr = _f(&mut data);
    unsafe { Local::from_raw(ptr) }
  }

  pub fn throw_exception<'a>(
    self: &'a Self,
    exception: Local<Value>,
  ) -> Local<'a, Value> {
    unsafe {
      self.cast_local(|sd| {
        raw::v8__Isolate__ThrowException(sd.get_isolate_ptr(), &*exception)
      })
    }
    .unwrap()
  }

  /// Open a handle passed from V8 in the current scope.
  ///
  /// # Safety
  ///
  /// The handle must be rooted in this scope.
  #[inline(always)]
  pub unsafe fn unseal<'a, T>(
    self: &'a Self,
    v: SealedLocal<T>,
  ) -> Local<'a, T> {
    unsafe { Local::from_non_null(v.0) }
  }
}

impl<'a, 's, C> GetIsolate for Pin<&'a mut HandleScope<'s, C>> {
  fn get_isolate_ptr(&self) -> *mut RealIsolate {
    self.isolate.as_ptr()
  }
}

// ContextScope

pub struct ContextScope<'s, 'p, P> {
  raw_handle_scope: raw::ContextScope,
  scope: &'p Pin<&'s mut P>,
}

impl<'s, 'p, P> ScopeInit for ContextScope<'s, 'p, P> {
  fn init_stack(storage: Pin<&mut ScopeStorage<Self>>) -> Pin<&mut Self> {
    storage.projected()
  }

  fn init_box(
    storage: Pin<Box<ScopeStorage<Self>>>,
  ) -> Pin<BoxedStorage<Self>> {
    BoxedStorage::casted(storage)
  }

  unsafe fn deinit(_me: &mut Self) {
    // let me = unsafe { me.get_unchecked_mut() };
    // unsafe { raw::v8__ContextScope__DESTRUCT(&mut me.raw_handle_scope) };
  }
}

impl<'s, 'p, P> Deref for ContextScope<'s, 'p, P> {
  type Target = P;
  fn deref(&self) -> &Self::Target {
    self.scope
  }
}

impl<'s, 'p, P> sealed::Sealed for ContextScope<'s, 'p, P> {}
impl<'s, 'p, P> Scope for ContextScope<'s, 'p, P> {}

pub trait NewContextScope<'s, 'p, 'i> {
  type NewScope: Scope;

  fn make_new_scope(me: Self, context: Local<'s, Context>) -> Self::NewScope;
}

// impl<'s, 'p, P: Scope> NewContextScope<'s, 'p> for ContextScope<'s, 'p, P> {
//     type NewScope = ContextScope<'s, 'p, P>;

//     fn make_new_scope(me: &'p Pin<&'s mut Self>, context: Local<Context>) -> Self::NewScope {
//         ContextScope {
//             raw_handle_scope: raw::ContextScope::new(context),
//             scope: me.scope,
//         }
//     }
// }

impl<'s, 'p, 'i, C> NewContextScope<'s, 'p, 'i>
  for &'p Pin<&'s mut HandleScope<'i, C>>
{
  type NewScope = ContextScope<'s, 'p, HandleScope<'i>>;

  fn make_new_scope(me: Self, context: Local<'s, Context>) -> Self::NewScope {
    ContextScope {
      raw_handle_scope: raw::ContextScope::new(context),
      scope: unsafe {
        // we are adding the context, so we can mark that it now has a context.
        std::mem::transmute::<
          &'p Pin<&'s mut HandleScope<'i, C>>,
          &'p Pin<&'s mut HandleScope<'i, Context>>,
        >(me)
      },
    }
  }
}

impl<'s, 'p, 'i, P: NewContextScope<'s, 'p, 'i>> ContextScope<'s, 'p, P> {
  #[allow(clippy::new_ret_no_self)]
  pub fn new(param: P, context: Local<'s, Context>) -> P::NewScope {
    // let scope_data = param.get_scope_data_mut();
    // if scope_data.get_isolate_ptr()
    //   != unsafe { raw::v8__Context__GetIsolate(&*context) }
    // {
    //   panic!(
    //     "{} and Context do not belong to the same Isolate",
    //     type_name::<P>()
    //   )
    // }
    // let new_scope_data = scope_data.new_context_scope_data(context);
    // new_scope_data.as_scope()
    P::make_new_scope(param, context)
  }
}

// impl<'s, P> Deref for ContextScope<'s, P> {
//     type Target = Pin<&'s mut P>;
//     fn deref(&self) -> &Self::Target {
//         self.scope
//     }
// }

// impl<'s, 'p, P> AsRef<Pin<&'s mut P>> for ContextScope<'s, 'p, P> {
//     fn as_ref(&self) -> &Pin<&'s mut P> {
//         self.scope
//     }
// }

// impl<'s, 'p: 's, 'e: 'p, C> NewContextScope<'s> for EscapableHandleScope<'p, 'e, C> {
//     type NewScope = ContextScope<'s, EscapableHandleScope<'p, 'e>>;
// }

// impl<'s, 'p: 's, P: NewContextScope<'s>> NewContextScope<'s> for TryCatch<'p, P> {
//     type NewScope = <P as NewContextScope<'s>>::NewScope;
// }

// impl<'s, 'p: 's, P: NewContextScope<'s>> NewContextScope<'s>
//     for DisallowJavascriptExecutionScope<'p, P>
// {
//     type NewScope = <P as NewContextScope<'s>>::NewScope;
// }

// impl<'s, 'p: 's, P: NewContextScope<'s>> NewContextScope<'s>
//     for AllowJavascriptExecutionScope<'p, P>
// {
//     type NewScope = <P as NewContextScope<'s>>::NewScope;
// }

// callback scope

#[repr(C)]
pub struct CallbackScope<'s, C = Context> {
  raw_handle_scope: raw::HandleScope,
  isolate: NonNull<RealIsolate>,
  context: Option<NonNull<Context>>,
  _phantom: PhantomData<&'s C>,
  needs_scope: bool,
}

impl<'s, C> Drop for CallbackScope<'s, C> {
  fn drop(&mut self) {
    // if self.needs_scope {
    unsafe { raw::v8__HandleScope__DESTRUCT(&mut self.raw_handle_scope) };
    // }
  }
}

impl<'s> CallbackScope<'s> {
  pub unsafe fn new<P: NewCallbackScope<'s>>(
    param: &P,
  ) -> ScopeStorage<P::NewScope> {
    ScopeStorage::new(P::make_new_scope(param))
  }
}

impl<'s> Deref for CallbackScope<'s> {
  type Target = HandleScope<'s>;
  fn deref(&self) -> &Self::Target {
    unsafe { std::mem::transmute(self) }
  }
}
impl<'s> Deref for CallbackScope<'s, ()> {
  type Target = HandleScope<'s, ()>;
  fn deref(&self) -> &Self::Target {
    unsafe { std::mem::transmute(self) }
  }
}

// impl<'a, C> Deref for CallbackScope<'a, C> {
//   type Target = Isolate;
//   fn deref(&self) -> &Self::Target {
//     unsafe { &*self.isolate.as_ptr() }
//   }
// }

// impl<'a, T, C> AsRef<T> for CallbackScope<'a, C>
// where
//   T: ?Sized,
//   <CallbackScope<'a, C> as Deref>::Target: AsRef<T>,
// {
//   fn as_ref(&self) -> &T {
//     self.deref().as_ref()
//   }
// }

// impl<'a, C> AsRef<Isolate> for CallbackScope<'a, C> {
//   fn as_ref(&self) -> &Isolate {
//     unsafe { &*self.isolate.as_ptr() }
//   }
// }

// impl<'a, C> AsRef<HandleScope<'a, C>> for CallbackScope<'a, C> {
//   fn as_ref(&self) -> &HandleScope<'a, C> {
//     unsafe { std::mem::transmute(self) }
//   }
// }

// impl<'a, C> Deref for CallbackScope<'a, C> {
//     type Target = HandleScope<'a, C>;
//     fn deref(&self) -> &Self::Target {
//         unsafe { std::mem::transmute(self) }
//     }
// }

// impl<'a, T, C> AsRef<T> for CallbackScope<'a, C>
// where
//   T: ?Sized,
//   <CallbackScope<'a, C> as Deref>::Target: AsRef<T>,
// {
//   fn as_ref(&self) -> &T {
//     self.deref().as_ref()
//   }
// }

impl<'a, C> CallbackScope<'a, C> {
  pub fn as_handle_scope<'b, 'c, 'd>(
    self: &'b Pin<&'d mut Self>,
  ) -> &'b Pin<&'d mut HandleScope<'c, C>> {
    unsafe { std::mem::transmute(self) }
  }
  pub fn as_handle_scope_mut<'b, 'c, 'd>(
    self: &'b mut Pin<&'d mut Self>,
  ) -> &'b mut Pin<&'d mut HandleScope<'c, C>> {
    unsafe { std::mem::transmute(self) }
  }
}

impl<'s, C> ScopeInit for CallbackScope<'s, C> {
  fn init_stack(storage: Pin<&mut ScopeStorage<Self>>) -> Pin<&mut Self> {
    let storage_mut = unsafe { storage.get_unchecked_mut() };
    let isolate = storage_mut.scope.isolate;
    // if storage_mut.scope.needs_scope {
    unsafe {
      raw::HandleScope::init(&mut storage_mut.scope.raw_handle_scope, isolate);
    }
    // }

    let projected = &mut storage_mut.scope;
    unsafe { Pin::new_unchecked(projected) }
  }

  fn init_box(
    storage: Pin<Box<ScopeStorage<Self>>>,
  ) -> Pin<BoxedStorage<Self>> {
    let mut storage = storage;
    let storage_mut = unsafe { storage.as_mut().get_unchecked_mut() };
    {
      let isolate = storage_mut.scope.isolate;
      // if storage_mut.scope.needs_scope {
      unsafe {
        raw::HandleScope::init(
          &mut storage_mut.scope.raw_handle_scope,
          isolate,
        );
      }
      // }
    }

    BoxedStorage::casted(storage)
  }

  unsafe fn deinit(me: &mut Self) {
    unsafe { raw::v8__HandleScope__DESTRUCT(&mut me.raw_handle_scope) };
  }
}

impl<'s, C> sealed::Sealed for CallbackScope<'s, C> {}
impl<'s, C> Scope for CallbackScope<'s, C> {}

pub trait NewCallbackScope<'s>: Sized + GetIsolate {
  type NewScope: Scope;
  const NEEDS_SCOPE: bool = false;

  #[inline]
  fn get_context(&self) -> Option<Local<'s, Context>> {
    None
  }

  fn make_new_scope(me: &Self) -> Self::NewScope;
}

const ASSERT_CALLBACK_SCOPE_SUBSET_OF_HANDLE_SCOPE: () = {
  if !(std::mem::size_of::<CallbackScope<'static, ()>>()
    > std::mem::size_of::<HandleScope<'static, ()>>())
  {
    panic!("CallbackScope must be larger than HandleScope");
  }
  if offset_of!(CallbackScope<'static, ()>, raw_handle_scope)
    != offset_of!(HandleScope<'static, ()>, raw_handle_scope)
  {
    panic!(
      "CallbackScope and HandleScope have different offsets for raw_handle_scope"
    );
  }
  if offset_of!(CallbackScope<'static, ()>, isolate)
    != offset_of!(HandleScope<'static, ()>, isolate)
  {
    panic!("CallbackScope and HandleScope have different offsets for isolate");
  }
  if offset_of!(CallbackScope<'static, ()>, context)
    != offset_of!(HandleScope<'static, ()>, context)
  {
    panic!("CallbackScope and HandleScope have different offsets for context");
  }
  if offset_of!(CallbackScope<'static, ()>, _phantom)
    != offset_of!(HandleScope<'static, ()>, _phantom)
  {
    panic!("CallbackScope and HandleScope have different offsets for _phantom");
  }
  if std::mem::align_of::<CallbackScope<'static, ()>>()
    != std::mem::align_of::<HandleScope<'static, ()>>()
  {
    panic!(
      "CallbackScope and HandleScope have different alignments for _phantom"
    );
  }
};

fn make_new_callback_scope<'a, C>(
  isolate: &impl GetIsolate,
  context: Option<NonNull<Context>>,
) -> CallbackScope<'a, C> {
  CallbackScope {
    raw_handle_scope: unsafe { raw::HandleScope::uninit() },
    isolate: NonNull::new(isolate.get_isolate_ptr()).unwrap(),
    context,
    _phantom: PhantomData,
    needs_scope: false,
  }
}

impl<'s> NewCallbackScope<'s> for Isolate {
  type NewScope = CallbackScope<'s, ()>;

  fn make_new_scope(me: &Self) -> Self::NewScope {
    make_new_callback_scope(&*me, None)
  }
}

impl<'s> NewCallbackScope<'s> for OwnedIsolate {
  type NewScope = CallbackScope<'s, ()>;

  fn make_new_scope(me: &Self) -> Self::NewScope {
    make_new_callback_scope(&*me, None)
  }
}

impl<'s> NewCallbackScope<'s> for FunctionCallbackInfo {
  type NewScope = CallbackScope<'s>;

  fn make_new_scope(me: &Self) -> Self::NewScope {
    make_new_callback_scope(&*me, None)
  }
}

impl<'s, T> NewCallbackScope<'s> for PropertyCallbackInfo<T> {
  type NewScope = CallbackScope<'s>;

  fn make_new_scope(me: &Self) -> Self::NewScope {
    make_new_callback_scope(&*me, None)
  }
}

impl<'s> NewCallbackScope<'s> for FastApiCallbackOptions<'s> {
  type NewScope = CallbackScope<'s>;
  const NEEDS_SCOPE: bool = true;

  fn make_new_scope(me: &Self) -> Self::NewScope {
    let isolate = (*me).get_isolate_ptr();
    CallbackScope {
      raw_handle_scope: unsafe { raw::HandleScope::uninit() },
      isolate: NonNull::new(isolate).unwrap(),
      context: (*me).get_context().map(|c| c.as_non_null()),
      _phantom: PhantomData,
      needs_scope: true,
    }
  }
}

impl<'s> NewCallbackScope<'s> for Local<'s, Context> {
  type NewScope = CallbackScope<'s>;

  #[inline]
  fn get_context(&self) -> Option<Local<'s, Context>> {
    Some(*self)
  }

  fn make_new_scope(me: &Self) -> Self::NewScope {
    make_new_callback_scope(&*me, Some((*me).as_non_null()))
  }
}

impl<'s> NewCallbackScope<'s> for Local<'s, Message> {
  type NewScope = CallbackScope<'s>;

  fn make_new_scope(me: &Self) -> Self::NewScope {
    make_new_callback_scope(&*me, None)
  }
}

impl<'s, T: Into<Local<'s, Object>> + GetIsolate> NewCallbackScope<'s> for T {
  type NewScope = CallbackScope<'s>;

  fn make_new_scope(me: &Self) -> Self::NewScope {
    make_new_callback_scope(&*me, None)
  }
}

impl<'s> NewCallbackScope<'s> for PromiseRejectMessage<'s> {
  type NewScope = CallbackScope<'s>;

  fn make_new_scope(me: &Self) -> Self::NewScope {
    make_new_callback_scope(&*me, None)
  }
}

impl<'s> AsRef<Pin<&'s mut HandleScope<'s, ()>>> for CallbackScope<'s, ()> {
  fn as_ref(&self) -> &Pin<&'s mut HandleScope<'s, ()>> {
    unsafe { std::mem::transmute(self) }
  }
}

pub struct TryCatch<'s, P> {
  raw_try_catch: raw::TryCatch,
  isolate: NonNull<RealIsolate>,
  _phantom: PhantomData<&'s mut P>,
}

impl<'s, P> ScopeInit for TryCatch<'s, P> {
  fn init_stack(storage: Pin<&mut ScopeStorage<Self>>) -> Pin<&mut Self> {
    let storage_mut = unsafe { storage.get_unchecked_mut() };
    let isolate = storage_mut.scope.isolate;
    unsafe {
      raw::TryCatch::init(&mut storage_mut.scope.raw_try_catch, isolate);
    }
    let projected = &mut storage_mut.scope;
    unsafe { Pin::new_unchecked(projected) }
  }

  fn init_box(
    storage: Pin<Box<ScopeStorage<Self>>>,
  ) -> Pin<BoxedStorage<Self>> {
    let mut storage = storage;
    let storage_mut = unsafe { storage.as_mut().get_unchecked_mut() };
    let isolate = storage_mut.scope.isolate;
    unsafe {
      raw::TryCatch::init(&mut storage_mut.scope.raw_try_catch, isolate);
    }
    BoxedStorage::casted(storage)
  }

  unsafe fn deinit(me: &mut Self) {
    unsafe { raw::v8__TryCatch__DESTRUCT(&mut me.raw_try_catch) };
  }
}

impl<'s, P> sealed::Sealed for TryCatch<'s, P> {}
impl<'s, P> Scope for TryCatch<'s, P> {}

pub trait NewTryCatch<'s> {
  type NewScope: Scope;
  fn make_new_scope(me: &Self) -> Self::NewScope;
}

impl<'s, 'p: 's, C> NewTryCatch<'s> for HandleScope<'p, C> {
  type NewScope = TryCatch<'s, HandleScope<'p, C>>;
  fn make_new_scope(me: &Self) -> Self::NewScope {
    TryCatch {
      _phantom: PhantomData,
      isolate: me.isolate,
      raw_try_catch: unsafe { raw::TryCatch::uninit() },
    }
  }
}

impl<'s, 'p: 's, C> NewTryCatch<'s> for CallbackScope<'p, C> {
  type NewScope = TryCatch<'s, HandleScope<'p, C>>;
  fn make_new_scope(me: &Self) -> Self::NewScope {
    TryCatch {
      _phantom: PhantomData,
      isolate: me.isolate,
      raw_try_catch: unsafe { raw::TryCatch::uninit() },
    }
  }
}

pub trait AsRef2<'b, O> {
  fn casted(self) -> &'b O;
}

impl<'s, 'b, 'c> AsRef2<'b, Pin<&'s mut HandleScope<'c, ()>>>
  for &'b Pin<&'s mut HandleScope<'c>>
{
  fn casted(self) -> &'b Pin<&'s mut HandleScope<'c, ()>> {
    unsafe { std::mem::transmute(self) }
  }
}

// impl<'s, 'b> AsRef2<'b, Pin<&'s mut HandleScope<'b, ()>>>
//   for &'b ContextScope<'s, 'b, HandleScope<'b, ()>>
// {
//   fn casted(self) -> &'b Pin<&'s mut HandleScope<'b, ()>> {
//     unsafe { std::mem::transmute(self.scope) }
//   }
// }
// impl<'s, 'b> AsRef2<'b, Pin<&'s mut HandleScope<'b, Context>>>
//   for &'b ContextScope<'s, 'b, HandleScope<'b, Context>>
// {
//   fn casted(self) -> &'b Pin<&'s mut HandleScope<'b, Context>> {
//     unsafe { std::mem::transmute(self.scope) }
//   }
// }
// impl<'s, 'r, 'c> AsRef2<'r, Pin<&'s mut HandleScope<'c, ()>>>
//   for &'r ContextScope<'s, 'c, HandleScope<'c, Context>>
// {
//   fn casted(self) -> &'r Pin<&'s mut HandleScope<'c, ()>> {
//     unsafe { std::mem::transmute(self.scope) }
//   }
// }

// impl<'s, 'b, 'c, C> AsRef2<'b, Pin<&'s mut HandleScope<'c, C>>>
//   for &'b Pin<&'s mut CallbackScope<'c, C>>
// {
//   fn casted(self) -> &'b Pin<&'s mut HandleScope<'c, C>> {
//     unsafe { std::mem::transmute(self) }
//   }
// }

// impl<'a, C> CallbackScope<'a, C> {
//   pub fn as_handle_scope<'b, 'c, 'd>(
//     self: &'b Pin<&'d mut Self>,
//   ) -> &'b Pin<&'d mut HandleScope<'c, C>> {
//     unsafe { std::mem::transmute(self) }
//   }
// }

/*impl<'s, P> AsRef<Pin<&'s mut P>> for ContextScope<'s, P> {
  fn as_ref(&self) -> &Pin<&'s mut P> {
    self.scope
  }
} */

// impl<'s, C> AsRef2<'s, C> for CallbackScope<'s, C> {
//   fn as_scope<'a>(self: &'a Pin<&'s mut Self>) -> &'a Pin<C> {
//     self.scope
//   }
// }

#[allow(unused_macros)]
macro_rules! bind_callbackscope {
  (unsafe $scope: ident, $param: expr) => {
    let $scope = std::pin::pin!(unsafe { $crate::CallbackScope::new($param) });
    let $scope = &$scope.init();
  };
}

#[allow(unused_imports)]
pub(crate) use bind_callbackscope;
