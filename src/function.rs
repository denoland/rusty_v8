use crate::support::{int, Opaque};
use crate::Context;
use crate::Local;
use crate::Value;

extern "C" {
  fn v8__Function__New(
    context: *mut Context,
    callback: *mut FunctionCallback,
  ) -> *mut Function;
  fn v8__Function__Call(
    function: *mut Function,
    context: *mut Context,
    recv: *mut Value,
    argc: int,
    argv: *mut *mut Value,
  ) -> *mut Value;
}

#[repr(C)]
pub struct FunctionCallbackInfo(Opaque);

impl FunctionCallbackInfo {
  pub fn get_return_value(&mut self) {
    unimplemented!();
  }
}

pub type FunctionCallback =
  unsafe extern "C" fn(info: *mut FunctionCallbackInfo);

#[repr(C)]
pub struct Function(Opaque);

/// A JavaScript function object (ECMA-262, 15.3).
impl Function {
  // TODO: add remaining arguments from C++
  /// Create a function in the current execution context
  /// for a given FunctionCallback.
  pub fn new(
    mut context: Local<'_, Context>,
    mut callback: FunctionCallback,
  ) -> Option<Local<'_, Function>> {
    unsafe { Local::from_raw(v8__Function__New(&mut *context, &mut callback)) }
  }

  pub fn call(
    &mut self,
    mut context: Local<'_, Context>,
    mut recv: Local<'_, Value>,
    argc: i32,
    argv: Vec<Local<'_, Value>>,
  ) -> Option<Local<'_, Value>> {
    let mut argv_: Vec<*mut Value> = vec![];
    for mut arg in argv {
      let n = &mut *arg;
      argv_.push(n);
    }

    unsafe {
      Local::from_raw(v8__Function__Call(
        &mut *self,
        &mut *context,
        &mut *recv,
        argc,
        argv_.as_mut_ptr(),
      ))
    }
  }
}
