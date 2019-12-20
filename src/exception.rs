#![allow(non_snake_case)]

use crate::isolate::Isolate;
use crate::support::int;
use crate::support::Opaque;
use crate::HandleScope;
use crate::Local;
use crate::String;
use crate::Value;

extern "C" {
  fn v8__Message__Get(message: *const Message) -> *mut String;
  fn v8__Message__GetIsolate(message: &Message) -> &mut Isolate;

  fn v8__StackTrace__GetFrameCount(stack_trace: *mut StackTrace) -> int;

  fn v8__Exception__RangeError(message: *mut String) -> *mut Value;
  fn v8__Exception__ReferenceError(message: *mut String) -> *mut Value;
  fn v8__Exception__SyntaxError(message: *mut String) -> *mut Value;
  fn v8__Exception__TypeError(message: *mut String) -> *mut Value;
  fn v8__Exception__Error(message: *mut String) -> *mut Value;

  fn v8__Exception__CreateMessage(
    isolate: &Isolate,
    exception: *mut Value,
  ) -> *mut Message;

  fn v8__Exception__GetStackTrace(exception: *mut Value) -> *mut StackTrace;
}

/// Representation of a JavaScript stack trace. The information collected is a
/// snapshot of the execution stack and the information remains valid after
/// execution continues.
#[repr(C)]
pub struct StackTrace(Opaque);

impl StackTrace {
  /// Returns the number of StackFrames.
  pub fn get_frame_count(&mut self) -> usize {
    unsafe { v8__StackTrace__GetFrameCount(self) as usize }
  }
}

/// An error message.
#[repr(C)]
pub struct Message(Opaque);

impl Message {
  pub fn get<'sc>(&self, _scope: &mut HandleScope<'sc>) -> Local<'sc, String> {
    unsafe { Local::from_raw(v8__Message__Get(self)) }.unwrap()
  }

  #[allow(clippy::mut_from_ref)]
  pub fn get_isolate(&self) -> &mut Isolate {
    unsafe { v8__Message__GetIsolate(self) }
  }
}

/// Creates an error message for the given exception.
/// Will try to reconstruct the original stack trace from the exception value,
/// or capture the current stack trace if not available.
pub fn create_message<'sc>(
  scope: &mut HandleScope<'sc>,
  mut exception: Local<'sc, Value>,
) -> Local<'sc, Message> {
  unsafe {
    Local::from_raw(v8__Exception__CreateMessage(
      scope.as_mut(),
      &mut *exception,
    ))
  }
  .unwrap()
}

/// Returns the original stack trace that was captured at the creation time
/// of a given exception, or an empty handle if not available.
pub fn get_stack_trace<'sc>(
  _scope: &mut HandleScope<'sc>,
  mut exception: Local<Value>,
) -> Option<Local<'sc, StackTrace>> {
  unsafe { Local::from_raw(v8__Exception__GetStackTrace(&mut *exception)) }
}

pub fn range_error<'sc>(
  _scope: &mut HandleScope<'sc>,
  mut message: Local<String>,
) -> Local<'sc, Value> {
  unsafe { Local::from_raw(v8__Exception__RangeError(&mut *message)) }.unwrap()
}

pub fn reference_error<'sc>(
  _scope: &mut HandleScope<'sc>,
  mut message: Local<String>,
) -> Local<'sc, Value> {
  unsafe { Local::from_raw(v8__Exception__ReferenceError(&mut *message)) }
    .unwrap()
}

pub fn syntax_error<'sc>(
  _scope: &mut HandleScope<'sc>,
  mut message: Local<String>,
) -> Local<'sc, Value> {
  unsafe { Local::from_raw(v8__Exception__SyntaxError(&mut *message)) }.unwrap()
}

pub fn type_error<'sc>(
  _scope: &mut HandleScope<'sc>,
  mut message: Local<String>,
) -> Local<'sc, Value> {
  unsafe { Local::from_raw(v8__Exception__TypeError(&mut *message)) }.unwrap()
}

pub fn error<'sc>(
  _scope: &mut HandleScope<'sc>,
  mut message: Local<String>,
) -> Local<'sc, Value> {
  unsafe { Local::from_raw(v8__Exception__Error(&mut *message)) }.unwrap()
}
