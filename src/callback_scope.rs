use std::marker::PhantomData;

use crate::scope::Entered;
use crate::scope::Scope;
use crate::scope::ScopeDefinition;
use crate::scope_traits::internal::GetRawIsolate;
use crate::Context;
use crate::FunctionCallbackInfo;
use crate::Isolate;
use crate::Local;
use crate::PromiseRejectMessage;
use crate::PropertyCallbackInfo;
use crate::ToLocal;

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
pub struct CallbackScope<X = Default> {
  isolate: *mut Isolate,
  phantom: PhantomData<X>,
}

pub struct Default;
pub struct Escapable;

impl<'s> CallbackScope {
  pub fn new<I>(input: I) -> Scope<'s, Self>
  where
    Scope<'s, Self>: From<I>,
  {
    Scope::from(input)
  }
}

impl<'s> CallbackScope<Escapable> {
  pub fn new_escapable<I>(input: I) -> Scope<'s, Self>
  where
    Scope<'s, Self>: From<I>,
  {
    Scope::from(input)
  }
}

impl<X> CallbackScope<X> {
  pub(crate) fn get_raw_isolate_(&self) -> *mut Isolate {
    self.isolate
  }
}

unsafe impl<'s, X> ScopeDefinition<'s> for CallbackScope<X> {
  type Parent = ();
  type Args = *mut Isolate;
  unsafe fn enter_scope(ptr: *mut Self, isolate: Self::Args) {
    let data = Self {
      isolate,
      phantom: PhantomData,
    };
    std::ptr::write(ptr, data);
  }
}

impl<'s, X> From<&'s mut Isolate> for Scope<'s, CallbackScope<X>> {
  fn from(isolate: &'s mut Isolate) -> Self {
    Scope::new(isolate as *mut Isolate)
  }
}

impl<'s, X, T> From<Local<'s, T>> for Scope<'s, CallbackScope<X>>
where
  Local<'s, T>: GetRawIsolate,
{
  fn from(local: Local<'s, T>) -> Self {
    Scope::new(local.get_raw_isolate())
  }
}

impl<'s, X> From<&'s PromiseRejectMessage<'s>> for Scope<'s, CallbackScope<X>> {
  fn from(msg: &'s PromiseRejectMessage<'s>) -> Self {
    Self::from(msg.get_promise())
  }
}

/// Stack-allocated class which sets the execution context for all operations
/// executed within a local scope.
pub struct ContextScope<P> {
  context: ContextContainer,
  phantom: PhantomData<P>,
}

impl<'s, P> ContextScope<P>
where
  P: ToLocal<'s>,
{
  pub fn new(
    _parent: &'s mut P,
    context: Local<'s, Context>,
  ) -> Scope<'s, Self> {
    Scope::new(context)
  }

  pub(crate) fn get_captured_context(&self) -> Local<'s, Context> {
    unsafe { self.context.to_local() }
  }
}

unsafe impl<'s, P> ScopeDefinition<'s> for ContextScope<P>
where
  P: ToLocal<'s>,
{
  type Parent = P;
  type Args = Local<'s, Context>;

  unsafe fn enter_scope(ptr: *mut Self, mut context: Self::Args) {
    context.enter();
    let data = Self {
      context: context.into(),
      phantom: PhantomData,
    };
    std::ptr::write(ptr, data);
  }
}

// TODO(piscisaureus): It should not be necessary to create an inner struct
// to appease the drop checker.
struct ContextContainer(*mut Context);

impl<'s> From<Local<'s, Context>> for ContextContainer {
  fn from(mut local: Local<Context>) -> Self {
    let context = &mut *local as *mut Context;
    Self(context)
  }
}

impl ContextContainer {
  unsafe fn to_local<'s>(&self) -> Local<'s, Context> {
    Local::from_raw(self.0).unwrap()
  }
}

impl Drop for ContextContainer {
  fn drop(&mut self) {
    unsafe { self.to_local() }.exit()
  }
}

pub type FunctionCallbackScope<'s> = &'s mut Entered<'s, FunctionCallbackInfo>;
pub type PropertyCallbackScope<'s> = &'s mut Entered<'s, PropertyCallbackInfo>;
