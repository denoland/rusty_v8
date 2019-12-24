use crate::scope::{Scope, Scoped};
use crate::Context;
use crate::InIsolate;
use crate::Isolate;
use crate::Local;
use crate::Message;
use std::mem::MaybeUninit;

pub trait GetIsolate
where
  Self: Sized,
{
  fn get_isolate(&mut self) -> &mut Isolate;
}

impl GetIsolate for Message {
  fn get_isolate(&mut self) -> &mut Isolate {
    self.get_isolate()
  }
}

impl GetIsolate for Context {
  fn get_isolate(&mut self) -> &mut Isolate {
    self.get_isolate()
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
