use crate::isolate::CxxIsolate;
use crate::isolate::LockedIsolate;
use crate::support::Opaque;
use crate::HandleScope;
use crate::Local;

extern "C" {
  fn v8__Context__New(isolate: *mut CxxIsolate) -> *mut Context;
  fn v8__Context__Enter(this: &mut Context);
  fn v8__Context__Exit(this: &mut Context);
  fn v8__Context__GetIsolate(this: &mut Context) -> *mut CxxIsolate;
}

#[repr(C)]
pub struct Context(Opaque);

impl Context {
  pub fn new<'sc>(scope: &mut HandleScope<'sc>) -> Local<'sc, Context> {
    // TODO: optional arguments;
    unsafe { Local::from_raw(v8__Context__New(scope.cxx_isolate())).unwrap() }
  }

  pub fn enter(&mut self) {
    // TODO: enter/exit should be controlled by a scope.
    unsafe { v8__Context__Enter(self) };
  }

  pub fn exit(&mut self) {
    // TODO: enter/exit should be controlled by a scope.
    unsafe { v8__Context__Exit(self) };
  }
}
