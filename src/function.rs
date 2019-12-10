use crate::isolate::{CxxIsolate, LockedIsolate};
use crate::support::{int, Delete, Opaque, UniquePtr};
use crate::Context;
use crate::Local;
use crate::Value;
use std;
use std::ops::DerefMut;

extern "C" {
  fn v8__Function__New(
    context: *mut Context,
    callback: extern "C" fn(&FunctionCallbackInfo),
  ) -> *mut Function;
  fn v8__Function__Call(
    function: *mut Function,
    context: *mut Context,
    recv: *mut Value,
    argc: int,
    argv: *mut *mut Value,
  ) -> *mut Value;

  fn v8__FunctionTemplate__New(
    isolate: *mut CxxIsolate,
    callback: extern "C" fn(&FunctionCallbackInfo),
  ) -> *mut FunctionTemplate;
  fn v8__FunctionTemplate__GetFunction(
    fn_template: *mut FunctionTemplate,
    context: *mut Context,
  ) -> *mut Function;

  fn v8__FunctionCallbackInfo__GetIsolate(
    info: &FunctionCallbackInfo,
  ) -> &mut CxxIsolate;
  fn v8__FunctionCallbackInfo__Length(info: &FunctionCallbackInfo) -> int;
  fn v8__FunctionCallbackInfo__GetReturnValue(
    info: &FunctionCallbackInfo,
  ) -> *mut ReturnValue;

  fn v8__ReturnValue__Set(rv: *mut ReturnValue, value: *mut Value);
}

#[repr(C)]
pub struct ReturnValue(Opaque);

impl ReturnValue {
  // TODO: supports only Local<Value>, add support for other
  pub fn set(&mut self, mut value: Local<'_, Value>) {
    unsafe { v8__ReturnValue__Set(&mut *self, &mut *value) }
  }
}

impl Delete for ReturnValue {
  fn delete(&'static mut self) -> () {}
}

#[repr(C)]
pub struct FunctionCallbackInfo(Opaque);

impl FunctionCallbackInfo {
  pub fn get_return_value(&self) -> &mut ReturnValue {
    let foo = unsafe { v8__FunctionCallbackInfo__GetReturnValue(&*self) };

    let ref_imm: &mut ReturnValue = unsafe { &mut *foo };
    ref_imm
  }

  pub fn get_isolate(&self) -> &mut CxxIsolate {
    unsafe { v8__FunctionCallbackInfo__GetIsolate(self) }
  }

  pub fn length(&self) -> int {
    unsafe { v8__FunctionCallbackInfo__Length(&*self) }
  }
}

pub type FunctionCallback =
  unsafe extern "C" fn(info: &mut FunctionCallbackInfo);

#[repr(C)]
pub struct FunctionTemplate(Opaque);

impl FunctionTemplate {
  /// Creates a function template.
  pub fn new(
    isolate: &mut impl LockedIsolate,
    callback: extern "C" fn(&FunctionCallbackInfo),
  ) -> Local<'_, FunctionTemplate> {
    unsafe {
      Local::from_raw(v8__FunctionTemplate__New(
        isolate.cxx_isolate(),
        callback,
      ))
      .unwrap()
    }
  }

  pub fn get_function(
    &mut self,
    mut context: Local<'_, Context>,
  ) -> Option<Local<'_, Function>> {
    unsafe {
      Local::from_raw(v8__FunctionTemplate__GetFunction(
        &mut *self,
        &mut *context,
      ))
    }
  }
}

#[repr(C)]
pub struct Function(Opaque);

/// A JavaScript function object (ECMA-262, 15.3).
impl Function {
  // TODO: add remaining arguments from C++
  /// Create a function in the current execution context
  /// for a given FunctionCallback.
  pub fn new(
    mut context: Local<'_, Context>,
    callback: extern "C" fn(&FunctionCallbackInfo),
  ) -> Option<Local<'_, Function>> {
    unsafe { Local::from_raw(v8__Function__New(&mut *context, callback)) }
  }

  pub fn call(
    &mut self,
    mut context: Local<'_, Context>,
    mut recv: Local<'_, Value>,
    arc: i32,
    argv: Vec<Local<'_, Value>>,
  ) -> Option<Local<'_, Value>> {
    let mut argv_: Vec<*mut Value> = vec![];
    for mut arg in argv {
      argv_.push(&mut *arg);
    }
    unsafe {
      Local::from_raw(v8__Function__Call(
        &mut *self,
        &mut *context,
        &mut *recv,
        arc,
        argv_.as_mut_ptr(),
      ))
    }
  }
}
