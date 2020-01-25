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
  unsafe fn enter_scope(buf: *mut Self, args: Self::Args) -> ();
}

/// A RAII scope wrapper object that will, when the `enter()` method is called,
/// initialize and activate the guarded object.
pub struct Scope<'s, S, P = ()>
where
  S: ScopeDefinition<'s>,
{
  state: ScopeState<'s, S, P>,
}

enum ScopeState<'s, S, P>
where
  S: ScopeDefinition<'s>,
{
  Empty,
  Allocated {
    args: S::Args,
    parent: &'s mut P,
  },
  EnteredUninit {
    data: MaybeUninit<S>,
    enter: MaybeUninit<Entered<'s, S, P>>,
  },
  EnteredReady {
    data: S,
    enter: Entered<'s, S, P>,
  },
}

fn parent_of_root() -> &'static mut () {
  unsafe { &mut *NonNull::<()>::dangling().as_ptr() }
}

impl<'s, S> Scope<'s, S, ()>
where
  S: ScopeDefinition<'s>,
{
  /// Create a new root Scope object in unentered state.
  pub(crate) fn new_root(args: S::Args) -> Self {
    Self::new(args, parent_of_root())
  }
}

impl<'s, S, P> Scope<'s, S, P>
where
  S: ScopeDefinition<'s>,
{
  /// Create a new Scope object in unentered state.
  pub(crate) fn new(args: S::Args, parent: &'s mut P) -> Self {
    Self {
      state: ScopeState::Allocated { args, parent },
    }
  }

  /// Initializes the guarded object and returns a mutable reference to it.
  /// A scope can only be entered once.
  pub fn enter(&'s mut self) -> &'s mut Entered<'s, S, P> {
    assert_eq!(size_of::<S>(), size_of::<MaybeUninit<S>>());

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
      EnteredUninit { data, .. } => data as *mut _ as *mut S,
      _ => unreachable!(),
    };

    unsafe { S::enter_scope(data_ptr, args) };

    *state = match take(state) {
      EnteredUninit { data, .. } => EnteredReady {
        data: unsafe { data.assume_init() },
        enter: Entered::new(unsafe { &mut *data_ptr }, parent),
      },
      _ => unreachable!(),
    };

    match state {
      EnteredReady { enter, .. } => enter,
      _ => unreachable!(),
    }
  }
}

impl<'s, S, P> Default for ScopeState<'s, S, P>
where
  S: ScopeDefinition<'s>,
{
  fn default() -> Self {
    Self::Empty
  }
}

/// A wrapper around the an instantiated and entered scope object.
#[repr(C)]
pub struct Entered<'s, S, P = ()> {
  data: *mut S,
  parent: &'s mut P,
}

impl<'s, S> Entered<'s, S, ()> {
  pub(crate) fn new_root(data: *mut S) -> Self {
    Self {
      data,
      parent: parent_of_root(),
    }
  }
}

impl<'s, S, P> Entered<'s, S, P> {
  pub(crate) fn new(data: *mut S, parent: &'s mut P) -> Self {
    Self { data, parent }
  }

  pub(crate) fn data(&self) -> &S {
    unsafe { &*self.data }
  }

  pub(crate) fn data_mut(&mut self) -> &mut S {
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
    Scope::new_root(isolate as *mut Isolate)
  }
}

impl<'s, X, T> From<Local<'s, T>> for Scope<'s, CallbackScope<X>>
where
  Local<'s, T>: GetRawIsolate,
{
  fn from(local: Local<'s, T>) -> Self {
    Scope::new_root(local.get_raw_isolate())
  }
}

impl<'s, X> From<&'s PromiseRejectMessage<'s>> for Scope<'s, CallbackScope<X>> {
  fn from(msg: &'s PromiseRejectMessage<'s>) -> Self {
    Self::from(msg.get_promise())
  }
}

#[repr(C)]
/// v8::Locker is a scoped lock object. While it's active, i.e. between its
/// construction and destruction, the current thread is allowed to use the locked
/// isolate. V8 guarantees that an isolate can be locked by at most one thread at
/// any time. In other words, the scope of a v8::Locker is a critical section.
pub struct Locker {
  has_lock: bool,
  top_level: bool,
  isolate: *mut Isolate,
}

extern "C" {
  fn v8__Locker__CONSTRUCT(buf: *mut Locker, isolate: *mut Isolate);
  fn v8__Locker__DESTRUCT(this: &mut Locker);
}

impl<'s> Locker {
  // TODO(piscisaureus): We should not be sharing &Isolate references between
  // threads while at the same time dereferencing to &mut Isolate *within* the
  // various scopes. Instead, add a separate type (e.g. IsolateHandle).
  pub fn new(isolate: &Isolate) -> Scope<'s, Self> {
    Scope::new_root(isolate as *const _ as *mut Isolate)
  }

  pub(crate) fn get_raw_isolate_(&self) -> *mut Isolate {
    self.isolate
  }
}

unsafe impl<'s> ScopeDefinition<'s> for Locker {
  type Args = *mut Isolate;

  unsafe fn enter_scope(buf: *mut Self, isolate: *mut Isolate) {
    v8__Locker__CONSTRUCT(buf, isolate)
  }
}

impl Drop for Locker {
  fn drop(&mut self) {
    unsafe { v8__Locker__DESTRUCT(self) }
  }
}

/// Stack-allocated class which sets the isolate for all operations
/// executed within a local scope.
pub struct IsolateScope {
  pub(crate) isolate: *mut Isolate,
}

impl<'s> IsolateScope {
  pub fn new<P>(
    parent: &'s mut P,
    isolate: &'s mut Isolate,
  ) -> Scope<'s, Self, P> {
    Scope::new(isolate as *mut Isolate, parent)
  }
}

unsafe impl<'s> ScopeDefinition<'s> for IsolateScope {
  type Args = *mut Isolate;

  unsafe fn enter_scope(ptr: *mut Self, isolate: Self::Args) {
    (&mut *isolate).enter();
    std::ptr::write(ptr, Self { isolate });
  }
}

impl Drop for IsolateScope {
  fn drop(&mut self) {
    unsafe { &mut *self.isolate }.exit();
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
  ) -> Scope<'s, Self, P>
  where
    P: InIsolate,
  {
    Scope::new(context, parent)
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

pub type FunctionCallbackScope<'s> = &'s mut Entered<'s, FunctionCallbackInfo>;
pub type PropertyCallbackScope<'s> = &'s mut Entered<'s, PropertyCallbackInfo>;
