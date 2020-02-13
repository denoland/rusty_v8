// Copyright 2019-2020 the Deno authors. All rights reserved. MIT license.

use crate::scope::Entered;
use crate::scope::Escapable;
use crate::CallbackScope;
use crate::Context;
use crate::ContextScope;
use crate::EscapableHandleScope;
use crate::FunctionCallbackInfo;
use crate::HandleScope;
use crate::Isolate;
use crate::Local;
use crate::Message;
use crate::Object;
use crate::PropertyCallbackInfo;

pub(crate) mod internal {
  use super::*;

  extern "C" {
    fn v8__Context__GetIsolate(self_: &Context) -> *mut Isolate;
    fn v8__EscapableHandleScope__GetIsolate(
      self_: &EscapableHandleScope,
    ) -> *mut Isolate;
    fn v8__FunctionCallbackInfo__GetIsolate(
      self_: &FunctionCallbackInfo,
    ) -> *mut Isolate;
    fn v8__HandleScope__GetIsolate(self_: &HandleScope) -> *mut Isolate;
    fn v8__Message__GetIsolate(self_: &Message) -> *mut Isolate;
    fn v8__Object__GetIsolate(self_: &Object) -> *mut Isolate;
    fn v8__PropertyCallbackInfo__GetIsolate(
      self_: &PropertyCallbackInfo,
    ) -> *mut Isolate;
  }

  /// Internal trait for retrieving a raw Isolate pointer from various V8
  /// API objects.
  pub trait GetRawIsolate {
    fn get_raw_isolate(&self) -> *mut Isolate;
  }

  impl<'s, T> GetRawIsolate for Local<'s, T>
  where
    Local<'s, Object>: From<Local<'s, T>>,
  {
    fn get_raw_isolate(&self) -> *mut Isolate {
      let local = Local::<'s, Object>::from(*self);
      (&*local).get_raw_isolate()
    }
  }

  impl<'s> GetRawIsolate for Local<'s, Context> {
    fn get_raw_isolate(&self) -> *mut Isolate {
      (&**self).get_raw_isolate()
    }
  }

  impl<'s> GetRawIsolate for Local<'s, Message> {
    fn get_raw_isolate(&self) -> *mut Isolate {
      (&**self).get_raw_isolate()
    }
  }

  impl<'s, S> GetRawIsolate for Entered<'_, S>
  where
    S: GetRawIsolate,
  {
    fn get_raw_isolate(&self) -> *mut Isolate {
      self.data().get_raw_isolate()
    }
  }

  impl<X> GetRawIsolate for CallbackScope<X> {
    fn get_raw_isolate(&self) -> *mut Isolate {
      self.get_raw_isolate_()
    }
  }

  impl<'s> GetRawIsolate for ContextScope {
    fn get_raw_isolate(&self) -> *mut Isolate {
      unsafe { self.get_captured_context() }.get_raw_isolate()
    }
  }

  impl GetRawIsolate for Context {
    fn get_raw_isolate(&self) -> *mut Isolate {
      unsafe { v8__Context__GetIsolate(self) }
    }
  }

  impl GetRawIsolate for EscapableHandleScope {
    fn get_raw_isolate(&self) -> *mut Isolate {
      unsafe { v8__EscapableHandleScope__GetIsolate(self) }
    }
  }

  impl GetRawIsolate for FunctionCallbackInfo {
    fn get_raw_isolate(&self) -> *mut Isolate {
      unsafe { v8__FunctionCallbackInfo__GetIsolate(self) }
    }
  }

  impl GetRawIsolate for HandleScope {
    fn get_raw_isolate(&self) -> *mut Isolate {
      unsafe { v8__HandleScope__GetIsolate(self) }
    }
  }

  impl GetRawIsolate for Message {
    fn get_raw_isolate(&self) -> *mut Isolate {
      unsafe { v8__Message__GetIsolate(self) }
    }
  }

  impl GetRawIsolate for Object {
    fn get_raw_isolate(&self) -> *mut Isolate {
      unsafe { v8__Object__GetIsolate(self) }
    }
  }

  impl GetRawIsolate for PropertyCallbackInfo {
    fn get_raw_isolate(&self) -> *mut Isolate {
      unsafe { v8__PropertyCallbackInfo__GetIsolate(self) }
    }
  }
}

/// Trait for retrieving the current isolate from a scope object.
pub trait InIsolate {
  // Do not implement this trait on unscoped Isolate references
  // (e.g. OwnedIsolate) or on shared references *e.g. &Isolate).
  fn isolate(&mut self) -> &mut Isolate;
}

impl<'s, S, P> InIsolate for Entered<'s, S, P>
where
  S: internal::GetRawIsolate,
{
  fn isolate(&mut self) -> &mut Isolate {
    unsafe { &mut *(self.data().get_raw_isolate()) }
  }
}

extern "C" {
  fn v8__Isolate__GetCurrentContext(this: *mut Isolate) -> *mut Context;
  fn v8__Isolate__GetEnteredOrMicrotaskContext(
    this: *mut Isolate,
  ) -> *mut Context;
}

/// When scope implements this trait, this means that Local handles can be
/// created inside it.
pub trait ToLocal<'s>: InIsolate {
  unsafe fn to_local<T>(&mut self, ptr: *mut T) -> Option<Local<'s, T>> {
    Local::from_raw(ptr)
  }

  fn get_current_context(&mut self) -> Option<Local<'s, Context>> {
    unsafe { Local::from_raw(v8__Isolate__GetCurrentContext(self.isolate())) }
  }

  fn get_entered_or_microtask_context(&mut self) -> Option<Local<'s, Context>> {
    unsafe {
      Local::from_raw(v8__Isolate__GetEnteredOrMicrotaskContext(self.isolate()))
    }
  }
}

impl<'s> ToLocal<'s> for Entered<'s, FunctionCallbackInfo> {}
impl<'s> ToLocal<'s> for Entered<'s, PropertyCallbackInfo> {}
impl<'s, P> ToLocal<'s> for Entered<'s, HandleScope, P> {}
impl<'s, P> ToLocal<'s> for Entered<'s, EscapableHandleScope, P> {}
impl<'s, 'p: 's, P> ToLocal<'p> for Entered<'s, ContextScope, P> where
  P: ToLocal<'p>
{
}

pub trait ToLocalOrReturnsLocal<'s>: InIsolate {}
impl<'s, E> ToLocalOrReturnsLocal<'s> for E where E: ToLocal<'s> {}
impl<'s, 'p: 's> ToLocalOrReturnsLocal<'p>
  for Entered<'s, CallbackScope<Escapable>>
{
}

pub trait EscapeLocal<'s, 'p: 's>: ToLocal<'s> {
  fn escape<T>(&mut self, local: Local<T>) -> Local<'p, T>;
}

impl<'s, 'p: 's, P> EscapeLocal<'s, 'p> for Entered<'s, EscapableHandleScope, P>
where
  P: ToLocalOrReturnsLocal<'p>,
{
  fn escape<T>(&mut self, local: Local<T>) -> Local<'p, T> {
    unsafe { self.data_mut().escape(local) }
  }
}

impl<'s, 'p: 's, P> EscapeLocal<'s, 'p> for Entered<'s, ContextScope, P>
where
  P: EscapeLocal<'s, 'p>,
{
  fn escape<T>(&mut self, local: Local<T>) -> Local<'p, T> {
    self.parent_mut().escape(local)
  }
}

impl<'s, 'p: 's, P> EscapeLocal<'s, 'p> for Entered<'s, HandleScope, P>
where
  P: EscapeLocal<'s, 'p>,
{
  fn escape<T>(&mut self, local: Local<T>) -> Local<'p, T> {
    self.parent_mut().escape(local)
  }
}

// TODO(piscisaureus): move the impls for Entered to a more sensible spot.

impl<'s, S, P> Entered<'s, S, P>
where
  Self: InIsolate,
{
  pub fn isolate(&mut self) -> &mut Isolate {
    <Self as InIsolate>::isolate(self)
  }
}

impl<'s, 'p: 's, S, P> Entered<'s, S, P>
where
  Self: ToLocal<'p>,
{
  /// Returns the context of the currently running JavaScript, or the context
  /// on the top of the stack if no JavaScript is running.
  pub fn get_current_context(&mut self) -> Option<Local<'p, Context>> {
    <Self as ToLocal<'p>>::get_current_context(self)
  }

  /// Returns either the last context entered through V8's C++ API, or the
  /// context of the currently running microtask while processing microtasks.
  /// If a context is entered while executing a microtask, that context is
  /// returned.
  pub fn get_entered_or_microtask_context(
    &mut self,
  ) -> Option<Local<'p, Context>> {
    <Self as ToLocal<'p>>::get_entered_or_microtask_context(self)
  }
}

impl<'s, 'p: 's, S, P> Entered<'s, S, P>
where
  Self: EscapeLocal<'s, 'p>,
{
  /// Pushes the value into the previous scope and returns a handle to it.
  /// Cannot be called twice.
  pub fn escape<T>(&mut self, local: Local<T>) -> Local<'p, T> {
    <Self as EscapeLocal<'s, 'p>>::escape(self, local)
  }
}
