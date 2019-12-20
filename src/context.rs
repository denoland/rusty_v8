use crate::isolate::Isolate;
use crate::support::Opaque;
use crate::Local;
use crate::Object;

extern "C" {
  fn v8__Context__New(isolate: &Isolate) -> *mut Context;
  fn v8__Context__Enter(this: &mut Context);
  fn v8__Context__Exit(this: &mut Context);
  fn v8__Context__GetIsolate(this: &mut Context) -> *mut Isolate;
  fn v8__Context__Global(this: *mut Context) -> *mut Object;
}

#[repr(C)]
pub struct Context(Opaque);

impl Context {
  pub fn new<'sc>(isolate: &Isolate) -> Local<Context> {
    // TODO: optional arguments;
    unsafe { Local::from_raw(v8__Context__New(isolate)).unwrap() }
  }

  /// Returns the global proxy object.
  ///
  /// Global proxy object is a thin wrapper whose prototype points to actual
  /// context's global object with the properties like Object, etc. This is done
  /// that way for security reasons (for more details see
  /// https://wiki.mozilla.org/Gecko:SplitWindow).
  ///
  /// Please note that changes to global proxy object prototype most probably
  /// would break VM---v8 expects only global object as a prototype of global
  /// proxy object.
  pub fn global<'sc>(&mut self) -> Local<Object> {
    unsafe { Local::from_raw(v8__Context__Global(&mut *self)).unwrap() }
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
