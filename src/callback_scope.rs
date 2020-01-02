use crate::scope::{Scope, Scoped};
use crate::Context;
use crate::InIsolate;
use crate::Isolate;
use crate::Local;
use crate::Message;
use crate::Promise;
use crate::PromiseRejectMessage;
use std::mem::MaybeUninit;

extern "C" {
  fn v8__Promise__GetIsolate(promise: *mut Promise) -> *mut Isolate;
}

pub trait GetIsolate
where
  Self: Sized,
{
  fn get_isolate(&mut self) -> &mut Isolate;
}

impl GetIsolate for Context {
  fn get_isolate(&mut self) -> &mut Isolate {
    self.get_isolate()
  }
}

impl GetIsolate for Message {
  fn get_isolate(&mut self) -> &mut Isolate {
    self.get_isolate()
  }
}

impl GetIsolate for Promise {
  fn get_isolate(&mut self) -> &mut Isolate {
    unsafe { &mut *v8__Promise__GetIsolate(self) }
  }
}

impl<'a> GetIsolate for PromiseRejectMessage<'a> {
  fn get_isolate(&mut self) -> &mut Isolate {
    unsafe { &mut *v8__Promise__GetIsolate(&mut *self.get_promise()) }
  }
}

pub struct CallbackScope<'s, T> {
  local: Local<'s, T>,
}

unsafe impl<'s, T> Scoped<'s> for CallbackScope<'s, T> {
  type Args = Local<'s, T>;
  fn enter_scope(buf: &mut MaybeUninit<Self>, local: Local<'s, T>) {
    *buf = MaybeUninit::new(CallbackScope { local });
  }
}

impl<'s, T> CallbackScope<'s, T> {
  pub fn new(local: Local<'s, T>) -> Scope<Self> {
    Scope::new(local)
  }
}

impl<'s, T> InIsolate for crate::scope::Entered<'s, CallbackScope<'s, T>>
where
  T: GetIsolate,
{
  fn isolate(&mut self) -> &mut Isolate {
    self.local.get_isolate()
  }
}
