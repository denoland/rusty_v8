use crate::isolate::{CxxIsolate, LockedIsolate};
use crate::support::{int, Opaque};
use crate::Context;
use crate::HandleScope;
use crate::Local;
use crate::Value;

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
  fn v8__FunctionCallbackInfo__SetReturnValue(
    info: &FunctionCallbackInfo,
    value: *mut Value,
  );
  fn v8__FunctionCallbackInfo__GetReturnValue(
    info: &FunctionCallbackInfo,
  ) -> *mut ReturnValue;

  fn v8__ReturnValue__Set(rv: *mut ReturnValue, value: *mut Value) -> ();
  fn v8__ReturnValue__Get(rv: *mut ReturnValue) -> *mut Value;
  fn v8__ReturnValue__GetIsolate(rv: *mut ReturnValue) -> *mut CxxIsolate;
}

#[repr(C)]
pub struct ReturnValue(Opaque);

/// In V8 ReturnValue<> has a type parameter, but
/// it turns out that in most of the APIs it's ReturnValue<Value>
/// and for our purposes we currently don't need
/// other types. So for now it's a simplified version.
impl ReturnValue {
  // NOTE: simplest setter, possibly we'll need to add
  // more setters specialized per type
  pub fn set(&mut self, mut value: Local<'_, Value>) {
    unsafe { v8__ReturnValue__Set(&mut *self, &mut *value) }
  }

  /// Convenience getter for Isolate
  pub fn get_isolate(&mut self) -> *mut CxxIsolate {
    unsafe { v8__ReturnValue__GetIsolate(&mut *self) }
  }

  /// Getter. Creates a new Local<> so it comes with a certain performance
  /// hit. If the ReturnValue was not yet set, this will return the undefined
  /// value.
  pub fn get<'sc>(
    &mut self,
    _scope: &mut HandleScope<'sc>,
  ) -> Local<'sc, Value> {
    unsafe { Local::from_raw(v8__ReturnValue__Get(&mut *self)).unwrap() }
  }
}

#[repr(C)]
pub struct FunctionCallbackInfo(Opaque);

impl FunctionCallbackInfo {
  pub fn set_return_value(&self, mut value: Local<'_, Value>) {
    unsafe { v8__FunctionCallbackInfo__SetReturnValue(&*self, &mut *value) };
  }

  pub fn get_return_value(&self) -> &mut ReturnValue {
    unsafe { &mut *v8__FunctionCallbackInfo__GetReturnValue(&*self) }
  }

  pub fn get_isolate(&self) -> &mut CxxIsolate {
    unsafe { v8__FunctionCallbackInfo__GetIsolate(self) }
  }

  pub fn length(&self) -> int {
    unsafe { v8__FunctionCallbackInfo__Length(&*self) }
  }
}

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
