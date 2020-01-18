// Copyright 2019-2020 the Deno authors. All rights reserved. MIT license.

use std::marker::PhantomData;
use std::mem::size_of;
use std::mem::take;
use std::mem::MaybeUninit;
use std::ops::Deref;
use std::ops::DerefMut;

use crate::scope_traits::internal::GetRawIsolate;
use crate::Context;
use crate::FunctionCallbackInfo;
use crate::Isolate;
use crate::Local;
use crate::PromiseRejectMessage;
use crate::PropertyCallbackInfo;
use crate::ToLocal;

// Note: the 's lifetime is there to ensure that after entering a scope once,
// the same scope object can't ever be entered again.

/// A trait for defining scoped objects.
pub unsafe trait ScopeDefinition<'s>
where
  Self: Sized,
{
  type Parent;
  type Args;
  unsafe fn enter_scope(buf: *mut Self, args: Self::Args) -> ();
}

/// A RAII scope wrapper object that will, when the `enter()` method is called,
/// initialize and activate the guarded object.
pub struct Scope<'s, S>
where
  S: ScopeDefinition<'s>,
{
  state: ScopeState<'s, S>,
}

enum ScopeState<'s, S>
where
  S: ScopeDefinition<'s>,
{
  Empty,
  New(S::Args),
  Uninit {
    data: MaybeUninit<S>,
    enter: MaybeUninit<Entered<'s, S>>,
  },
  Ready {
    data: S,
    enter: Entered<'s, S>,
  },
}

impl<'s, S> Scope<'s, S>
where
  S: ScopeDefinition<'s>,
{
  /// Create a new Scope object in unentered state.
  pub(crate) fn new(args: S::Args) -> Self {
    Self {
      state: ScopeState::New(args),
    }
  }

  /// Initializes the guarded object and returns a mutable reference to it.
  /// A scope can only be entered once.
  pub fn enter(&'s mut self) -> &'s mut Entered<'s, S> {
    assert_eq!(size_of::<S>(), size_of::<MaybeUninit<S>>());

    use ScopeState::*;
    let Self { state } = self;

    let args = match take(state) {
      New(f) => f,
      _ => unreachable!(),
    };

    *state = Uninit {
      data: MaybeUninit::uninit(),
      enter: MaybeUninit::uninit(),
    };
    let data_ptr = match state {
      Uninit { data, .. } => data as *mut _ as *mut S,
      _ => unreachable!(),
    };

    unsafe { S::enter_scope(data_ptr, args) };

    *state = match take(state) {
      Uninit { data, .. } => Ready {
        data: unsafe { data.assume_init() },
        enter: Entered::new(unsafe { &mut *data_ptr }),
      },
      _ => unreachable!(),
    };

    match state {
      Ready { enter, .. } => enter,
      _ => unreachable!(),
    }
  }
}

impl<'s, S> Default for ScopeState<'s, S>
where
  S: ScopeDefinition<'s>,
{
  fn default() -> Self {
    Self::Empty
  }
}

/// A wrapper around the an instantiated and entered scope object.
#[repr(transparent)]
pub struct Entered<'s, S>(*mut S, PhantomData<&'s ()>);

impl<'s, S> Entered<'s, S> {
  pub(crate) fn new(data: *mut S) -> Self {
    Self(data, PhantomData)
  }
}

impl<'s, S> Deref for Entered<'s, S> {
  type Target = S;
  fn deref(&self) -> &S {
    unsafe { &*self.0 }
  }
}

impl<'s, S> DerefMut for Entered<'s, S> {
  fn deref_mut(&mut self) -> &mut S {
    unsafe { &mut *self.0 }
  }
}

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
pub struct CallbackScope<X = Contained> {
  isolate: *mut Isolate,
  phantom: PhantomData<X>,
}

pub struct Contained;
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
