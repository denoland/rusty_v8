use crate::isolate::Isolate;
use crate::Boolean;
use crate::Local;
use crate::Primitive;
use crate::ToLocal;

extern "C" {
  fn v8__Null(isolate: *mut Isolate) -> *const Primitive;
  fn v8__Undefined(isolate: *mut Isolate) -> *const Primitive;

  fn v8__Boolean__New(isolate: *mut Isolate, value: bool) -> *const Boolean;
}

pub fn null<'sc>(scope: &mut impl ToLocal<'sc>) -> Local<'sc, Primitive> {
  let ptr = unsafe { v8__Null(scope.isolate()) };
  unsafe { scope.to_local(ptr) }.unwrap()
}

pub fn undefined<'sc>(scope: &mut impl ToLocal<'sc>) -> Local<'sc, Primitive> {
  let ptr = unsafe { v8__Undefined(scope.isolate()) };
  unsafe { scope.to_local(ptr) }.unwrap()
}

impl Boolean {
  pub fn new<'sc>(
    scope: &mut impl ToLocal<'sc>,
    value: bool,
  ) -> Local<'sc, Boolean> {
    let ptr = unsafe { v8__Boolean__New(scope.isolate(), value) };
    unsafe { scope.to_local(ptr) }.unwrap()
  }
}
