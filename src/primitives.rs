use crate::isolate::CxxIsolate;
use crate::isolate::LockedIsolate;
use crate::support::Opaque;
use crate::HandleScope;
use crate::Local;

/// The superclass of primitive values.  See ECMA-262 4.3.2.
#[repr(C)]
pub struct Primitive(Opaque);

/// A primitive boolean value (ECMA-262, 4.3.14).  Either the true
/// or false value.
#[repr(C)]
pub struct Boolean(Opaque);

extern "C" {
  fn v8__Null(isolate: *mut CxxIsolate) -> *mut Primitive;

  fn v8__Undefined(isolate: *mut CxxIsolate) -> *mut Primitive;

  fn v8__True(isolate: *mut CxxIsolate) -> *mut Boolean;

  fn v8__False(isolate: *mut CxxIsolate) -> *mut Boolean;
}

pub fn new_null<'sc>(scope: &mut HandleScope<'sc>) -> Local<'sc, Primitive> {
  unsafe { Local::from_raw(v8__Null(scope.cxx_isolate())) }.unwrap()
}

pub fn new_undefined<'sc>(
  scope: &mut HandleScope<'sc>,
) -> Local<'sc, Primitive> {
  unsafe { Local::from_raw(v8__Undefined(scope.cxx_isolate())) }.unwrap()
}

pub fn new_true<'sc>(scope: &mut HandleScope<'sc>) -> Local<'sc, Boolean> {
  unsafe { Local::from_raw(v8__True(scope.cxx_isolate())) }.unwrap()
}

pub fn new_false<'sc>(scope: &mut HandleScope<'sc>) -> Local<'sc, Boolean> {
  unsafe { Local::from_raw(v8__False(scope.cxx_isolate())) }.unwrap()
}
