// Copyright 2019-2020 the Deno authors. All rights reserved. MIT license.

use std::marker::PhantomData;
use std::mem::size_of;
use std::mem::take;
use std::mem::MaybeUninit;
use std::ptr::NonNull;

use crate::scope_traits::internal::GetRawIsolate;
use crate::Context;
use crate::FunctionCallbackInfo;
use crate::InIsolate;
use crate::Isolate;
use crate::Local;
use crate::PromiseRejectMessage;
use crate::PropertyCallbackInfo;

// Note: the 's lifetime is there to ensure that after entering a scope once,
// the same scope object can't ever be entered again.

/// A trait for defining scoped objects.
pub unsafe trait ScopeDefinition<'s>
where
  Self: Sized,
{
  type Args;
  unsafe fn enter_scope(buf: *mut Self, args: Self::Args);
}

/// A RAII scope wrapper object that will, when the `enter()` method is called,
/// initialize and activate the guarded object.
pub struct ScopeData<'s, D, P = ()>
where
  D: ScopeDefinition<'s>,
{
  state: ScopeState<'s, D, P>,
}

enum ScopeState<'s, D, P>
where
  D: ScopeDefinition<'s>,
{
  Empty,
  Allocated {
    args: D::Args,
    parent: &'s mut P,
  },
  EnteredUninit {
    data: MaybeUninit<D>,
    enter: MaybeUninit<Scope<'s, D, P>>,
  },
  EnteredReady {
    data: D,
    enter: Scope<'s, D, P>,
  },
}

fn parent_of_root() -> &'static mut () {
  unsafe { &mut *NonNull::<()>::dangling().as_ptr() }
}

impl<'s, D> ScopeData<'s, D, ()>
where
  D: ScopeDefinition<'s>,
{
  /// Create a new root ScopeData object in unentered state.
  pub(crate) fn new_root(args: D::Args) -> Self {
    Self::new(args, parent_of_root())
  }
}

impl<'s, D, P> ScopeData<'s, D, P>
where
  D: ScopeDefinition<'s>,
{
  /// Create a new ScopeData object in unentered state.
  pub(crate) fn new(args: D::Args, parent: &'s mut P) -> Self {
    Self {
      state: ScopeState::Allocated { args, parent },
    }
  }

  /// Initializes the guarded object and returns a mutable reference to it.
  /// A scope can only be entered once.
  pub fn enter(&'s mut self) -> &'s mut Scope<'s, D, P> {
    assert_eq!(size_of::<D>(), size_of::<MaybeUninit<D>>());

    use ScopeState::*;
    let Self { state } = self;

    let (parent, args) = match take(state) {
      Allocated { parent, args } => (parent, args),
      _ => unreachable!(),
    };

    *state = EnteredUninit {
      data: MaybeUninit::uninit(),
      enter: MaybeUninit::uninit(),
    };
    let data_ptr = match state {
      EnteredUninit { data, .. } => data as *mut _ as *mut D,
      _ => unreachable!(),
    };

    unsafe { D::enter_scope(data_ptr, args) };

    *state = match take(state) {
      EnteredUninit { data, .. } => EnteredReady {
        data: unsafe { data.assume_init() },
        enter: Scope::new(unsafe { &mut *data_ptr }, parent),
      },
      _ => unreachable!(),
    };

    match state {
      EnteredReady { enter, .. } => enter,
      _ => unreachable!(),
    }
  }
}

impl<'s, D, P> Default for ScopeState<'s, D, P>
where
  D: ScopeDefinition<'s>,
{
  fn default() -> Self {
    Self::Empty
  }
}

/// A wrapper around the an instantiated and entered scope object.
#[repr(C)]
pub struct Scope<'s, D, P = ()> {
  data: *mut D,
  parent: &'s mut P,
}

impl<'s, D> Scope<'s, D, ()> {
  pub(crate) fn new_root(data: *mut D) -> Self {
    Self {
      data,
      parent: parent_of_root(),
    }
  }
}

impl<'s, D, P> Scope<'s, D, P> {
  pub(crate) fn new(data: *mut D, parent: &'s mut P) -> Self {
    Self { data, parent }
  }

  pub(crate) fn data(&self) -> &D {
    unsafe { &*self.data }
  }

  pub(crate) fn data_mut(&mut self) -> &mut D {
    unsafe { &mut *self.data }
  }

  pub(crate) fn parent(&self) -> &P {
    &self.parent
  }

  pub(crate) fn parent_mut(&mut self) -> &mut P {
    &mut self.parent
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
///   - `Local<Context>`
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
  pub fn new<I>(input: I) -> ScopeData<'s, Self>
  where
    ScopeData<'s, Self>: From<I>,
  {
    ScopeData::from(input)
  }
}

impl<'s> CallbackScope<Escapable> {
  pub fn new_escapable<I>(input: I) -> ScopeData<'s, Self>
  where
    ScopeData<'s, Self>: From<I>,
  {
    ScopeData::from(input)
  }
}

impl<X> CallbackScope<X> {
  pub(crate) fn get_raw_isolate_(&self) -> *mut Isolate {
    self.isolate
  }
}

unsafe impl<'s, X> ScopeDefinition<'s> for CallbackScope<X> {
  type Args = *mut Isolate;
  unsafe fn enter_scope(ptr: *mut Self, isolate: Self::Args) {
    let data = Self {
      isolate,
      phantom: PhantomData,
    };
    std::ptr::write(ptr, data);
  }
}

impl<'s, X> From<&'s mut Isolate> for ScopeData<'s, CallbackScope<X>> {
  fn from(isolate: &'s mut Isolate) -> Self {
    ScopeData::new_root(isolate as *mut Isolate)
  }
}

impl<'s, X, T> From<Local<'s, T>> for ScopeData<'s, CallbackScope<X>>
where
  Local<'s, T>: GetRawIsolate,
{
  fn from(local: Local<'s, T>) -> Self {
    ScopeData::new_root(local.get_raw_isolate())
  }
}

impl<'s, X> From<&'s PromiseRejectMessage<'s>>
  for ScopeData<'s, CallbackScope<X>>
{
  fn from(msg: &'s PromiseRejectMessage<'s>) -> Self {
    Self::from(msg.get_promise())
  }
}

/// Stack-allocated class which sets the execution context for all operations
/// executed within a local scope.
pub struct ContextScope {
  context: *mut Context,
}

impl<'s> ContextScope {
  pub fn new<P>(
    parent: &'s mut P,
    context: Local<'s, Context>,
  ) -> ScopeData<'s, Self, P>
  where
    P: InIsolate,
  {
    ScopeData::new(context, parent)
  }

  pub(crate) unsafe fn get_captured_context(&self) -> Local<'s, Context> {
    Local::from_raw(self.context).unwrap()
  }
}

unsafe impl<'s> ScopeDefinition<'s> for ContextScope {
  type Args = Local<'s, Context>;

  unsafe fn enter_scope(ptr: *mut Self, mut context: Self::Args) {
    context.enter();
    std::ptr::write(
      ptr,
      Self {
        context: &mut *context,
      },
    );
  }
}

impl Drop for ContextScope {
  fn drop(&mut self) {
    unsafe { self.get_captured_context() }.exit()
  }
}

pub type FunctionCallbackScope<'s> = &'s mut Scope<'s, FunctionCallbackInfo>;
pub type PropertyCallbackScope<'s> = &'s mut Scope<'s, PropertyCallbackInfo>;
