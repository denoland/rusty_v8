use crate::Boolean;
use crate::Isolate;
use crate::Local;
use crate::Primitive;
use crate::Scope;

extern "C" {
  fn v8__Null(isolate: *mut Isolate) -> *mut Primitive;
  fn v8__Undefined(isolate: *mut Isolate) -> *mut Primitive;

  fn v8__Boolean__New(isolate: *mut Isolate, value: bool) -> *mut Boolean;
}

pub fn null<'sc>(scope: &'sc mut Scope) -> Local<Primitive> {
  let ptr = unsafe { v8__Null(scope.isolate()) };
  unsafe { scope.to_local(ptr) }.unwrap()
}

pub fn undefined<'sc>(scope: &'sc mut Scope) -> Local<Primitive> {
  let ptr = unsafe { v8__Undefined(scope.isolate()) };
  unsafe { scope.to_local(ptr) }.unwrap()
}

impl Boolean {
  pub fn new<'sc>(scope: &'sc mut Scope, value: bool) -> Local<Boolean> {
    let ptr = unsafe { v8__Boolean__New(scope.isolate(), value) };
    unsafe { scope.to_local(ptr) }.unwrap()
  }
}
