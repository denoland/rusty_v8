use std::marker::PhantomData;

use crate::support::MaybeBool;
use crate::Context;
use crate::Function;
use crate::Local;
use crate::Promise;
use crate::PromiseResolver;
use crate::ToLocal;
use crate::Value;

extern "C" {
  fn v8__Promise__Resolver__New(
    context: *const Context,
  ) -> *const PromiseResolver;
  fn v8__Promise__Resolver__GetPromise(
    this: *const PromiseResolver,
  ) -> *const Promise;
  fn v8__Promise__Resolver__Resolve(
    this: *const PromiseResolver,
    context: *const Context,
    value: *const Value,
  ) -> MaybeBool;
  fn v8__Promise__Resolver__Reject(
    this: *const PromiseResolver,
    context: *const Context,
    value: *const Value,
  ) -> MaybeBool;
  fn v8__Promise__State(this: *const Promise) -> PromiseState;
  fn v8__Promise__HasHandler(this: *const Promise) -> bool;
  fn v8__Promise__Result(this: *const Promise) -> *const Value;
  fn v8__Promise__Catch(
    this: *const Promise,
    context: *const Context,
    handler: *const Function,
  ) -> *const Promise;
  fn v8__Promise__Then(
    this: *const Promise,
    context: *const Context,
    handler: *const Function,
  ) -> *const Promise;
  fn v8__Promise__Then2(
    this: *const Promise,
    context: *const Context,
    on_fulfilled: *const Function,
    on_rejected: *const Function,
  ) -> *const Promise;

  fn v8__PromiseRejectMessage__GetPromise(
    this: *const PromiseRejectMessage,
  ) -> *const Promise;
  fn v8__PromiseRejectMessage__GetValue(
    this: *const PromiseRejectMessage,
  ) -> *const Value;
  fn v8__PromiseRejectMessage__GetEvent(
    this: *const PromiseRejectMessage,
  ) -> PromiseRejectEvent;
}

#[derive(Debug, PartialEq)]
#[repr(C)]
pub enum PromiseState {
  Pending,
  Fulfilled,
  Rejected,
}

impl Promise {
  /// Returns the value of the [[PromiseState]] field.
  pub fn state(&self) -> PromiseState {
    unsafe { v8__Promise__State(&*self) }
  }

  /// Returns true if the promise has at least one derived promise, and
  /// therefore resolve/reject handlers (including default handler).
  pub fn has_handler(&self) -> bool {
    unsafe { v8__Promise__HasHandler(&*self) }
  }

  /// Returns the content of the [[PromiseResult]] field. The Promise must not
  /// be pending.
  pub fn result<'sc>(
    &self,
    scope: &mut impl ToLocal<'sc>,
  ) -> Local<'sc, Value> {
    unsafe { scope.to_local(v8__Promise__Result(&*self)) }.unwrap()
  }

  /// Register a rejection handler with a promise.
  ///
  /// See `Self::then2`.
  pub fn catch<'sc>(
    &self,
    context: Local<'sc, Context>,
    handler: Local<'sc, Function>,
  ) -> Option<Local<'sc, Promise>> {
    unsafe { Local::from_raw(v8__Promise__Catch(&*self, &*context, &*handler)) }
  }

  /// Register a resolution handler with a promise.
  ///
  /// See `Self::then2`.
  pub fn then<'sc>(
    &self,
    context: Local<'sc, Context>,
    handler: Local<'sc, Function>,
  ) -> Option<Local<'sc, Promise>> {
    unsafe { Local::from_raw(v8__Promise__Then(&*self, &*context, &*handler)) }
  }

  /// Register a resolution/rejection handler with a promise.
  /// The handler is given the respective resolution/rejection value as
  /// an argument. If the promise is already resolved/rejected, the handler is
  /// invoked at the end of turn.
  pub fn then2<'sc>(
    &self,
    context: Local<'sc, Context>,
    on_fulfilled: Local<'sc, Function>,
    on_rejected: Local<'sc, Function>,
  ) -> Option<Local<'sc, Promise>> {
    unsafe {
      Local::from_raw(v8__Promise__Then2(
        &*self,
        &*context,
        &*on_fulfilled,
        &*on_rejected,
      ))
    }
  }
}

impl PromiseResolver {
  /// Create a new resolver, along with an associated promise in pending state.
  pub fn new<'sc>(
    scope: &mut impl ToLocal<'sc>,
    context: Local<'sc, Context>,
  ) -> Option<Local<'sc, PromiseResolver>> {
    unsafe { scope.to_local(v8__Promise__Resolver__New(&*context)) }
  }

  /// Extract the associated promise.
  pub fn get_promise<'sc>(
    &self,
    scope: &mut impl ToLocal<'sc>,
  ) -> Local<'sc, Promise> {
    unsafe { scope.to_local(v8__Promise__Resolver__GetPromise(&*self)) }
      .unwrap()
  }

  /// Resolve the associated promise with a given value.
  /// Ignored if the promise is no longer pending.
  pub fn resolve<'sc>(
    &self,
    context: Local<'sc, Context>,
    value: Local<'sc, Value>,
  ) -> Option<bool> {
    unsafe { v8__Promise__Resolver__Resolve(&*self, &*context, &*value).into() }
  }

  /// Reject the associated promise with a given value.
  /// Ignored if the promise is no longer pending.
  pub fn reject<'sc>(
    &self,
    context: Local<'sc, Context>,
    value: Local<'sc, Value>,
  ) -> Option<bool> {
    unsafe { v8__Promise__Resolver__Reject(&*self, &*context, &*value).into() }
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
      Local::from_raw(v8__PromiseRejectMessage__GetPromise(self)).unwrap()
    }
  }

  pub fn get_event(&self) -> PromiseRejectEvent {
    unsafe { v8__PromiseRejectMessage__GetEvent(self) }
  }

  pub fn get_value(&self) -> Local<'msg, Value> {
    unsafe {
      Local::from_raw(v8__PromiseRejectMessage__GetValue(self)).unwrap()
    }
  }
}
