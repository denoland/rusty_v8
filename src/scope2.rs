#![allow(dead_code)]
use crate::{
  Context, Data, DataError, Function, FunctionCallbackInfo, Isolate, Local,
  Message, Object, OwnedIsolate, PromiseRejectMessage, PropertyCallbackInfo,
  SealedLocal, Value, fast_api::FastApiCallbackOptions, isolate::RealIsolate,
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

pub type PinScope<'s, 'i, C = Context> = PinnedRef<'s, HandleScope<'i, C>>;

#[repr(C)]
pub struct ScopeStorage<T: ScopeInit> {
  inited: bool,
  scope: ManuallyDrop<T>,
  _pinned: PhantomPinned,
}

impl<T: ScopeInit> ScopeStorage<T> {
  pub(crate) fn projected(self: Pin<&mut Self>) -> Pin<&mut T> {
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

  pub fn init(mut self: Pin<&mut Self>) -> PinnedRef<'_, T> {
    if self.inited {
      // free old, going to reuse this storage
      unsafe {
        let self_mut = self.as_mut().get_unchecked_mut();
        self_mut.drop_inner();
        self_mut.inited = false;
      }
    }

    // hold onto a pointer so we can set this after initialization. we can't use a normal
    // mutable reference because the borrow checker will see overlapping borrows. this is
    // safe, however, because we lose our mutable reference to the storage in `init_stack`
    // as it gets projected to the inner type
    let inited_ptr =
      unsafe { &raw mut self.as_mut().get_unchecked_mut().inited };
    let ret = T::init_stack(self);
    unsafe { inited_ptr.write(true) };
    PinnedRef(ret)
  }

  pub fn init_box(mut self: Pin<Box<Self>>) -> PinnedBox<T> {
    if self.inited {
      // free old, going to reuse this storage
      unsafe {
        let self_mut = self.as_mut().get_unchecked_mut();
        self_mut.drop_inner();
        self_mut.inited = false;
      }
    }

    // hold onto a pointer so we can set this after initialization. we can't use a normal
    // mutable reference because the borrow checker will see overlapping borrows. this is
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

#[repr(transparent)]
pub struct PinnedBox<T: ScopeInit>(Pin<Box<ScopeStorage<T>>>);

impl<T: ScopeInit> PinnedBox<T> {
  pub fn as_mut(&mut self) -> PinnedRef<'_, T> {
    let storage = self.0.as_mut();
    let scope = unsafe { &mut storage.get_unchecked_mut().scope };
    PinnedRef(unsafe { Pin::new_unchecked(scope) })
  }
}

impl<T: ScopeInit> BoxedStorage<T> {
  pub fn into_ref(self: Pin<&mut BoxedStorage<T>>) -> PinnedRef<'_, T> {
    let self_mut = unsafe { self.get_unchecked_mut() };
    PinnedRef(unsafe { Pin::new_unchecked(&mut self_mut.0.scope) })
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

  fn init_box(storage: Pin<Box<ScopeStorage<Self>>>) -> PinnedBox<Self>;

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

  fn init_box(storage: Pin<Box<ScopeStorage<Self>>>) -> PinnedBox<Self> {
    let mut storage = storage;
    let storage_mut = unsafe { storage.as_mut().get_unchecked_mut() };
    unsafe {
      let isolate = storage_mut.scope.isolate;
      raw::HandleScope::init(&mut storage_mut.scope.raw_handle_scope, isolate)
    };

    PinnedBox(storage)
  }

  unsafe fn deinit(me: &mut Self) {
    unsafe { raw::v8__HandleScope__DESTRUCT(&mut me.raw_handle_scope) };
  }
}

#[repr(C)]
#[derive(Debug)]
pub struct HandleScope<'s, C = Context> {
  raw_handle_scope: raw::HandleScope,
  isolate: NonNull<RealIsolate>,
  context: Cell<Option<NonNull<Context>>>,
  _phantom: PhantomData<&'s C>,
  _pinned: PhantomPinned,
}

impl<'s, C> sealed::Sealed for HandleScope<'s, C> {}
impl<'s, C> Scope for HandleScope<'s, C> {}

pub trait GetIsolate {
  fn get_isolate_ptr(&self) -> *mut RealIsolate;
}

mod get_isolate_impls {
  use crate::{Promise, PromiseRejectMessage};

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

  // impl<'s, T: Into<Local<'s, Object>> + Copy> GetIsolate for T {
  //   fn get_isolate_ptr(&self) -> *mut RealIsolate {
  //     let object: Local<Object> = (*self).into();
  //     unsafe { &mut *raw::v8__Object__GetIsolate(&*object) }
  //   }
  // }

  macro_rules! impl_get_isolate_for_object_like {
      ($($t:ty),* $(,)? ) => {
          $(
              impl<'s> GetIsolate for Local<'s, $t> {
                  fn get_isolate_ptr(&self) -> *mut RealIsolate {
                      let object: Local<Object> = (*self).into();
                      unsafe { raw::v8__Object__GetIsolate(&*object) }
                  }
              }
          )*
      };
  }

  impl_get_isolate_for_object_like!(Promise,);

  impl<'s> GetIsolate for PromiseRejectMessage<'s> {
    fn get_isolate_ptr(&self) -> *mut RealIsolate {
      let object: Local<Object> = self.get_promise().into();
      unsafe { raw::v8__Object__GetIsolate(&*object) }
    }
  }

  impl<'s, C> GetIsolate for HandleScope<'s, C> {
    fn get_isolate_ptr(&self) -> *mut RealIsolate {
      self.isolate.as_ptr()
    }
  }

  impl<'s, P: GetIsolate> GetIsolate for ContextScope<'s, P> {
    fn get_isolate_ptr(&self) -> *mut RealIsolate {
      self.scope.get_isolate_ptr()
    }
  }

  impl<P: GetIsolate> GetIsolate for &mut P {
    fn get_isolate_ptr(&self) -> *mut RealIsolate {
      P::get_isolate_ptr(self)
    }
  }

  impl<P: GetIsolate> GetIsolate for &P {
    fn get_isolate_ptr(&self) -> *mut RealIsolate {
      P::get_isolate_ptr(self)
    }
  }

  impl<'s, P: GetIsolate> GetIsolate for TryCatch<'s, P> {
    fn get_isolate_ptr(&self) -> *mut RealIsolate {
      self.scope.get_isolate_ptr()
    }
  }
}

pub trait NewHandleScope<'s> {
  type NewScope: Scope;

  fn make_new_scope(me: Self) -> Self::NewScope;
}

impl<'a, 's, 'p: 's, C> NewHandleScope<'s>
  for &'a mut PinnedRef<'s, HandleScope<'p, C>>
{
  type NewScope = HandleScope<'a, C>;

  fn make_new_scope(me: Self) -> Self::NewScope {
    HandleScope {
      raw_handle_scope: unsafe { raw::HandleScope::uninit() },
      isolate: me.0.isolate,
      context: Cell::new(me.0.context.get()),
      _phantom: PhantomData,
      _pinned: PhantomPinned,
    }
  }
}

impl<'s> NewHandleScope<'s> for &'s mut Isolate {
  type NewScope = HandleScope<'s, ()>;

  fn make_new_scope(me: Self) -> Self::NewScope {
    HandleScope {
      raw_handle_scope: unsafe { raw::HandleScope::uninit() },
      isolate: unsafe { NonNull::new_unchecked(me.as_real_ptr()) },
      context: Cell::new(None),
      _phantom: PhantomData,
      _pinned: PhantomPinned,
    }
  }
}

impl<'s> NewHandleScope<'s> for &'s mut OwnedIsolate {
  type NewScope = HandleScope<'s, ()>;

  fn make_new_scope(me: Self) -> Self::NewScope {
    HandleScope {
      raw_handle_scope: unsafe { raw::HandleScope::uninit() },
      isolate: unsafe { NonNull::new_unchecked(me.get_isolate_ptr()) },
      context: Cell::new(None),
      _phantom: PhantomData,
      _pinned: PhantomPinned,
    }
  }
}

impl<'s, 'p: 's, C> NewHandleScope<'s>
  for &mut PinnedRef<'p, CallbackScope<'s, C>>
{
  type NewScope = HandleScope<'s, C>;

  fn make_new_scope(me: Self) -> Self::NewScope {
    HandleScope {
      raw_handle_scope: unsafe { raw::HandleScope::uninit() },
      isolate: me.0.isolate,
      context: Cell::new(me.0.context),
      _phantom: PhantomData,
      _pinned: PhantomPinned,
    }
  }
}

impl<'s, 'p> NewHandleScope<'s> for &mut ContextScope<'s, HandleScope<'p>> {
  type NewScope = HandleScope<'s>;
  fn make_new_scope(me: Self) -> Self::NewScope {
    HandleScope {
      raw_handle_scope: unsafe { raw::HandleScope::uninit() },
      isolate: unsafe { NonNull::new_unchecked(me.scope.get_isolate_ptr()) },
      context: Cell::new(Some(me.raw_handle_scope.entered_context)),
      _phantom: PhantomData,
      _pinned: PhantomPinned,
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
}

impl<'s, 'i> PinnedRef<'s, HandleScope<'i>> {
  /// Returns the context of the currently running JavaScript, or the context
  /// on the top of the stack if no JavaScript is running
  pub fn get_current_context(&self) -> Local<'_, Context> {
    if let Some(context) = self.0.context.get() {
      unsafe { Local::from_non_null(context) }
    } else {
      let isolate = self.0.isolate;
      let context =
        unsafe { raw::v8__Isolate__GetCurrentContext(isolate.as_ptr()) }
          .cast_mut();
      unsafe {
        self.0.context.set(Some(NonNull::new_unchecked(context)));
        Local::from_raw(context).unwrap()
      }
    }
  }

  /// Returns either the last context entered through V8's C++ API, or the
  /// context of the currently running microtask while processing microtasks.
  /// If a context is entered while executing a microtask, that context is
  /// returned.
  pub fn get_entered_or_microtask_context(&self) -> Local<'_, Context> {
    let context_ptr = unsafe {
      raw::v8__Isolate__GetEnteredOrMicrotaskContext(self.0.isolate.as_ptr())
    };
    unsafe { Local::from_raw(context_ptr) }.unwrap()
  }

  /// Return data that was previously attached to the isolate snapshot via
  /// SnapshotCreator, and removes the reference to it. If called again with
  /// same `index` argument, this function returns `DataError::NoData`.
  ///
  /// The value that was stored in the snapshot must either match or be
  /// convertible to type parameter `T`, otherwise `DataError::BadType` is
  /// returned.
  pub fn get_isolate_data_from_snapshot_once<T>(
    &self,
    index: usize,
  ) -> Result<Local<'s, T>, DataError>
  where
    T: 'static,
    for<'l> <Local<'l, Data> as TryInto<Local<'l, T>>>::Error:
      get_data_sealed::ToDataError,
    for<'l> Local<'l, Data>: TryInto<Local<'l, T>>,
  {
    unsafe {
      let Some(res) = self.cast_local(|sd| {
        raw::v8__Isolate__GetDataFromSnapshotOnce(sd.get_isolate_ptr(), index)
      }) else {
        return Err(DataError::no_data::<T>());
      };
      use get_data_sealed::ToDataError;
      match res.try_into() {
        Ok(x) => Ok(x),
        Err(e) => Err(e.to_data_error()),
      }
    }
  }

  /// Return data that was previously attached to the context snapshot via
  /// SnapshotCreator, and removes the reference to it. If called again with
  /// same `index` argument, this function returns `DataError::NoData`.
  ///
  /// The value that was stored in the snapshot must either match or be
  /// convertible to type parameter `T`, otherwise `DataError::BadType` is
  /// returned.
  pub fn get_context_data_from_snapshot_once<T>(
    &self,
    index: usize,
  ) -> Result<Local<'s, T>, DataError>
  where
    T: 'static,
    for<'l> <Local<'l, Data> as TryInto<Local<'l, T>>>::Error:
      get_data_sealed::ToDataError,
    for<'l> Local<'l, Data>: TryInto<Local<'l, T>>,
  {
    unsafe {
      let Some(res) = self.cast_local(|sd| {
        raw::v8__Context__GetDataFromSnapshotOnce(
          sd.get_current_context(),
          index,
        )
      }) else {
        return Err(DataError::no_data::<T>());
      };
      use get_data_sealed::ToDataError;
      match res.try_into() {
        Ok(x) => Ok(x),
        Err(e) => Err(e.to_data_error()),
      }
    }
  }

  #[inline(always)]
  pub fn set_promise_hooks(
    &self,
    init_hook: Option<Local<Function>>,
    before_hook: Option<Local<Function>>,
    after_hook: Option<Local<Function>>,
    resolve_hook: Option<Local<Function>>,
  ) {
    unsafe {
      let context = self.get_current_context();
      raw::v8__Context__SetPromiseHooks(
        context.as_non_null().as_ptr(),
        init_hook.map_or_else(std::ptr::null, |v| &*v),
        before_hook.map_or_else(std::ptr::null, |v| &*v),
        after_hook.map_or_else(std::ptr::null, |v| &*v),
        resolve_hook.map_or_else(std::ptr::null, |v| &*v),
      );
    }
  }

  #[inline(always)]
  pub fn set_continuation_preserved_embedder_data(&self, data: Local<Value>) {
    unsafe {
      let isolate = self.0.isolate;
      raw::v8__Context__SetContinuationPreservedEmbedderData(
        isolate.as_ptr(),
        &*data,
      );
    }
  }

  #[inline(always)]
  pub fn get_continuation_preserved_embedder_data(&self) -> Local<'s, Value> {
    unsafe {
      self
        .cast_local(|sd| {
          raw::v8__Context__GetContinuationPreservedEmbedderData(
            sd.get_isolate_ptr(),
          )
        })
        .unwrap()
    }
  }
}

// for<'l> DataError: From<<Local<'s, Data> as TryInto<Local<'l, T>>>::Error>,
mod get_data_sealed {
  use crate::DataError;
  use std::convert::Infallible;

  pub trait ToDataError {
    fn to_data_error(self) -> DataError;
  }
  impl ToDataError for DataError {
    fn to_data_error(self) -> DataError {
      self
    }
  }
  impl ToDataError for Infallible {
    fn to_data_error(self) -> DataError {
      unreachable!()
    }
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

impl<'p, 'i> PinnedRef<'p, HandleScope<'i, ()>> {
  #[inline(always)]
  pub(crate) unsafe fn cast_local<'a, 'b, T>(
    &self,
    _f: impl FnOnce(&mut ScopeData) -> *const T,
  ) -> Option<Local<'b, T>> {
    let mut data: ScopeData = ScopeData {
      context: Cell::new(self.0.context.get()),
      isolate: Some(self.0.isolate),
    };
    let ptr = _f(&mut data);
    unsafe { Local::from_raw(ptr) }
  }
  pub fn throw_exception(
    &self,
    exception: Local<'p, Value>,
  ) -> Local<'p, Value> {
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
  pub unsafe fn unseal<'a, T>(&self, v: SealedLocal<T>) -> Local<'a, T> {
    unsafe { Local::from_non_null(v.0) }
  }
}

impl<'s, C> HandleScope<'s, C> {}

impl<'a, 's, C> GetIsolate for Pin<&'a mut HandleScope<'s, C>> {
  fn get_isolate_ptr(&self) -> *mut RealIsolate {
    self.isolate.as_ptr()
  }
}

// ContextScope

#[repr(C)]
pub struct ContextScope<'s, P> {
  raw_handle_scope: raw::ContextScope,
  scope: PinnedRef<'s, P>,
}

impl<'s, P> ScopeInit for ContextScope<'s, P> {
  fn init_stack(storage: Pin<&mut ScopeStorage<Self>>) -> Pin<&mut Self> {
    storage.projected()
  }

  fn init_box(storage: Pin<Box<ScopeStorage<Self>>>) -> PinnedBox<Self> {
    PinnedBox(storage)
  }

  unsafe fn deinit(_me: &mut Self) {
    // let me = unsafe { me.get_unchecked_mut() };
    // unsafe { raw::v8__ContextScope__DESTRUCT(&mut me.raw_handle_scope) };
  }
}

impl<'s, P> Deref for ContextScope<'s, P> {
  type Target = PinnedRef<'s, P>;
  fn deref(&self) -> &Self::Target {
    &self.scope
  }
}

impl<'s, P> DerefMut for ContextScope<'s, P> {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.scope
  }
}

impl<'s, P> sealed::Sealed for ContextScope<'s, P> {}
impl<'s, P> Scope for ContextScope<'s, P> {}

pub trait NewContextScope<'s> {
  type NewScope: Scope;

  fn make_new_scope(me: Self, context: Local<'s, Context>) -> Self::NewScope;
}

impl<'s, 'p, P: Scope> NewContextScope<'s> for &'p mut ContextScope<'s, P> {
  type NewScope = ContextScope<'p, P>;

  fn make_new_scope(me: Self, context: Local<Context>) -> Self::NewScope {
    ContextScope {
      raw_handle_scope: raw::ContextScope::new(context),
      scope: me.scope.as_mut(),
    }
  }
}

impl<'s, 'p, C> NewContextScope<'s> for PinnedRef<'s, HandleScope<'p, C>> {
  type NewScope = ContextScope<'s, HandleScope<'p>>;

  fn make_new_scope(me: Self, context: Local<'s, Context>) -> Self::NewScope {
    ContextScope {
      raw_handle_scope: raw::ContextScope::new(context),
      scope: unsafe {
        // we are adding the context, so we can mark that it now has a context.
        std::mem::transmute::<
          PinnedRef<'s, HandleScope<'p, C>>,
          PinnedRef<'s, HandleScope<'p, Context>>,
        >(me)
      },
    }
  }
}

impl<'a, 's, 'p, C> NewContextScope<'a>
  for &'a mut PinnedRef<'s, HandleScope<'p, C>>
{
  type NewScope = ContextScope<'a, HandleScope<'p>>;

  fn make_new_scope(me: Self, context: Local<'a, Context>) -> Self::NewScope {
    NewContextScope::make_new_scope(me.as_mut(), context)
  }
}

impl<'a, 's, 'p, C> NewContextScope<'a>
  for &'a mut PinnedRef<'s, CallbackScope<'p, C>>
{
  type NewScope = ContextScope<'a, HandleScope<'p>>;

  fn make_new_scope(me: Self, context: Local<'a, Context>) -> Self::NewScope {
    ContextScope {
      raw_handle_scope: raw::ContextScope::new(context),
      scope: unsafe {
        // we are adding the context, so we can mark that it now has a context.
        std::mem::transmute::<
          PinnedRef<'a, CallbackScope<'p, C>>,
          PinnedRef<'a, HandleScope<'p, Context>>,
        >(me.as_mut())
      },
    }
  }
}

impl<'s, P: NewContextScope<'s>> ContextScope<'s, P> {
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
  _pinned: PhantomPinned,
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
    param: P,
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

impl<'s, C> AsRef<Isolate> for CallbackScope<'s, C> {
  fn as_ref(&self) -> &Isolate {
    unsafe {
      &std::mem::transmute::<&NonNull<RealIsolate>, &Isolate>(&self.isolate)
    }
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

  fn init_box(storage: Pin<Box<ScopeStorage<Self>>>) -> PinnedBox<Self> {
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

    PinnedBox(storage)
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

  fn make_new_scope(me: Self) -> Self::NewScope;
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
  isolate: impl GetIsolate,
  context: Option<NonNull<Context>>,
) -> CallbackScope<'a, C> {
  CallbackScope {
    raw_handle_scope: unsafe { raw::HandleScope::uninit() },
    isolate: NonNull::new(isolate.get_isolate_ptr()).unwrap(),
    context,
    _phantom: PhantomData,
    _pinned: PhantomPinned,
    needs_scope: false,
  }
}

impl<'s> NewCallbackScope<'s> for &Isolate {
  type NewScope = CallbackScope<'s, ()>;

  fn make_new_scope(me: Self) -> Self::NewScope {
    make_new_callback_scope(&*me, None)
  }
}

impl<'s> NewCallbackScope<'s> for &OwnedIsolate {
  type NewScope = CallbackScope<'s, ()>;

  fn make_new_scope(me: Self) -> Self::NewScope {
    make_new_callback_scope(&*me, None)
  }
}

impl<'s> NewCallbackScope<'s> for &FunctionCallbackInfo {
  type NewScope = CallbackScope<'s>;

  fn make_new_scope(me: Self) -> Self::NewScope {
    make_new_callback_scope(&*me, None)
  }
}

impl<'s, T> NewCallbackScope<'s> for &PropertyCallbackInfo<T> {
  type NewScope = CallbackScope<'s>;

  fn make_new_scope(me: Self) -> Self::NewScope {
    make_new_callback_scope(&*me, None)
  }
}

impl<'s> NewCallbackScope<'s> for &FastApiCallbackOptions<'s> {
  type NewScope = CallbackScope<'s>;
  const NEEDS_SCOPE: bool = true;

  fn make_new_scope(me: Self) -> Self::NewScope {
    let isolate = (*me).get_isolate_ptr();
    CallbackScope {
      raw_handle_scope: unsafe { raw::HandleScope::uninit() },
      isolate: NonNull::new(isolate).unwrap(),
      context: me.get_context().map(|c| c.as_non_null()),
      _phantom: PhantomData,
      _pinned: PhantomPinned,
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

  fn make_new_scope(me: Self) -> Self::NewScope {
    make_new_callback_scope(me, Some(me.as_non_null()))
  }
}

impl<'s> NewCallbackScope<'s> for Local<'s, Message> {
  type NewScope = CallbackScope<'s>;

  fn make_new_scope(me: Self) -> Self::NewScope {
    make_new_callback_scope(&me, None)
  }
}

impl<'s, T: Into<Local<'s, Object>> + GetIsolate> NewCallbackScope<'s> for T {
  type NewScope = CallbackScope<'s>;

  fn make_new_scope(me: Self) -> Self::NewScope {
    make_new_callback_scope(me, None)
  }
}

impl<'s> NewCallbackScope<'s> for &PromiseRejectMessage<'s> {
  type NewScope = CallbackScope<'s>;

  fn make_new_scope(me: Self) -> Self::NewScope {
    make_new_callback_scope(me, None)
  }
}

impl<'s> AsRef<Pin<&'s mut HandleScope<'s, ()>>> for CallbackScope<'s, ()> {
  fn as_ref(&self) -> &Pin<&'s mut HandleScope<'s, ()>> {
    unsafe { std::mem::transmute(self) }
  }
}

pub struct TryCatch<'s, P> {
  raw_try_catch: raw::TryCatch,
  scope: PinnedRef<'s, P>,
  _pinned: PhantomPinned,
}

impl<'s, P: NewTryCatch<'s>> TryCatch<'s, P> {
  pub fn new(param: P) -> ScopeStorage<P::NewScope> {
    ScopeStorage::new(P::make_new_scope(param))
  }
}

impl<'s, P: GetIsolate> ScopeInit for TryCatch<'s, P> {
  fn init_stack(storage: Pin<&mut ScopeStorage<Self>>) -> Pin<&mut Self> {
    let storage_mut = unsafe { storage.get_unchecked_mut() };
    let isolate = unsafe {
      NonNull::new_unchecked(storage_mut.scope.scope.get_isolate_ptr())
    };
    unsafe {
      raw::TryCatch::init(&mut storage_mut.scope.raw_try_catch, isolate);
    }
    let projected = &mut storage_mut.scope;
    unsafe { Pin::new_unchecked(projected) }
  }

  fn init_box(storage: Pin<Box<ScopeStorage<Self>>>) -> PinnedBox<Self> {
    let mut storage = storage;
    let storage_mut = unsafe { storage.as_mut().get_unchecked_mut() };
    let isolate = unsafe {
      NonNull::new_unchecked(storage_mut.scope.scope.get_isolate_ptr())
    };
    unsafe {
      raw::TryCatch::init(&mut storage_mut.scope.raw_try_catch, isolate);
    }
    PinnedBox(storage)
  }

  unsafe fn deinit(me: &mut Self) {
    unsafe { raw::v8__TryCatch__DESTRUCT(&mut me.raw_try_catch) };
  }
}

impl<'t, 's, 'i> PinnedRef<'t, TryCatch<'s, HandleScope<'i>>> {
  /// Returns true if an exception has been caught by this try/catch block.
  #[inline(always)]
  pub fn has_caught(&self) -> bool {
    unsafe { raw::v8__TryCatch__HasCaught(self.get_raw()) }
  }

  /// For certain types of exceptions, it makes no sense to continue execution.
  ///
  /// If CanContinue returns false, the correct action is to perform any C++
  /// cleanup needed and then return. If CanContinue returns false and
  /// HasTerminated returns true, it is possible to call
  /// CancelTerminateExecution in order to continue calling into the engine.
  #[inline(always)]
  pub fn can_continue(&self) -> bool {
    unsafe { raw::v8__TryCatch__CanContinue(self.get_raw()) }
  }

  /// Returns true if an exception has been caught due to script execution
  /// being terminated.
  ///
  /// There is no JavaScript representation of an execution termination
  /// exception. Such exceptions are thrown when the TerminateExecution
  /// methods are called to terminate a long-running script.
  ///
  /// If such an exception has been thrown, HasTerminated will return true,
  /// indicating that it is possible to call CancelTerminateExecution in order
  /// to continue calling into the engine.
  #[inline(always)]
  pub fn has_terminated(&self) -> bool {
    unsafe { raw::v8__TryCatch__HasTerminated(self.get_raw()) }
  }

  /// Returns true if verbosity is enabled.
  #[inline(always)]
  pub fn is_verbose(&self) -> bool {
    unsafe { raw::v8__TryCatch__IsVerbose(self.get_raw()) }
  }

  /// Set verbosity of the external exception handler.
  ///
  /// By default, exceptions that are caught by an external exception
  /// handler are not reported. Call SetVerbose with true on an
  /// external exception handler to have exceptions caught by the
  /// handler reported as if they were not caught.
  #[inline(always)]
  pub fn set_verbose(&mut self, value: bool) {
    unsafe { raw::v8__TryCatch__SetVerbose(self.get_raw_mut(), value) };
  }

  /// Set whether or not this TryCatch should capture a Message object
  /// which holds source information about where the exception
  /// occurred. True by default.
  #[inline(always)]
  pub fn set_capture_message(&mut self, value: bool) {
    unsafe { raw::v8__TryCatch__SetCaptureMessage(self.get_raw_mut(), value) };
  }

  /// Clears any exceptions that may have been caught by this try/catch block.
  /// After this method has been called, HasCaught() will return false. Cancels
  /// the scheduled exception if it is caught and ReThrow() is not called
  /// before.
  ///
  /// It is not necessary to clear a try/catch block before using it again; if
  /// another exception is thrown the previously caught exception will just be
  /// overwritten. However, it is often a good idea since it makes it easier
  /// to determine which operation threw a given exception.
  #[inline(always)]
  pub fn reset(&mut self) {
    unsafe { raw::v8__TryCatch__Reset(self.get_raw_mut()) };
  }

  #[inline(always)]
  fn get_raw(&self) -> &raw::TryCatch {
    &self.0.raw_try_catch
  }

  #[inline(always)]
  unsafe fn get_raw_mut<'a>(&'a mut self) -> &'a mut raw::TryCatch {
    unsafe { &mut self.0.as_mut().get_unchecked_mut().raw_try_catch }
  }

  pub fn exception(&self) -> Option<Local<'s, Value>> {
    unsafe {
      self
        .0
        .scope
        .cast_local(|_data| raw::v8__TryCatch__Exception(self.get_raw()))
    }
  }

  pub fn message(&self) -> Option<Local<'s, Message>> {
    unsafe {
      self
        .0
        .scope
        .cast_local(|_data| raw::v8__TryCatch__Message(self.get_raw()))
    }
  }

  pub fn rethrow<'a, 'b>(&'a mut self) -> Option<Local<'b, Value>> {
    let raw_mut = unsafe { self.get_raw_mut() as *mut raw::TryCatch };
    unsafe {
      self
        .0
        .scope
        .cast_local(|_data| raw::v8__TryCatch__ReThrow(raw_mut))
    }
  }

  pub fn stack_trace(&self) -> Option<Local<'s, Value>> {
    unsafe {
      self.0.scope.cast_local(|_data| {
        raw::v8__TryCatch__StackTrace(
          self.get_raw(),
          _data.get_current_context(),
        )
      })
    }
  }
}

impl<'s, P> sealed::Sealed for TryCatch<'s, P> {}
impl<'s, P: Scope + GetIsolate> Scope for TryCatch<'s, P> {}

pub trait NewTryCatch<'s>: GetIsolate {
  type NewScope: Scope;
  fn make_new_scope(me: Self) -> Self::NewScope;
}

impl<'s, 'p, C> NewTryCatch<'s> for PinnedRef<'s, HandleScope<'p, C>> {
  type NewScope = TryCatch<'s, HandleScope<'p, C>>;
  fn make_new_scope(me: Self) -> Self::NewScope {
    TryCatch {
      scope: me,
      raw_try_catch: unsafe { raw::TryCatch::uninit() },
      _pinned: PhantomPinned,
    }
  }
}

impl<'borrow, 's, 'p, C> NewTryCatch<'s>
  for &'borrow mut PinnedRef<'s, HandleScope<'p, C>>
{
  type NewScope = TryCatch<'borrow, HandleScope<'p, C>>;
  fn make_new_scope(me: Self) -> Self::NewScope {
    NewTryCatch::make_new_scope(me.as_mut())
  }
}

impl<'borrow, 's, 'i, T: GetIsolate + Scope> NewTryCatch<'s>
  for &'borrow mut ContextScope<'s, T>
{
  type NewScope = TryCatch<'borrow, T>;
  fn make_new_scope(me: Self) -> Self::NewScope {
    TryCatch {
      scope: me.scope.as_mut(),
      raw_try_catch: unsafe { raw::TryCatch::uninit() },
      _pinned: PhantomPinned,
    }
  }
}

impl<'p, 's, 'i, C> NewTryCatch<'p>
  for PinnedRef<'p, TryCatch<'s, HandleScope<'i, C>>>
{
  type NewScope = TryCatch<'p, HandleScope<'i, C>>;
  fn make_new_scope(me: Self) -> Self::NewScope {
    TryCatch {
      scope: unsafe { std::ptr::read(&mut me.0.get_unchecked_mut().scope) },
      raw_try_catch: unsafe { raw::TryCatch::uninit() },
      _pinned: PhantomPinned,
    }
  }
}

impl<'borrow, 'p, 's, 'i, C> NewTryCatch<'borrow>
  for &'borrow mut PinnedRef<'p, TryCatch<'s, HandleScope<'i, C>>>
{
  type NewScope = TryCatch<'borrow, HandleScope<'i, C>>;
  fn make_new_scope(me: Self) -> Self::NewScope {
    <PinnedRef<'borrow, TryCatch<'s, HandleScope<'i, C>>> as NewTryCatch<
      'borrow,
    >>::make_new_scope(me.as_mut())
  }
}

// impl<'borrow, 's, 'i, C> NewTryCatch<'borrow>
//   for PinnedRef<'borrow, TryCatch<'s, HandleScope<'i, C>>>
// {
//   type NewScope = TryCatch<'borrow, HandleScope<'i, C>>;
//   fn make_new_scope(me: Self) -> Self::NewScope {
//     TryCatch {
//       scope: unsafe { PinnedRef(me.0.get_unchecked_mut().scope.0) },
//       raw_try_catch: unsafe { raw::TryCatch::uninit() },
//       _pinned: PhantomPinned,
//     }
//   }
// }

#[repr(C)]
pub struct EscapableHandleScope<'s, 'esc: 's> {
  raw_handle_scope: raw::HandleScope,
  isolate: NonNull<RealIsolate>,
  context: Cell<Option<NonNull<Context>>>,
  raw_escape_slot: Option<raw::EscapeSlot>,
  _phantom: PhantomData<(
    &'s mut raw::HandleScope,
    &'esc mut raw::EscapeSlot,
    &'s Context,
  )>,
  _pinned: PhantomPinned,
}

impl<'s, 'esc: 's> ScopeInit for EscapableHandleScope<'s, 'esc> {
  fn init_stack(storage: Pin<&mut ScopeStorage<Self>>) -> Pin<&mut Self> {
    let storage_mut = unsafe { storage.get_unchecked_mut() };
    unsafe {
      let isolate = storage_mut.scope.isolate;
      raw::HandleScope::init(&mut storage_mut.scope.raw_handle_scope, isolate);
    }
    let projected = &mut storage_mut.scope;

    unsafe { Pin::new_unchecked(projected) }
  }

  fn init_box(storage: Pin<Box<ScopeStorage<Self>>>) -> PinnedBox<Self> {
    let mut storage = storage;
    let storage_mut = unsafe { storage.as_mut().get_unchecked_mut() };
    unsafe {
      let isolate = storage_mut.scope.isolate;
      raw::HandleScope::init(&mut storage_mut.scope.raw_handle_scope, isolate);
    }
    PinnedBox(storage)
  }

  unsafe fn deinit(me: &mut Self) {
    unsafe { raw::v8__HandleScope__DESTRUCT(&raw mut me.raw_handle_scope) };
  }
}

impl<'s, 'esc: 's> EscapableHandleScope<'s, 'esc> {
  pub fn new<P: NewEscapableHandleScope<'s>>(
    scope: P,
  ) -> ScopeStorage<P::NewScope> {
    ScopeStorage::new(P::make_new_scope(scope))
  }
}

impl<'p, 's, 'esc: 's> PinnedRef<'p, EscapableHandleScope<'s, 'esc>> {
  pub fn escape<T>(&mut self, value: Local<T>) -> Local<'esc, T>
  where
    for<'l> Local<'l, T>: Into<Local<'l, crate::Data>>,
  {
    let escape_slot = unsafe { self.0.as_mut().get_unchecked_mut() }
      .raw_escape_slot
      .take()
      .expect("EscapableHandleScope::escape() called twice");
    escape_slot.escape(value)
  }
}

impl<'p, 's, 'esc: 's> Deref for PinnedRef<'p, EscapableHandleScope<'s, 'esc>> {
  type Target = PinnedRef<'p, HandleScope<'s>>;
  fn deref(&self) -> &Self::Target {
    unsafe { std::mem::transmute(self) }
  }
}

impl<'p, 's, 'esc: 's> DerefMut
  for PinnedRef<'p, EscapableHandleScope<'s, 'esc>>
{
  fn deref_mut(&mut self) -> &mut Self::Target {
    unsafe { std::mem::transmute(self) }
  }
}

pub trait NewEscapableHandleScope<'s> {
  type NewScope: Scope;
  fn make_new_scope(me: Self) -> Self::NewScope;
}

impl<'s, 'p: 's> NewEscapableHandleScope<'s>
  for PinnedRef<'s, HandleScope<'p, Context>>
{
  type NewScope = EscapableHandleScope<'s, 'p>;
  fn make_new_scope(me: Self) -> Self::NewScope {
    // Note: the `raw_escape_slot` field must be initialized _before_ the
    // `raw_handle_scope` field, otherwise the escaped local handle ends up
    // inside the `EscapableHandleScope` that's being constructed here,
    // rather than escaping from it.
    let isolate = me.0.isolate;
    let raw_escape_slot = raw::EscapeSlot::new(isolate);
    let raw_handle_scope = unsafe { raw::HandleScope::uninit() };

    EscapableHandleScope {
      isolate,
      context: Cell::new(me.0.context.get()),
      raw_escape_slot: Some(raw_escape_slot),
      raw_handle_scope: raw_handle_scope,
      _phantom: PhantomData,
      _pinned: PhantomPinned,
    }
  }
}

impl<'borrow, 's, 'p: 's> NewEscapableHandleScope<'s>
  for &'borrow mut PinnedRef<'p, HandleScope<'s, Context>>
{
  type NewScope = EscapableHandleScope<'borrow, 'p>;
  fn make_new_scope(me: Self) -> Self::NewScope {
    NewEscapableHandleScope::make_new_scope(me.as_mut())
  }
}

impl<'borrow, 's, 'p: 's> NewEscapableHandleScope<'s>
  for &'borrow mut ContextScope<'s, HandleScope<'p, Context>>
{
  type NewScope = EscapableHandleScope<'borrow, 'p>;
  fn make_new_scope(me: Self) -> Self::NewScope {
    NewEscapableHandleScope::make_new_scope(me.scope.as_mut())
  }
}

impl<'s, 'esc: 's> sealed::Sealed for EscapableHandleScope<'s, 'esc> {}
impl<'s, 'esc: 's> Scope for EscapableHandleScope<'s, 'esc> {}

// impl<'s, 'p: 's, C> NewTryCatch<'s> for &mut CallbackScope<'p, C> {
//   type NewScope = TryCatch<'s, HandleScope<'p, C>>;
//   fn make_new_scope(me: Self) -> Self::NewScope {
//     TryCatch {
//       scope: me,
//       raw_try_catch: unsafe { raw::TryCatch::uninit() },
//     }
//   }
// }
//
//

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub enum OnFailure {
  CrashOnFailure,
  ThrowOnFailure,
  DumpOnFailure,
}

#[repr(C)]
pub struct DisallowJavascriptExecutionScope<'s, P> {
  raw: raw::DisallowJavascriptExecutionScope,
  scope: PinnedRef<'s, P>,
  on_failure: OnFailure,
  _pinned: PhantomPinned,
}

impl<'s, P: GetIsolate> ScopeInit for DisallowJavascriptExecutionScope<'s, P> {
  fn init_stack(storage: Pin<&mut ScopeStorage<Self>>) -> Pin<&mut Self> {
    let storage_mut = unsafe { storage.get_unchecked_mut() };
    let isolate = storage_mut.scope.scope.get_isolate_ptr();
    let on_failure = storage_mut.scope.on_failure;
    unsafe {
      raw::DisallowJavascriptExecutionScope::init(
        &mut storage_mut.scope.raw,
        NonNull::new_unchecked(isolate),
        on_failure,
      );
      Pin::new_unchecked(&mut storage_mut.scope)
    }
  }

  fn init_box(storage: Pin<Box<ScopeStorage<Self>>>) -> PinnedBox<Self> {
    let mut storage = storage;
    let storage_mut = unsafe { storage.as_mut().get_unchecked_mut() };
    let isolate = storage_mut.scope.scope.get_isolate_ptr();
    let on_failure = storage_mut.scope.on_failure;
    unsafe {
      raw::DisallowJavascriptExecutionScope::init(
        &mut storage_mut.scope.raw,
        NonNull::new_unchecked(isolate),
        on_failure,
      );
    }
    PinnedBox(storage)
  }

  unsafe fn deinit(me: &mut Self) {
    unsafe { raw::v8__DisallowJavascriptExecutionScope__DESTRUCT(&mut me.raw) };
  }
}

impl<'s, P: GetIsolate> sealed::Sealed
  for DisallowJavascriptExecutionScope<'s, P>
{
}
impl<'s, P: Scope + GetIsolate> Scope
  for DisallowJavascriptExecutionScope<'s, P>
{
}

impl<'s, P: NewDisallowJavascriptExecutionScope<'s>>
  DisallowJavascriptExecutionScope<'s, P>
{
  pub fn new(param: P, on_failure: OnFailure) -> ScopeStorage<P::NewScope> {
    ScopeStorage::new(P::make_new_scope(param, on_failure))
  }
}

pub trait NewDisallowJavascriptExecutionScope<'s> {
  type NewScope: Scope;
  fn make_new_scope(me: Self, on_failure: OnFailure) -> Self::NewScope;
}

impl<'s, 'i, C> NewDisallowJavascriptExecutionScope<'s>
  for PinnedRef<'s, HandleScope<'i, C>>
{
  type NewScope = DisallowJavascriptExecutionScope<'s, HandleScope<'i, C>>;
  fn make_new_scope(me: Self, on_failure: OnFailure) -> Self::NewScope {
    DisallowJavascriptExecutionScope {
      raw: unsafe { raw::DisallowJavascriptExecutionScope::uninit() },
      scope: me,
      on_failure,
      _pinned: PhantomPinned,
    }
  }
}

impl<'borrow, 's, P> NewDisallowJavascriptExecutionScope<'borrow>
  for &'borrow mut PinnedRef<'s, P>
where
  PinnedRef<'borrow, P>: NewDisallowJavascriptExecutionScope<'borrow>,
{
  type NewScope =
        <PinnedRef<'borrow, P> as NewDisallowJavascriptExecutionScope<'borrow>>::NewScope;
  fn make_new_scope(me: Self, on_failure: OnFailure) -> Self::NewScope {
    PinnedRef::<'borrow, P>::make_new_scope(me.as_mut(), on_failure)
  }
}

impl<'borrow, 's, P> NewDisallowJavascriptExecutionScope<'borrow>
  for &'borrow mut ContextScope<'s, P>
where
  PinnedRef<'borrow, P>: NewDisallowJavascriptExecutionScope<'borrow>,
{
  type NewScope = <PinnedRef<'borrow, P> as NewDisallowJavascriptExecutionScope<
    'borrow,
  >>::NewScope;
  fn make_new_scope(me: Self, on_failure: OnFailure) -> Self::NewScope {
    PinnedRef::<'borrow, P>::make_new_scope(me.scope.as_mut(), on_failure)
  }
}

#[repr(C)]
pub struct AllowJavascriptExecutionScope<'s, P> {
  raw: raw::AllowJavascriptExecutionScope,
  scope: PinnedRef<'s, P>,
  _pinned: PhantomPinned,
}

impl<'s, P: GetIsolate> ScopeInit for AllowJavascriptExecutionScope<'s, P> {
  fn init_stack(storage: Pin<&mut ScopeStorage<Self>>) -> Pin<&mut Self> {
    let storage_mut = unsafe { storage.get_unchecked_mut() };
    let isolate = unsafe {
      NonNull::new_unchecked(storage_mut.scope.scope.get_isolate_ptr())
    };
    unsafe {
      raw::AllowJavascriptExecutionScope::init(
        &mut storage_mut.scope.raw,
        isolate,
      );
    }
    let projected = &mut storage_mut.scope;
    unsafe { Pin::new_unchecked(projected) }
  }

  fn init_box(storage: Pin<Box<ScopeStorage<Self>>>) -> PinnedBox<Self> {
    let mut storage = storage;
    let storage_mut = unsafe { storage.as_mut().get_unchecked_mut() };
    let isolate = unsafe {
      NonNull::new_unchecked(storage_mut.scope.scope.get_isolate_ptr())
    };
    unsafe {
      raw::AllowJavascriptExecutionScope::init(
        &mut storage_mut.scope.raw,
        isolate,
      );
    }
    PinnedBox(storage)
  }

  unsafe fn deinit(me: &mut Self) {
    unsafe { raw::v8__AllowJavascriptExecutionScope__DESTRUCT(&mut me.raw) };
  }
}

impl<'s, P: GetIsolate> sealed::Sealed
  for AllowJavascriptExecutionScope<'s, P>
{
}
impl<'s, P: Scope + GetIsolate> Scope for AllowJavascriptExecutionScope<'s, P> {}

impl<'s, P: NewAllowJavascriptExecutionScope<'s>>
  AllowJavascriptExecutionScope<'s, P>
{
  pub fn new(param: P) -> ScopeStorage<P::NewScope> {
    ScopeStorage::new(P::make_new_scope(param))
  }
}

pub trait NewAllowJavascriptExecutionScope<'s> {
  type NewScope: Scope;
  fn make_new_scope(me: Self) -> Self::NewScope;
}

impl<'s, 'i, C> NewAllowJavascriptExecutionScope<'s>
  for PinnedRef<'s, HandleScope<'i, C>>
{
  type NewScope = AllowJavascriptExecutionScope<'s, HandleScope<'i, C>>;
  fn make_new_scope(me: Self) -> Self::NewScope {
    AllowJavascriptExecutionScope {
      raw: unsafe { raw::AllowJavascriptExecutionScope::uninit() },
      scope: me,
      _pinned: PhantomPinned,
    }
  }
}

impl<'borrow, 's, P> NewAllowJavascriptExecutionScope<'borrow>
  for &'borrow mut PinnedRef<'s, P>
where
  PinnedRef<'borrow, P>: NewAllowJavascriptExecutionScope<'borrow>,
{
  type NewScope = <PinnedRef<'borrow, P> as NewAllowJavascriptExecutionScope<
    'borrow,
  >>::NewScope;
  fn make_new_scope(me: Self) -> Self::NewScope {
    PinnedRef::<'borrow, P>::make_new_scope(me.as_mut())
  }
}

#[allow(unused_macros)]
#[macro_export]
macro_rules! make_callback_scope {
  (unsafe $scope: ident, $param: expr) => {
    let $scope = std::pin::pin!(unsafe { $crate::CallbackScope::new($param) });
    let $scope = &mut $scope.init();
  };
}

#[allow(unused_imports)]
pub(crate) use make_callback_scope;

#[allow(unused_macros)]
#[macro_export]
macro_rules! make_handle_scope {
  ($scope: ident, $param: expr) => {
    let $scope = std::pin::pin!($crate::HandleScope::new($param));
    let $scope = &mut $scope.init();
  };
}

#[allow(unused_imports)]
pub(crate) use make_handle_scope;

#[repr(transparent)]
pub struct PinnedRef<'p, T>(Pin<&'p mut T>);

impl<'p, T> From<Pin<&'p mut T>> for PinnedRef<'p, T> {
  fn from(value: Pin<&'p mut T>) -> Self {
    PinnedRef(value)
  }
}

impl<'p, T> PinnedRef<'p, T> {
  pub fn as_mut(&mut self) -> PinnedRef<'_, T> {
    PinnedRef(self.0.as_mut())
  }
}

impl<'p, 'i> Deref for PinnedRef<'p, HandleScope<'i>> {
  type Target = PinnedRef<'p, HandleScope<'i, ()>>;
  fn deref(&self) -> &Self::Target {
    unsafe {
      std::mem::transmute::<
        &PinnedRef<'p, HandleScope<'i>>,
        &PinnedRef<'p, HandleScope<'i, ()>>,
      >(self)
    }
  }
}

impl<'p, 'i> DerefMut for PinnedRef<'p, HandleScope<'i>> {
  fn deref_mut(&mut self) -> &mut Self::Target {
    unsafe { std::mem::transmute(self) }
  }
}

impl<'p, 'i> Deref for PinnedRef<'p, HandleScope<'i, ()>> {
  type Target = Isolate;
  fn deref(&self) -> &Self::Target {
    unsafe {
      std::mem::transmute::<&NonNull<RealIsolate>, &Isolate>(&self.0.isolate)
    }
  }
}

impl<'p, 'i> DerefMut for PinnedRef<'p, HandleScope<'i, ()>> {
  fn deref_mut(&mut self) -> &mut Self::Target {
    unsafe {
      std::mem::transmute(&mut self.0.as_mut().get_unchecked_mut().isolate)
    }
  }
}

impl<'p, 'i> Deref for PinnedRef<'p, CallbackScope<'i>> {
  type Target = PinnedRef<'p, HandleScope<'i>>;
  fn deref(&self) -> &Self::Target {
    unsafe { std::mem::transmute(self) }
  }
}

impl<'p, 'i> DerefMut for PinnedRef<'p, CallbackScope<'i>> {
  fn deref_mut(&mut self) -> &mut Self::Target {
    unsafe { std::mem::transmute(self) }
  }
}

impl<'p, 'i> Deref for PinnedRef<'p, CallbackScope<'i, ()>> {
  type Target = PinnedRef<'p, HandleScope<'i, ()>>;
  fn deref(&self) -> &Self::Target {
    unsafe { std::mem::transmute(self) }
  }
}

impl<'p, 'i> DerefMut for PinnedRef<'p, CallbackScope<'i, ()>> {
  fn deref_mut(&mut self) -> &mut Self::Target {
    unsafe { std::mem::transmute(self) }
  }
}

impl<'p, 'i, C> From<PinnedRef<'p, CallbackScope<'i, C>>>
  for PinnedRef<'p, HandleScope<'i, C>>
{
  fn from(value: PinnedRef<'p, CallbackScope<'i, C>>) -> Self {
    unsafe { std::mem::transmute(value) }
  }
}

impl<'p, 's, 'i, C> Deref for PinnedRef<'p, TryCatch<'s, HandleScope<'i, C>>> {
  type Target = PinnedRef<'s, HandleScope<'i, C>>;
  fn deref(&self) -> &Self::Target {
    &self.0.scope
  }
}

impl<'p, 's, 'i, C> DerefMut
  for PinnedRef<'p, TryCatch<'s, HandleScope<'i, C>>>
{
  fn deref_mut(&mut self) -> &mut Self::Target {
    unsafe { &mut self.0.as_mut().get_unchecked_mut().scope }
  }
}

impl<'p, 's, P> Deref
  for PinnedRef<'p, DisallowJavascriptExecutionScope<'s, P>>
{
  type Target = PinnedRef<'s, P>;
  fn deref(&self) -> &Self::Target {
    &self.0.scope
  }
}

impl<'p, 's, P> DerefMut
  for PinnedRef<'p, DisallowJavascriptExecutionScope<'s, P>>
{
  fn deref_mut(&mut self) -> &mut Self::Target {
    unsafe { &mut self.0.as_mut().get_unchecked_mut().scope }
  }
}

impl<'p, 's, P> Deref for PinnedRef<'p, AllowJavascriptExecutionScope<'s, P>> {
  type Target = PinnedRef<'s, P>;
  fn deref(&self) -> &Self::Target {
    &self.0.scope
  }
}
impl<'p, 's, P> DerefMut
  for PinnedRef<'p, AllowJavascriptExecutionScope<'s, P>>
{
  fn deref_mut(&mut self) -> &mut Self::Target {
    unsafe { &mut self.0.as_mut().get_unchecked_mut().scope }
  }
}

impl<'s, P> GetIsolate for PinnedRef<'s, P>
where
  P: GetIsolate,
{
  fn get_isolate_ptr(&self) -> *mut RealIsolate {
    self.0.get_isolate_ptr()
  }
}

impl<'s, 'i, C> AsRef<Isolate> for PinnedRef<'s, HandleScope<'i, C>> {
  fn as_ref(&self) -> &Isolate {
    unsafe { Isolate::from_raw_ref(&self.0.isolate) }
  }
}

impl<'s, 'i, C> AsRef<Isolate> for PinnedRef<'s, CallbackScope<'i, C>> {
  fn as_ref(&self) -> &Isolate {
    unsafe { Isolate::from_raw_ref(&self.0.isolate) }
  }
}

impl<'s, 'i, C> AsRef<Isolate>
  for PinnedRef<'s, TryCatch<'i, HandleScope<'i, C>>>
{
  fn as_ref(&self) -> &Isolate {
    unsafe { Isolate::from_raw_ref(&self.0.scope.0.isolate) }
  }
}

impl<'s, 'i, C> AsRef<Isolate>
  for PinnedRef<'s, DisallowJavascriptExecutionScope<'i, HandleScope<'i, C>>>
{
  fn as_ref(&self) -> &Isolate {
    unsafe { Isolate::from_raw_ref(&self.0.scope.0.isolate) }
  }
}

impl<'s, 'i, C> AsRef<Isolate>
  for PinnedRef<'s, AllowJavascriptExecutionScope<'i, HandleScope<'i, C>>>
{
  fn as_ref(&self) -> &Isolate {
    unsafe { Isolate::from_raw_ref(&self.0.scope.0.isolate) }
  }
}

impl<'p, 's, 'esc> AsRef<Isolate>
  for PinnedRef<'p, EscapableHandleScope<'s, 'esc>>
{
  fn as_ref(&self) -> &Isolate {
    unsafe { Isolate::from_raw_ref(&self.0.isolate) }
  }
}

impl<'s, 'i, C> AsRef<Isolate> for ContextScope<'s, HandleScope<'i, C>> {
  fn as_ref(&self) -> &Isolate {
    unsafe { Isolate::from_raw_ref(&self.scope.0.isolate) }
  }
}
impl<'s, 'i, C> AsRef<Isolate> for ContextScope<'s, CallbackScope<'i, C>> {
  fn as_ref(&self) -> &Isolate {
    unsafe { Isolate::from_raw_ref(&self.scope.0.isolate) }
  }
}

// WIP
/*
#[cfg(test)]
mod tests {
  use super::*;
  use crate::ContextOptions;
  use crate::Global;
  use std::any::type_name;
  use std::pin::pin;

  trait SameType {}
  impl<A> SameType for (A, A) {}

  /// `AssertTypeOf` facilitates comparing types. The important difference with
  /// assigning a value to a variable with an explicitly stated type is that the
  /// latter allows coercions and dereferencing to change the type, whereas
  /// `AssertTypeOf` requires the compared types to match exactly.
  struct AssertTypeOf<'a, T>(#[allow(dead_code)] &'a T);
  impl<T> AssertTypeOf<'_, T> {
    pub fn is<A>(self)
    where
      (A, T): SameType,
    {
      assert_eq!(type_name::<A>(), type_name::<T>());
    }
  }

  #[test]
  fn deref_types() {
    crate::initialize_v8();
    let isolate = &mut Isolate::new(Default::default());
    AssertTypeOf(isolate).is::<OwnedIsolate>();
    let l1_hs = pin!(HandleScope::new(isolate));
    let l1_hs = &mut l1_hs.init();
    AssertTypeOf(l1_hs).is::<PinnedRef<HandleScope<()>>>();
    let context = Context::new(l1_hs, ContextOptions::default());
    {
      let l2_cxs = &mut ContextScope::new(l1_hs, context);
      AssertTypeOf(l2_cxs).is::<ContextScope<HandleScope>>();
      {
        let d = l2_cxs.deref_mut();
        AssertTypeOf(d).is::<PinnedRef<HandleScope>>();
        let d = d.deref_mut();
        AssertTypeOf(d).is::<PinnedRef<HandleScope<()>>>();
        let d = d.deref_mut();
        AssertTypeOf(d).is::<Isolate>();
      }
      {
        let l3_tc = &mut TryCatch::new(&mut **l2_cxs);
        AssertTypeOf(l3_tc).is::<TryCatch<HandleScope>>();
        let d = l3_tc.deref_mut();
        AssertTypeOf(d).is::<HandleScope>();
        let d = d.deref_mut();
        AssertTypeOf(d).is::<HandleScope<()>>();
        let d = d.deref_mut();
        AssertTypeOf(d).is::<Isolate>();
      }
      {
        let l3_djses = &mut DisallowJavascriptExecutionScope::new(
          l2_cxs,
          OnFailure::CrashOnFailure,
        );
        AssertTypeOf(l3_djses)
          .is::<DisallowJavascriptExecutionScope<HandleScope>>();
        let d = l3_djses.deref_mut();
        AssertTypeOf(d).is::<HandleScope>();
        let d = d.deref_mut();
        AssertTypeOf(d).is::<HandleScope<()>>();
        let d = d.deref_mut();
        AssertTypeOf(d).is::<Isolate>();
        {
          let l4_ajses = &mut AllowJavascriptExecutionScope::new(l3_djses);
          AssertTypeOf(l4_ajses).is::<HandleScope>();
          let d = l4_ajses.deref_mut();
          AssertTypeOf(d).is::<HandleScope<()>>();
          let d = d.deref_mut();
          AssertTypeOf(d).is::<Isolate>();
        }
      }
      {
        let l3_ehs = &mut EscapableHandleScope::new(l2_cxs);
        AssertTypeOf(l3_ehs).is::<EscapableHandleScope>();
        {
          let l4_cxs = &mut ContextScope::new(l3_ehs, context);
          AssertTypeOf(l4_cxs).is::<ContextScope<EscapableHandleScope>>();
          let d = l4_cxs.deref_mut();
          AssertTypeOf(d).is::<EscapableHandleScope>();
          let d = d.deref_mut();
          AssertTypeOf(d).is::<HandleScope>();
          let d = d.deref_mut();
          AssertTypeOf(d).is::<HandleScope<()>>();
          let d = d.deref_mut();
          AssertTypeOf(d).is::<Isolate>();
        }
        {
          let l4_tc = &mut TryCatch::new(l3_ehs);
          AssertTypeOf(l4_tc).is::<TryCatch<EscapableHandleScope>>();
          let d = l4_tc.deref_mut();
          AssertTypeOf(d).is::<EscapableHandleScope>();
          let d = d.deref_mut();
          AssertTypeOf(d).is::<HandleScope>();
          let d = d.deref_mut();
          AssertTypeOf(d).is::<HandleScope<()>>();
          let d = d.deref_mut();
          AssertTypeOf(d).is::<Isolate>();
        }
        {
          let l4_djses = &mut DisallowJavascriptExecutionScope::new(
            l3_ehs,
            OnFailure::CrashOnFailure,
          );
          AssertTypeOf(l4_djses)
            .is::<DisallowJavascriptExecutionScope<EscapableHandleScope>>();
          let d = l4_djses.deref_mut();
          AssertTypeOf(d).is::<EscapableHandleScope>();
          let d = d.deref_mut();
          AssertTypeOf(d).is::<HandleScope>();
          let d = d.deref_mut();
          AssertTypeOf(d).is::<HandleScope<()>>();
          let d = d.deref_mut();
          AssertTypeOf(d).is::<Isolate>();
          {
            let l5_ajses = &mut AllowJavascriptExecutionScope::new(l4_djses);
            AssertTypeOf(l5_ajses).is::<EscapableHandleScope>();
            let d = l5_ajses.deref_mut();
            AssertTypeOf(d).is::<HandleScope>();
            let d = d.deref_mut();
            AssertTypeOf(d).is::<HandleScope<()>>();
            let d = d.deref_mut();
            AssertTypeOf(d).is::<Isolate>();
          }
        }
      }
    }
    {
      let l2_tc = &mut TryCatch::new(l1_hs);
      AssertTypeOf(l2_tc).is::<TryCatch<HandleScope<()>>>();
      let d = l2_tc.deref_mut();
      AssertTypeOf(d).is::<HandleScope<()>>();
      let d = d.deref_mut();
      AssertTypeOf(d).is::<Isolate>();
      {
        let l3_djses = &mut DisallowJavascriptExecutionScope::new(
          l2_tc,
          OnFailure::CrashOnFailure,
        );
        AssertTypeOf(l3_djses)
          .is::<DisallowJavascriptExecutionScope<TryCatch<HandleScope<()>>>>();
        let d = l3_djses.deref_mut();
        AssertTypeOf(d).is::<TryCatch<HandleScope<()>>>();
        let d = d.deref_mut();
        AssertTypeOf(d).is::<HandleScope<()>>();
        let d = d.deref_mut();
        AssertTypeOf(d).is::<Isolate>();
        {
          let l4_ajses = &mut AllowJavascriptExecutionScope::new(l3_djses);
          AssertTypeOf(l4_ajses).is::<TryCatch<HandleScope<()>>>();
          let d = l4_ajses.deref_mut();
          AssertTypeOf(d).is::<HandleScope<()>>();
          let d = d.deref_mut();
          AssertTypeOf(d).is::<Isolate>();
        }
      }
    }
    {
      let l2_ehs = &mut EscapableHandleScope::new(l1_hs);
      AssertTypeOf(l2_ehs).is::<EscapableHandleScope<()>>();
      let l3_tc = &mut TryCatch::new(l2_ehs);
      AssertTypeOf(l3_tc).is::<TryCatch<EscapableHandleScope<()>>>();
      let d = l3_tc.deref_mut();
      AssertTypeOf(d).is::<EscapableHandleScope<()>>();
      let d = d.deref_mut();
      AssertTypeOf(d).is::<HandleScope<()>>();
      let d = d.deref_mut();
      AssertTypeOf(d).is::<Isolate>();
    }
    {
      // `CallbackScope` is meant to be used inside V8 API callback functions
      // only. It assumes that a `HandleScope` already exists on the stack, and
      // that a context has been entered. Push a `ContextScope` onto the stack
      // to also meet the second expectation.
      let _ = ContextScope::new(l1_hs, context);
      let l2_cbs = &mut unsafe { CallbackScope::new(context) };
      AssertTypeOf(l2_cbs).is::<CallbackScope>();
      let d = l2_cbs.deref_mut();
      AssertTypeOf(d).is::<HandleScope>();
      let d = d.deref_mut();
      AssertTypeOf(d).is::<HandleScope<()>>();
      let d = d.deref_mut();
      AssertTypeOf(d).is::<Isolate>();
    }
    {
      let isolate: &mut Isolate = l1_hs.as_mut();
      let l2_cbs = &mut unsafe { CallbackScope::new(isolate) };
      AssertTypeOf(l2_cbs).is::<CallbackScope<()>>();
      let d = l2_cbs.deref_mut();
      AssertTypeOf(d).is::<HandleScope<()>>();
      let d = d.deref_mut();
      AssertTypeOf(d).is::<Isolate>();
    }
  }

  #[test]
  fn new_scope_types() {
    crate::initialize_v8();
    let isolate = &mut Isolate::new(Default::default());
    AssertTypeOf(isolate).is::<OwnedIsolate>();
    let global_context: Global<Context>;
    {
      let l1_hs = &mut HandleScope::new(isolate);
      AssertTypeOf(l1_hs).is::<HandleScope<()>>();
      let context = Context::new(l1_hs, Default::default());
      global_context = Global::new(l1_hs, context);
      AssertTypeOf(&HandleScope::new(l1_hs)).is::<HandleScope<()>>();
      {
        let l2_cxs = &mut ContextScope::new(l1_hs, context);
        AssertTypeOf(l2_cxs).is::<ContextScope<HandleScope>>();
        AssertTypeOf(&ContextScope::new(l2_cxs, context))
          .is::<ContextScope<HandleScope>>();
        AssertTypeOf(&HandleScope::new(l2_cxs)).is::<HandleScope>();
        AssertTypeOf(&EscapableHandleScope::new(l2_cxs))
          .is::<EscapableHandleScope>();
        AssertTypeOf(&TryCatch::new(l2_cxs)).is::<TryCatch<HandleScope>>();
      }
      {
        let l2_ehs = &mut EscapableHandleScope::new(l1_hs);
        AssertTypeOf(l2_ehs).is::<EscapableHandleScope<()>>();
        AssertTypeOf(&HandleScope::new(l2_ehs))
          .is::<EscapableHandleScope<()>>();
        AssertTypeOf(&EscapableHandleScope::new(l2_ehs))
          .is::<EscapableHandleScope<()>>();
        {
          let l3_cxs = &mut ContextScope::new(l2_ehs, context);
          AssertTypeOf(l3_cxs).is::<ContextScope<EscapableHandleScope>>();
          AssertTypeOf(&ContextScope::new(l3_cxs, context))
            .is::<ContextScope<EscapableHandleScope>>();
          AssertTypeOf(&HandleScope::new(l3_cxs)).is::<EscapableHandleScope>();
          AssertTypeOf(&EscapableHandleScope::new(l3_cxs))
            .is::<EscapableHandleScope>();
          {
            let l4_tc = &mut TryCatch::new(l3_cxs);
            AssertTypeOf(l4_tc).is::<TryCatch<EscapableHandleScope>>();
            AssertTypeOf(&ContextScope::new(l4_tc, context))
              .is::<ContextScope<EscapableHandleScope>>();
            AssertTypeOf(&HandleScope::new(l4_tc)).is::<EscapableHandleScope>();
            AssertTypeOf(&EscapableHandleScope::new(l4_tc))
              .is::<EscapableHandleScope>();
            AssertTypeOf(&TryCatch::new(l4_tc))
              .is::<TryCatch<EscapableHandleScope>>();
          }
        }
        {
          let l3_tc = &mut TryCatch::new(l2_ehs);
          AssertTypeOf(l3_tc).is::<TryCatch<EscapableHandleScope<()>>>();
          AssertTypeOf(&ContextScope::new(l3_tc, context))
            .is::<ContextScope<EscapableHandleScope>>();
          AssertTypeOf(&HandleScope::new(l3_tc))
            .is::<EscapableHandleScope<()>>();
          AssertTypeOf(&EscapableHandleScope::new(l3_tc))
            .is::<EscapableHandleScope<()>>();
          AssertTypeOf(&TryCatch::new(l3_tc))
            .is::<TryCatch<EscapableHandleScope<()>>>();
        }
      }
      {
        let l2_tc = &mut TryCatch::new(l1_hs);
        AssertTypeOf(l2_tc).is::<TryCatch<HandleScope<()>>>();
        AssertTypeOf(&ContextScope::new(l2_tc, context))
          .is::<ContextScope<HandleScope>>();
        AssertTypeOf(&HandleScope::new(l2_tc)).is::<HandleScope<()>>();
        AssertTypeOf(&EscapableHandleScope::new(l2_tc))
          .is::<EscapableHandleScope<()>>();
        AssertTypeOf(&TryCatch::new(l2_tc)).is::<TryCatch<HandleScope<()>>>();
      }
      {
        let l2_cbs = &mut unsafe { CallbackScope::new(context) };
        AssertTypeOf(l2_cbs).is::<CallbackScope>();
        AssertTypeOf(&ContextScope::new(l2_cbs, context))
          .is::<ContextScope<HandleScope>>();
        {
          let l3_hs = &mut HandleScope::new(l2_cbs);
          AssertTypeOf(l3_hs).is::<HandleScope>();
          AssertTypeOf(&ContextScope::new(l3_hs, context))
            .is::<ContextScope<HandleScope>>();
          AssertTypeOf(&HandleScope::new(l3_hs)).is::<HandleScope>();
          AssertTypeOf(&EscapableHandleScope::new(l3_hs))
            .is::<EscapableHandleScope>();
          AssertTypeOf(&TryCatch::new(l3_hs)).is::<TryCatch<HandleScope>>();
        }
        {
          let l3_ehs = &mut EscapableHandleScope::new(l2_cbs);
          AssertTypeOf(l3_ehs).is::<EscapableHandleScope>();
          AssertTypeOf(&ContextScope::new(l3_ehs, context))
            .is::<ContextScope<EscapableHandleScope>>();
          AssertTypeOf(&HandleScope::new(l3_ehs)).is::<EscapableHandleScope>();
          AssertTypeOf(&EscapableHandleScope::new(l3_ehs))
            .is::<EscapableHandleScope>();
          AssertTypeOf(&TryCatch::new(l3_ehs))
            .is::<TryCatch<EscapableHandleScope>>();
        }
        {
          let l3_tc = &mut TryCatch::new(l2_cbs);
          AssertTypeOf(l3_tc).is::<TryCatch<HandleScope>>();
          AssertTypeOf(&ContextScope::new(l3_tc, context))
            .is::<ContextScope<HandleScope>>();
          AssertTypeOf(&HandleScope::new(l3_tc)).is::<HandleScope>();
          AssertTypeOf(&EscapableHandleScope::new(l3_tc))
            .is::<EscapableHandleScope>();
          AssertTypeOf(&TryCatch::new(l3_tc)).is::<TryCatch<HandleScope>>();
        }
      }
    }
    {
      let l1_cbs = &mut unsafe { CallbackScope::new(&mut *isolate) };
      AssertTypeOf(l1_cbs).is::<CallbackScope<()>>();
      let context = Context::new(l1_cbs, Default::default());
      AssertTypeOf(&ContextScope::new(l1_cbs, context))
        .is::<ContextScope<HandleScope>>();
      AssertTypeOf(&HandleScope::new(l1_cbs)).is::<HandleScope<()>>();
      AssertTypeOf(&EscapableHandleScope::new(l1_cbs))
        .is::<EscapableHandleScope<()>>();
      AssertTypeOf(&TryCatch::new(l1_cbs)).is::<TryCatch<HandleScope<()>>>();
    }
    {
      AssertTypeOf(&HandleScope::with_context(isolate, &global_context))
        .is::<HandleScope>();
      AssertTypeOf(&HandleScope::with_context(isolate, global_context))
        .is::<HandleScope>();
    }
  }
}
*/
