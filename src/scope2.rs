#![allow(dead_code)]
use crate::{
  Context, FunctionCallbackInfo, Isolate, Local, Message, Object, OwnedIsolate,
  PromiseRejectMessage, PropertyCallbackInfo, SealedLocal, Value,
  fast_api::FastApiCallbackOptions,
};
use std::{
  marker::{PhantomData, PhantomPinned},
  mem::ManuallyDrop,
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

  pub fn init_stack(mut self: Pin<&mut Self>) -> Pin<&mut T> {
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
    unsafe { raw::v8__HandleScope__DESTRUCT(&mut me.raw_handle_scope) };
  }
}

#[derive(Debug)]
pub struct HandleScope<'s, C = Context> {
  raw_handle_scope: raw::HandleScope,
  isolate: NonNull<Isolate>,
  context: Option<NonNull<Context>>,
  _phantom: PhantomData<&'s C>,
}

impl<'s, C> sealed::Sealed for HandleScope<'s, C> {}
impl<'s, C> Scope for HandleScope<'s, C> {}

pub trait GetIsolate {
  fn get_isolate_ptr(&self) -> *mut Isolate;
}

mod get_isolate_impls {
  use crate::PromiseRejectMessage;

  use super::*;
  impl GetIsolate for Isolate {
    fn get_isolate_ptr(&self) -> *mut Isolate {
      self as *const _ as *mut _
    }
  }

  impl GetIsolate for OwnedIsolate {
    fn get_isolate_ptr(&self) -> *mut Isolate {
      self as *const _ as *mut _
    }
  }

  impl GetIsolate for FunctionCallbackInfo {
    fn get_isolate_ptr(&self) -> *mut Isolate {
      self.get_isolate_ptr()
    }
  }

  impl<T> GetIsolate for PropertyCallbackInfo<T> {
    fn get_isolate_ptr(&self) -> *mut Isolate {
      self.get_isolate_ptr()
    }
  }

  impl<'s> GetIsolate for FastApiCallbackOptions<'s> {
    fn get_isolate_ptr(&self) -> *mut Isolate {
      self.isolate
    }
  }

  impl<'s> GetIsolate for Local<'s, Context> {
    fn get_isolate_ptr(&self) -> *mut Isolate {
      unsafe { raw::v8__Context__GetIsolate(&**self) }
    }
  }

  impl<'s> GetIsolate for Local<'s, Message> {
    fn get_isolate_ptr(&self) -> *mut Isolate {
      unsafe { raw::v8__Message__GetIsolate(&**self) }
    }
  }

  impl<'s, T: Into<Local<'s, Object>> + Copy> GetIsolate for T {
    fn get_isolate_ptr(&self) -> *mut Isolate {
      let object: Local<Object> = (*self).into();
      unsafe { &mut *raw::v8__Object__GetIsolate(&*object) }
    }
  }

  impl<'s> GetIsolate for PromiseRejectMessage<'s> {
    fn get_isolate_ptr(&self) -> *mut Isolate {
      let object: Local<Object> = self.get_promise().into();
      unsafe { raw::v8__Object__GetIsolate(&*object) }
    }
  }
}

pub trait NewHandleScope<'s> {
  type NewScope: Scope;

  fn make_new_scope(me: *const Self) -> Self::NewScope;
}

impl<'s, 'p: 's, C> NewHandleScope<'s> for HandleScope<'p, C> {
  type NewScope = HandleScope<'s, C>;

  fn make_new_scope(me: *const Self) -> Self::NewScope {
    HandleScope {
      raw_handle_scope: unsafe { raw::HandleScope::uninit() },
      isolate: unsafe { (*me).isolate },
      context: unsafe { (*me).context },
      _phantom: PhantomData,
    }
  }
}

impl<'s> NewHandleScope<'s> for Isolate {
  type NewScope = HandleScope<'s, ()>;

  fn make_new_scope(me: *const Self) -> Self::NewScope {
    HandleScope {
      raw_handle_scope: unsafe { raw::HandleScope::uninit() },
      isolate: NonNull::new(me as *const _ as *mut _).unwrap(),
      context: None,
      _phantom: PhantomData,
    }
  }
}

impl<'s, 'p: 's, C> NewHandleScope<'s> for CallbackScope<'p, C> {
  type NewScope = HandleScope<'s, C>;

  fn make_new_scope(me: *const Self) -> Self::NewScope {
    HandleScope {
      raw_handle_scope: unsafe { raw::HandleScope::uninit() },
      isolate: unsafe { (*me).isolate },
      context: unsafe { (*me).context },
      _phantom: PhantomData,
    }
  }
}

pub(crate) struct ScopeData {
  isolate: Option<NonNull<Isolate>>,
  context: Option<NonNull<Context>>,
}

impl ScopeData {
  pub(crate) fn get_isolate_ptr(&self) -> *mut Isolate {
    self.isolate.unwrap().as_ptr()
  }

  pub(crate) fn get_current_context(&self) -> *mut Context {
    self.context.unwrap().as_ptr()
  }
}

// stuff ported over-ish from scope.rs

impl<'s> HandleScope<'s> {
  pub fn new<P: NewHandleScope<'s>>(
    scope: *const P,
  ) -> ScopeStorage<P::NewScope> {
    ScopeStorage::new(P::make_new_scope(scope))
  }

  pub fn get_current_context<'a>(
    self: &Pin<&'a mut Self>,
  ) -> Local<'a, Context> {
    unsafe { Local::from_raw(self.context.unwrap().as_ptr()).unwrap() }
  }
}

impl<'a, C> Deref for HandleScope<'a, C> {
  type Target = Isolate;
  fn deref(&self) -> &Self::Target {
    unsafe { &*self.isolate.as_ptr() }
  }
}

impl<'a, T, C> AsRef<T> for HandleScope<'a, C>
where
  T: ?Sized,
  <HandleScope<'a, C> as Deref>::Target: AsRef<T>,
{
  fn as_ref(&self) -> &T {
    self.deref().as_ref()
  }
}

impl<'a, 'b> AsRef<Pin<&'a mut HandleScope<'b, ()>>>
  for Pin<&'a mut HandleScope<'b>>
{
  fn as_ref(&self) -> &Pin<&'a mut HandleScope<'b, ()>> {
    unsafe { std::mem::transmute(self) }
  }
}

impl<'s, C> HandleScope<'s, C> {
  #[inline(always)]
  pub(crate) unsafe fn cast_local<T>(
    self: &Pin<&mut Self>,
    _f: impl FnOnce(&mut ScopeData) -> *const T,
  ) -> Option<Local<'s, T>> {
    let mut data = ScopeData {
      context: self.context,
      isolate: Some(self.isolate),
    };
    let ptr = _f(&mut data);
    unsafe { Local::from_raw(ptr) }
  }

  pub fn throw_exception<'a>(
    self: &Pin<&'a mut Self>,
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
    self: &Pin<&'a mut Self>,
    v: SealedLocal<T>,
  ) -> Local<'a, T> {
    unsafe { Local::from_non_null(v.0) }
  }
}

impl<'a, 's, C> GetIsolate for Pin<&'a mut HandleScope<'s, C>> {
  fn get_isolate_ptr(&self) -> *mut Isolate {
    self.isolate.as_ptr()
  }
}

// ContextScope

pub struct ContextScope<'s, P> {
  raw_handle_scope: raw::ContextScope,
  scope: &'s Pin<&'s mut P>,
}

impl<'s, P> ScopeInit for ContextScope<'s, P> {
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

impl<'s, P> sealed::Sealed for ContextScope<'s, P> {}
impl<'s, P> Scope for ContextScope<'s, P> {}

pub trait NewContextScope<'s> {
  type NewScope: Scope;

  fn make_new_scope(
    me: &'s Pin<&'s mut Self>,
    context: Local<Context>,
  ) -> Self::NewScope;
}

impl<'s, 'p: 's, P: Scope> NewContextScope<'s> for ContextScope<'p, P> {
  type NewScope = ContextScope<'s, P>;

  fn make_new_scope(
    me: &'s Pin<&'s mut Self>,
    context: Local<Context>,
  ) -> Self::NewScope {
    ContextScope {
      raw_handle_scope: raw::ContextScope::new(context),
      scope: me.scope,
    }
  }
}

impl<'s, 'p: 's, C> NewContextScope<'s> for HandleScope<'p, C> {
  type NewScope = ContextScope<'s, HandleScope<'p>>;

  fn make_new_scope(
    me: &'s Pin<&'s mut Self>,
    context: Local<Context>,
  ) -> Self::NewScope {
    ContextScope {
      raw_handle_scope: raw::ContextScope::new(context),
      scope: unsafe { std::mem::transmute(me) },
    }
  }
}

impl<'s, P: NewContextScope<'s>> ContextScope<'s, P> {
  #[allow(clippy::new_ret_no_self)]
  pub fn new(
    param: &'s Pin<&'s mut P>,
    context: Local<Context>,
  ) -> P::NewScope {
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

impl<'s, P> Deref for ContextScope<'s, P> {
  type Target = Pin<&'s mut P>;
  fn deref(&self) -> &Self::Target {
    self.scope
  }
}

impl<'s, P> AsRef<Pin<&'s mut P>> for ContextScope<'s, P> {
  fn as_ref(&self) -> &Pin<&'s mut P> {
    self.scope
  }
}

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

pub struct CallbackScope<'s, C = Context> {
  raw_handle_scope: raw::HandleScope,
  isolate: NonNull<Isolate>,
  context: Option<NonNull<Context>>,
  _phantom: PhantomData<&'s C>,
  needs_scope: bool,
}

impl<'s, C> Drop for CallbackScope<'s, C> {
  fn drop(&mut self) {
    if self.needs_scope {
      unsafe { raw::v8__HandleScope__DESTRUCT(&mut self.raw_handle_scope) };
    }
  }
}

impl<'s> CallbackScope<'s> {
  pub unsafe fn new<P: NewCallbackScope<'s>>(
    param: *const P,
  ) -> ScopeStorage<P::NewScope> {
    ScopeStorage::new(P::make_new_scope(param))
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

impl<'a, C> Deref for CallbackScope<'a, C> {
  type Target = HandleScope<'a, C>;
  fn deref(&self) -> &Self::Target {
    unsafe { std::mem::transmute(self) }
  }
}

impl<'a, T, C> AsRef<T> for CallbackScope<'a, C>
where
  T: ?Sized,
  <CallbackScope<'a, C> as Deref>::Target: AsRef<T>,
{
  fn as_ref(&self) -> &T {
    self.deref().as_ref()
  }
}

impl<'a, C> CallbackScope<'a, C> {
  pub fn as_handle_scope<'b, 'c, 'd>(
    self: &'b Pin<&'d mut Self>,
  ) -> &'b Pin<&'d mut HandleScope<'c, C>> {
    unsafe { std::mem::transmute(self) }
  }
}

impl<'s, C> ScopeInit for CallbackScope<'s, C> {
  fn init_stack(storage: Pin<&mut ScopeStorage<Self>>) -> Pin<&mut Self> {
    let storage_mut = unsafe { storage.get_unchecked_mut() };
    let isolate = storage_mut.scope.isolate;
    if storage_mut.scope.needs_scope {
      unsafe {
        raw::HandleScope::init(
          &mut storage_mut.scope.raw_handle_scope,
          isolate,
        );
      }
    }

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
      if storage_mut.scope.needs_scope {
        unsafe {
          raw::HandleScope::init(
            &mut storage_mut.scope.raw_handle_scope,
            isolate,
          );
        }
      }
    }

    BoxedStorage::casted(storage)
  }

  unsafe fn deinit(_me: &mut Self) {
    // let me = unsafe { me.get_unchecked_mut() };
    // unsafe { raw::v8__ContextScope__DESTRUCT(&mut me.raw_handle_scope) };
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

  fn make_new_scope(me: *const Self) -> Self::NewScope;
}

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

  fn make_new_scope(me: *const Self) -> Self::NewScope {
    make_new_callback_scope(unsafe { &*me }, None)
  }
}

impl<'s> NewCallbackScope<'s> for OwnedIsolate {
  type NewScope = CallbackScope<'s, ()>;

  fn make_new_scope(me: *const Self) -> Self::NewScope {
    make_new_callback_scope(unsafe { &*me }, None)
  }
}

impl<'s> NewCallbackScope<'s> for FunctionCallbackInfo {
  type NewScope = CallbackScope<'s>;

  fn make_new_scope(me: *const Self) -> Self::NewScope {
    make_new_callback_scope(unsafe { &*me }, None)
  }
}

impl<'s, T> NewCallbackScope<'s> for PropertyCallbackInfo<T> {
  type NewScope = CallbackScope<'s>;

  fn make_new_scope(me: *const Self) -> Self::NewScope {
    make_new_callback_scope(unsafe { &*me }, None)
  }
}

impl<'s> NewCallbackScope<'s> for FastApiCallbackOptions<'s> {
  type NewScope = CallbackScope<'s>;
  const NEEDS_SCOPE: bool = true;

  fn make_new_scope(me: *const Self) -> Self::NewScope {
    let isolate = unsafe { (*me).get_isolate_ptr() };
    CallbackScope {
      raw_handle_scope: unsafe { raw::HandleScope::uninit() },
      isolate: NonNull::new(isolate).unwrap(),
      context: unsafe { (*me).get_context() }.map(|c| c.as_non_null()),
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

  fn make_new_scope(me: *const Self) -> Self::NewScope {
    make_new_callback_scope(
      unsafe { &*me },
      Some(unsafe { (*me).as_non_null() }),
    )
  }
}

impl<'s> NewCallbackScope<'s> for Local<'s, Message> {
  type NewScope = CallbackScope<'s>;

  fn make_new_scope(me: *const Self) -> Self::NewScope {
    make_new_callback_scope(unsafe { &*me }, None)
  }
}

impl<'s, T: Into<Local<'s, Object>> + GetIsolate> NewCallbackScope<'s> for T {
  type NewScope = CallbackScope<'s>;

  fn make_new_scope(me: *const Self) -> Self::NewScope {
    make_new_callback_scope(unsafe { &*me }, None)
  }
}

impl<'s> NewCallbackScope<'s> for PromiseRejectMessage<'s> {
  type NewScope = CallbackScope<'s>;

  fn make_new_scope(me: *const Self) -> Self::NewScope {
    make_new_callback_scope(unsafe { &*me }, None)
  }
}

impl<'s> AsRef<Pin<&'s mut HandleScope<'s, ()>>> for CallbackScope<'s, ()> {
  fn as_ref(&self) -> &Pin<&'s mut HandleScope<'s, ()>> {
    unsafe { std::mem::transmute(self) }
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

impl<'s, 'b> AsRef2<'b, Pin<&'s mut HandleScope<'b, ()>>>
  for &'b ContextScope<'s, HandleScope<'b, ()>>
{
  fn casted(self) -> &'b Pin<&'s mut HandleScope<'b, ()>> {
    unsafe { std::mem::transmute(self.scope) }
  }
}
impl<'s, 'b> AsRef2<'b, Pin<&'s mut HandleScope<'b>>>
  for ContextScope<'s, HandleScope<'b>>
{
  fn casted(self) -> &'b Pin<&'s mut HandleScope<'b>> {
    self.scope
  }
}

impl<'s, 'b, 'c, C> AsRef2<'b, Pin<&'s mut HandleScope<'c, C>>>
  for &'b Pin<&'s mut CallbackScope<'c, C>>
{
  fn casted(self) -> &'b Pin<&'s mut HandleScope<'c, C>> {
    unsafe { std::mem::transmute(self) }
  }
}


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

macro_rules! bind_callbackscope {
  (unsafe $scope: ident, $param: expr) => {
    let $scope = unsafe { $crate::CallbackScope::new($param) };
    let $scope = std::pin::pin!($scope);
    let $scope = &$scope.init_stack();
  };
}

pub(crate) use bind_callbackscope;
