#![allow(non_snake_case)]

use crate::isolate::CxxIsolate;
use crate::support::int;
use crate::support::Opaque;
use crate::Local;
use crate::String;
use crate::Value;

extern "C" {
  fn v8__Message__Get(message: *mut Message) -> *mut String;
  fn v8__StackTrace__GetFrameCount(stack_trace: *mut StackTrace) -> int;

  fn v8__Exception__RangeError(message: *mut String) -> *mut Value;
  fn v8__Exception__ReferenceError(message: *mut String) -> *mut Value;
  fn v8__Exception__SyntaxError(message: *mut String) -> *mut Value;
  fn v8__Exception__TypeError(message: *mut String) -> *mut Value;
  fn v8__Exception__Error(message: *mut String) -> *mut Value;

  fn v8__Exception__CreateMessage(
    isolate: *mut CxxIsolate,
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
  pub fn get(&mut self) -> Local<'_, String> {
    unsafe { Local::from_raw(v8__Message__Get(self)) }.unwrap()
  }
}

/// Create new error objects by calling the corresponding error object
/// constructor with the message.
pub mod Exception {
  use super::*;
  use crate::isolate::LockedIsolate;

  /// Creates an error message for the given exception.
  /// Will try to reconstruct the original stack trace from the exception value,
  /// or capture the current stack trace if not available.
  pub fn CreateMessage<'sc>(
    isolate: &mut impl LockedIsolate,
    mut exception: Local<'sc, Value>,
  ) -> Local<'sc, Message> {
    unsafe {
      Local::from_raw(v8__Exception__CreateMessage(
        isolate.cxx_isolate(),
        &mut *exception,
      ))
    }
    .unwrap()
  }

  /// Returns the original stack trace that was captured at the creation time
  /// of a given exception, or an empty handle if not available.
  pub fn GetStackTrace(
    mut exception: Local<'_, Value>,
  ) -> Option<Local<'_, StackTrace>> {
    unsafe { Local::from_raw(v8__Exception__GetStackTrace(&mut *exception)) }
  }

  pub fn RangeError(mut message: Local<'_, String>) -> Local<'_, Value> {
    unsafe { Local::from_raw(v8__Exception__RangeError(&mut *message)) }
      .unwrap()
  }

  pub fn ReferenceError(mut message: Local<'_, String>) -> Local<'_, Value> {
    unsafe { Local::from_raw(v8__Exception__ReferenceError(&mut *message)) }
      .unwrap()
  }

  pub fn SyntaxError(mut message: Local<'_, String>) -> Local<'_, Value> {
    unsafe { Local::from_raw(v8__Exception__SyntaxError(&mut *message)) }
      .unwrap()
  }

  pub fn TypeError(mut message: Local<'_, String>) -> Local<'_, Value> {
    unsafe { Local::from_raw(v8__Exception__TypeError(&mut *message)) }.unwrap()
  }

  pub fn Error(mut message: Local<'_, String>) -> Local<'_, Value> {
    unsafe { Local::from_raw(v8__Exception__Error(&mut *message)) }.unwrap()
  }
}
