// Copyright 2019-2020 the Deno authors. All rights reserved. MIT license.

use crate::handle_scope::CxxEscapableHandleScope;
use crate::handle_scope::CxxHandleScope;
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
use crate::Locker;
use crate::Message;
use crate::Object;
use crate::PropertyCallbackInfo;
use crate::ReturnValue;

pub(crate) mod internal {
  use super::*;

  extern "C" {
    fn v8__Context__GetIsolate(self_: &Context) -> *mut Isolate;
    fn v8__EscapableHandleScope__GetIsolate(
      self_: &CxxEscapableHandleScope,
    ) -> *mut Isolate;
    fn v8__FunctionCallbackInfo__GetIsolate(
      self_: &FunctionCallbackInfo,
    ) -> *mut Isolate;
    fn v8__HandleScope__GetIsolate(self_: &CxxHandleScope) -> *mut Isolate;
    fn v8__Message__GetIsolate(self_: &Message) -> *mut Isolate;
    fn v8__Object__GetIsolate(self_: &Object) -> *mut Isolate;
    fn v8__PropertyCallbackInfo__GetIsolate(
      self_: &PropertyCallbackInfo,
    ) -> *mut Isolate;
    fn v8__ReturnValue__GetIsolate(self_: &ReturnValue) -> *mut Isolate;
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
      (&**self).get_raw_isolate()
    }
  }

  impl<X> GetRawIsolate for CallbackScope<X> {
    fn get_raw_isolate(&self) -> *mut Isolate {
      self.get_raw_isolate_()
    }
  }

  impl<'s, P> GetRawIsolate for ContextScope<P>
  where
    P: ToLocal<'s>,
  {
    fn get_raw_isolate(&self) -> *mut Isolate {
      self.get_captured_context().get_raw_isolate()
    }
  }

  impl GetRawIsolate for Locker {
    fn get_raw_isolate(&self) -> *mut Isolate {
      self.get_raw_isolate_()
    }
  }

  impl GetRawIsolate for Context {
    fn get_raw_isolate(&self) -> *mut Isolate {
      unsafe { v8__Context__GetIsolate(self) }
    }
  }

  impl<P> GetRawIsolate for EscapableHandleScope<P> {
    fn get_raw_isolate(&self) -> *mut Isolate {
      unsafe { v8__EscapableHandleScope__GetIsolate(self.inner()) }
    }
  }

  impl GetRawIsolate for FunctionCallbackInfo {
    fn get_raw_isolate(&self) -> *mut Isolate {
      unsafe { v8__FunctionCallbackInfo__GetIsolate(self) }
    }
  }

  impl<P> GetRawIsolate for HandleScope<P> {
    fn get_raw_isolate(&self) -> *mut Isolate {
      unsafe { v8__HandleScope__GetIsolate(self.inner()) }
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

  impl<'a> GetRawIsolate for ReturnValue<'a> {
    fn get_raw_isolate(&self) -> *mut Isolate {
      unsafe { v8__ReturnValue__GetIsolate(self) }
    }
  }
}

use internal::GetRawIsolate;

/// Trait for retrieving the current isolate from a scope object.
pub trait InIsolate {
  // Do not implement this trait on unscoped Isolate references
  // (e.g. OwnedIsolate) or on shared references *e.g. &Isolate).
  fn isolate(&mut self) -> &mut Isolate;
}

impl InIsolate for Locker {
  fn isolate(&mut self) -> &mut Isolate {
    unsafe { &mut *self.get_raw_isolate() }
  }
}

impl<'s, S> InIsolate for Entered<'s, S>
where
  S: internal::GetRawIsolate,
{
  fn isolate(&mut self) -> &mut Isolate {
    unsafe { &mut *self.get_raw_isolate() }
  }
}

pub trait InContext {}
impl<'s> InContext for Entered<'s, FunctionCallbackInfo> {}
impl<'s> InContext for Entered<'s, PropertyCallbackInfo> {}
impl<'s, X> InContext for Entered<'s, CallbackScope<X>> {}
impl<'s, P> InContext for Entered<'s, ContextScope<P>> {}
impl<'s, P> InContext for Entered<'s, HandleScope<P>> where P: InContext {}
impl<'s, P> InContext for Entered<'s, EscapableHandleScope<P>> where P: InContext
{}

/// When scope implements this trait, this means that it Local handles can be
/// created inside it.
pub trait ToLocal<'s>: InIsolate {
  unsafe fn to_local<T>(&mut self, ptr: *mut T) -> Option<Local<'s, T>> {
    crate::Local::<'s, T>::from_raw(ptr)
  }
}

impl<'s> ToLocal<'s> for Entered<'s, FunctionCallbackInfo> {}
impl<'s> ToLocal<'s> for Entered<'s, PropertyCallbackInfo> {}
impl<'s, P> ToLocal<'s> for Entered<'s, HandleScope<P>> {}
impl<'s, P> ToLocal<'s> for Entered<'s, EscapableHandleScope<P>> {}
impl<'s, P> ToLocal<'s> for Entered<'s, ContextScope<P>> where P: ToLocal<'s> {}

pub trait ToLocalOrReturnsLocal<'s>: InIsolate {}
impl<'s, E> ToLocalOrReturnsLocal<'s> for E where E: ToLocal<'s> {}
impl<'s> ToLocalOrReturnsLocal<'s> for Entered<'s, CallbackScope<Escapable>> {}
