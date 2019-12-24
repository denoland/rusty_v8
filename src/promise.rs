use std::marker::PhantomData;

use crate::support::MaybeBool;
use crate::support::Opaque;
use crate::Context;
use crate::Function;
use crate::Local;
use crate::ToLocal;
use crate::Value;

extern "C" {
  fn v8__Promise__Resolver__New(context: *mut Context) -> *mut PromiseResolver;
  fn v8__Promise__Resolver__GetPromise(
    resolver: *mut PromiseResolver,
  ) -> *mut Promise;
  fn v8__Promise__Resolver__Resolve(
    resolver: *mut PromiseResolver,
    context: *mut Context,
    value: *mut Value,
  ) -> MaybeBool;
  fn v8__Promise__Resolver__Reject(
    resolver: *mut PromiseResolver,
    context: *mut Context,
    value: *mut Value,
  ) -> MaybeBool;
  fn v8__Promise__State(promise: *mut Promise) -> PromiseState;
  fn v8__Promise__HasHandler(promise: *mut Promise) -> bool;
  fn v8__Promise__Result(promise: *mut Promise) -> *mut Value;
  fn v8__Promise__Catch(
    promise: *mut Promise,
    context: *mut Context,
    handler: *mut Function,
  ) -> *mut Promise;
  fn v8__Promise__Then(
    promise: *mut Promise,
    context: *mut Context,
    handler: *mut Function,
  ) -> *mut Promise;
  fn v8__Promise__Then2(
    promise: *mut Promise,
    context: *mut Context,
    on_fulfilled: *mut Function,
    on_rejected: *mut Function,
  ) -> *mut Promise;

  fn v8__PromiseRejectMessage__GetPromise(
    this: &PromiseRejectMessage,
  ) -> *mut Promise;
  fn v8__PromiseRejectMessage__GetValue(
    this: &PromiseRejectMessage,
  ) -> *mut Value;
  fn v8__PromiseRejectMessage__GetEvent(
    this: &PromiseRejectMessage,
  ) -> PromiseRejectEvent;
}

#[derive(Debug, PartialEq)]
#[repr(C)]
pub enum PromiseState {
  Pending,
  Fulfilled,
  Rejected,
}

/// An instance of the built-in Promise constructor (ES6 draft).
#[repr(C)]
pub struct Promise(Opaque);

impl Promise {
  /// Returns the value of the [[PromiseState]] field.
  pub fn state(&mut self) -> PromiseState {
    unsafe { v8__Promise__State(&mut *self) }
  }

  /// Returns true if the promise has at least one derived promise, and
  /// therefore resolve/reject handlers (including default handler).
  pub fn has_handler(&mut self) -> bool {
    unsafe { v8__Promise__HasHandler(&mut *self) }
  }

  /// Returns the content of the [[PromiseResult]] field. The Promise must not
  /// be pending.
  pub fn result<'sc>(
    &mut self,
    scope: &mut impl ToLocal<'sc>,
  ) -> Local<'sc, Value> {
    unsafe { scope.to_local(v8__Promise__Result(&mut *self)) }.unwrap()
  }

  /// Register a rejection handler with a promise.
  ///
  /// See `Self::then2`.
  pub fn catch<'sc>(
    &mut self,
    mut context: Local<'sc, Context>,
    mut handler: Local<'sc, Function>,
  ) -> Option<Local<'sc, Promise>> {
    unsafe {
      Local::from_raw_(v8__Promise__Catch(
        &mut *self,
        &mut *context,
        &mut *handler,
      ))
    }
  }

  /// Register a resolution handler with a promise.
  ///
  /// See `Self::then2`.
  pub fn then<'sc>(
    &mut self,
    mut context: Local<'sc, Context>,
    mut handler: Local<'sc, Function>,
  ) -> Option<Local<'sc, Promise>> {
    unsafe {
      Local::from_raw_(v8__Promise__Then(
        &mut *self,
        &mut *context,
        &mut *handler,
      ))
    }
  }

  /// Register a resolution/rejection handler with a promise.
  /// The handler is given the respective resolution/rejection value as
  /// an argument. If the promise is already resolved/rejected, the handler is
  /// invoked at the end of turn.
  pub fn then2<'sc>(
    &mut self,
    mut context: Local<'sc, Context>,
    mut on_fulfilled: Local<'sc, Function>,
    mut on_rejected: Local<'sc, Function>,
  ) -> Option<Local<'sc, Promise>> {
    unsafe {
      Local::from_raw_(v8__Promise__Then2(
        &mut *self,
        &mut *context,
        &mut *on_fulfilled,
        &mut *on_rejected,
      ))
    }
  }
}

#[repr(C)]
pub struct PromiseResolver(Opaque);

impl PromiseResolver {
  /// Create a new resolver, along with an associated promise in pending state.
  pub fn new<'sc>(
    scope: &mut impl ToLocal<'sc>,
    mut context: Local<'sc, Context>,
  ) -> Option<Local<'sc, PromiseResolver>> {
    unsafe { scope.to_local(v8__Promise__Resolver__New(&mut *context)) }
  }

  /// Extract the associated promise.
  pub fn get_promise<'sc>(
    &mut self,
    scope: &mut impl ToLocal<'sc>,
  ) -> Local<'sc, Promise> {
    unsafe { scope.to_local(v8__Promise__Resolver__GetPromise(&mut *self)) }
      .unwrap()
  }

  /// Resolve the associated promise with a given value.
  /// Ignored if the promise is no longer pending.
  pub fn resolve<'sc>(
    &mut self,
    mut context: Local<'sc, Context>,
    mut value: Local<'sc, Value>,
  ) -> Option<bool> {
    unsafe {
      v8__Promise__Resolver__Resolve(&mut *self, &mut *context, &mut *value)
        .into()
    }
  }

  /// Reject the associated promise with a given value.
  /// Ignored if the promise is no longer pending.
  pub fn reject<'sc>(
    &mut self,
    mut context: Local<'sc, Context>,
    mut value: Local<'sc, Value>,
  ) -> Option<bool> {
    unsafe {
      v8__Promise__Resolver__Reject(&mut *self, &mut *context, &mut *value)
        .into()
    }
  }
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)]
pub enum PromiseRejectEvent {
  PromiseRejectWithNoHandler,
  PromiseHandlerAddedAfterReject,
  PromiseRejectAfterResolved,
  PromiseResolveAfterResolved,
}

#[repr(C)]
pub struct PromiseRejectMessage<'msg>([usize; 3], PhantomData<&'msg ()>);

impl<'msg> PromiseRejectMessage<'msg> {
  pub fn get_promise(&self) -> Local<'msg, Promise> {
    unsafe {
      Local::from_raw_(v8__PromiseRejectMessage__GetPromise(self)).unwrap()
    }
  }

  pub fn get_event(&self) -> PromiseRejectEvent {
    unsafe { v8__PromiseRejectMessage__GetEvent(self) }
  }

  pub fn get_value(&self) -> Local<'msg, Value> {
    unsafe {
      Local::from_raw_(v8__PromiseRejectMessage__GetValue(self)).unwrap()
    }
  }
}
