use std::mem::MaybeUninit;

use crate::scope::Entered;
use crate::scope::Scope;
use crate::scope::Scoped;
use crate::scope_traits::internal::GetRawIsolate;
use crate::FunctionCallbackInfo;
use crate::Isolate;
use crate::Local;
use crate::PromiseRejectMessage;
use crate::PropertyCallbackInfo;

/// A CallbackScope can be used to obtain a mutable Isolate reference within
/// a callback that is called by V8 on the thread that already has a Locker
/// on the stack.
///
/// Using a CallbackScope in any other situation is unsafe.
/// Also note that CallbackScope should not be used for function and property
/// accessor callbacks; use FunctionCallbackScope and PropertyCallbackScope
/// instead.
///
/// A CallbackScope can be created from the following inputs:
///   - `&mut Isolate`
/// ` - `Local<Context>`
///   - `Local<Message>`
///   - `Local<Object>`
///   - `Local<Promise>`
///   - `Local<SharedArrayBuffer>`
///   - `&PromiseRejectMessage`
pub struct CallbackScope {
  isolate: *mut Isolate,
}

impl CallbackScope {
  pub fn new<'s, I>(input: I) -> Scope<'s, Self>
  where
    Scope<'s, Self>: From<I>,
  {
    Scope::from(input)
  }

  pub(crate) fn get_raw_isolate_(&self) -> *mut Isolate {
    self.isolate
  }
}

unsafe impl<'s> Scoped<'s> for CallbackScope {
  type Args = *mut Isolate;
  fn enter_scope(buf: &mut MaybeUninit<Self>, isolate: Self::Args) {
    *buf = MaybeUninit::new(Self { isolate });
  }
}

impl<'s> From<&'s mut Isolate> for Scope<'s, CallbackScope> {
  fn from(isolate: &'s mut Isolate) -> Self {
    Scope::new(isolate as *mut Isolate)
  }
}

impl<'s, T> From<Local<'s, T>> for Scope<'s, CallbackScope>
where
  Local<'s, T>: GetRawIsolate,
{
  fn from(local: Local<'s, T>) -> Self {
    Scope::new(local.get_raw_isolate())
  }
}

impl<'s> From<&'s PromiseRejectMessage<'s>> for Scope<'s, CallbackScope> {
  fn from(msg: &'s PromiseRejectMessage<'s>) -> Self {
    Self::from(msg.get_promise())
  }
}

pub type FunctionCallbackScope<'s> = &'s mut Entered<'s, FunctionCallbackInfo>;
pub type PropertyCallbackScope<'s> = &'s mut Entered<'s, PropertyCallbackInfo>;
