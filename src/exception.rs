#![allow(non_snake_case)]

use crate::isolate::Isolate;
use crate::support::int;
use crate::support::Opaque;
use crate::Context;
use crate::Local;
use crate::String;
use crate::ToLocal;
use crate::Value;

extern "C" {
  fn v8__Message__Get(message: *const Message) -> *mut String;
  fn v8__Message__GetSourceLine(
    message: &Message,
    context: *mut Context,
  ) -> *mut String;
  fn v8__Message__GetScriptResourceName(message: &Message) -> *mut Value;
  fn v8__Message__GetLineNumber(
    message: &Message,
    context: *mut Context,
  ) -> int;
  fn v8__Message__GetStartPosition(message: &Message) -> int;
  fn v8__Message__GetEndPosition(message: &Message) -> int;
  fn v8__Message__GetWasmFunctionIndex(message: &Message) -> int;
  fn v8__Message__ErrorLevel(message: &Message) -> int;
  fn v8__Message__GetStartColumn(message: &Message) -> int;
  fn v8__Message__GetEndColumn(message: &Message) -> int;
  fn v8__Message__IsSharedCrossOrigin(message: &Message) -> bool;
  fn v8__Message__IsOpaque(message: &Message) -> bool;
  fn v8__Message__GetStackTrace(message: &Message) -> *mut StackTrace;

  fn v8__StackTrace__GetFrameCount(self_: &StackTrace) -> int;
  fn v8__StackTrace__GetFrame(
    self_: &StackTrace,
    isolate: *mut Isolate,
    index: u32,
  ) -> *mut StackFrame;

  fn v8__StackFrame__GetLineNumber(self_: &StackFrame) -> int;
  fn v8__StackFrame__GetColumn(self_: &StackFrame) -> int;
  fn v8__StackFrame__GetScriptId(self_: &StackFrame) -> int;
  fn v8__StackFrame__GetScriptName(self_: &StackFrame) -> *mut String;
  fn v8__StackFrame__GetScriptNameOrSourceURL(
    self_: &StackFrame,
  ) -> *mut String;
  fn v8__StackFrame__GetFunctionName(self_: &StackFrame) -> *mut String;
  fn v8__StackFrame__IsEval(self_: &StackFrame) -> bool;
  fn v8__StackFrame__IsConstructor(self_: &StackFrame) -> bool;
  fn v8__StackFrame__IsWasm(self_: &StackFrame) -> bool;
  fn v8__StackFrame__IsUserJavaScript(self_: &StackFrame) -> bool;

  fn v8__Exception__RangeError(message: *mut String) -> *mut Value;
  fn v8__Exception__ReferenceError(message: *mut String) -> *mut Value;
  fn v8__Exception__SyntaxError(message: *mut String) -> *mut Value;
  fn v8__Exception__TypeError(message: *mut String) -> *mut Value;
  fn v8__Exception__Error(message: *mut String) -> *mut Value;

  fn v8__Exception__CreateMessage(
    isolate: &Isolate,
    exception: Local<Value>,
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
  pub fn get_frame_count(&self) -> usize {
    unsafe { v8__StackTrace__GetFrameCount(self) as usize }
  }

  /// Returns a StackFrame at a particular index.
  pub fn get_frame<'sc>(
    &self,
    scope: &mut impl ToLocal<'sc>,
    index: usize,
  ) -> Option<Local<'sc, StackFrame>> {
    let isolate = scope.isolate();
    unsafe {
      Local::from_raw(v8__StackTrace__GetFrame(self, isolate, index as u32))
    }
  }
}

/// A single JavaScript stack frame.
#[repr(C)]
pub struct StackFrame(Opaque);

impl StackFrame {
  /// Returns the number, 1-based, of the line for the associate function call.
  /// This method will return Message::kNoLineNumberInfo if it is unable to
  /// retrieve the line number, or if kLineNumber was not passed as an option
  /// when capturing the StackTrace.
  pub fn get_line_number(&self) -> usize {
    unsafe { v8__StackFrame__GetLineNumber(self) as usize }
  }

  /// Returns the 1-based column offset on the line for the associated function
  /// call.
  /// This method will return Message::kNoColumnInfo if it is unable to retrieve
  /// the column number, or if kColumnOffset was not passed as an option when
  /// capturing the StackTrace.
  pub fn get_column(&self) -> usize {
    unsafe { v8__StackFrame__GetColumn(self) as usize }
  }

  /// Returns the id of the script for the function for this StackFrame.
  /// This method will return Message::kNoScriptIdInfo if it is unable to
  /// retrieve the script id, or if kScriptId was not passed as an option when
  /// capturing the StackTrace.
  pub fn get_script_id(&self) -> usize {
    unsafe { v8__StackFrame__GetScriptId(self) as usize }
  }

  /// Returns the name of the resource that contains the script for the
  /// function for this StackFrame.
  pub fn get_script_name<'sc>(
    &self,
    scope: &mut impl ToLocal<'sc>,
  ) -> Option<Local<'sc, String>> {
    unsafe { scope.to_local(v8__StackFrame__GetScriptName(self)) }
  }

  /// Returns the name of the resource that contains the script for the
  /// function for this StackFrame or sourceURL value if the script name
  /// is undefined and its source ends with //# sourceURL=... string or
  /// deprecated //@ sourceURL=... string.
  pub fn get_script_name_or_source_url<'sc>(
    &self,
    scope: &mut impl ToLocal<'sc>,
  ) -> Option<Local<'sc, String>> {
    unsafe { scope.to_local(v8__StackFrame__GetScriptNameOrSourceURL(self)) }
  }

  /// Returns the name of the function associated with this stack frame.
  pub fn get_function_name<'sc>(
    &self,
    scope: &mut impl ToLocal<'sc>,
  ) -> Option<Local<'sc, String>> {
    unsafe { scope.to_local(v8__StackFrame__GetFunctionName(self)) }
  }

  /// Returns whether or not the associated function is compiled via a call to
  /// eval().
  pub fn is_eval(&self) -> bool {
    unsafe { v8__StackFrame__IsEval(self) }
  }

  /// Returns whether or not the associated function is called as a
  /// constructor via "new".
  pub fn is_constructor(&self) -> bool {
    unsafe { v8__StackFrame__IsConstructor(self) }
  }

  /// Returns whether or not the associated functions is defined in wasm.
  pub fn is_wasm(&self) -> bool {
    unsafe { v8__StackFrame__IsWasm(self) }
  }

  /// Returns whether or not the associated function is defined by the user.
  pub fn is_user_javascript(&self) -> bool {
    unsafe { v8__StackFrame__IsUserJavaScript(self) }
  }
}

/// An error message.
#[repr(C)]
pub struct Message(Opaque);

impl Message {
  pub fn get<'sc>(&self, scope: &mut impl ToLocal<'sc>) -> Local<'sc, String> {
    unsafe { scope.to_local(v8__Message__Get(self)) }.unwrap()
  }

  /// Exception stack trace. By default stack traces are not captured for
  /// uncaught exceptions. SetCaptureStackTraceForUncaughtExceptions allows
  /// to change this option.
  pub fn get_stack_trace<'sc>(
    &self,
    scope: &mut impl ToLocal<'sc>,
  ) -> Option<Local<'sc, StackTrace>> {
    unsafe { scope.to_local(v8__Message__GetStackTrace(self)) }
  }

  pub fn get_source_line<'s>(
    &self,
    scope: &mut impl ToLocal<'s>,
    mut context: Local<Context>,
  ) -> Option<Local<'s, String>> {
    unsafe { scope.to_local(v8__Message__GetSourceLine(self, &mut *context)) }
  }

  /// Returns the resource name for the script from where the function causing
  /// the error originates.
  pub fn get_script_resource_name<'s>(
    &self,
    scope: &mut impl ToLocal<'s>,
  ) -> Option<Local<'s, Value>> {
    unsafe { scope.to_local(v8__Message__GetScriptResourceName(self)) }
  }

  /// Returns the number, 1-based, of the line where the error occurred.
  pub fn get_line_number(&self, mut context: Local<Context>) -> Option<usize> {
    let i = unsafe { v8__Message__GetLineNumber(self, &mut *context) };
    if i < 0 {
      None
    } else {
      Some(i as usize)
    }
  }

  /// Returns the index within the script of the first character where
  /// the error occurred.
  pub fn get_start_position(&self) -> int {
    unsafe { v8__Message__GetStartPosition(self) }
  }

  /// Returns the index within the script of the last character where
  /// the error occurred.
  pub fn get_end_position(&self) -> int {
    unsafe { v8__Message__GetEndPosition(self) }
  }

  /// Returns the Wasm function index where the error occurred. Returns -1 if
  /// message is not from a Wasm script.
  pub fn get_wasm_function_index(&self) -> int {
    unsafe { v8__Message__GetWasmFunctionIndex(self) }
  }

  /// Returns the error level of the message.
  pub fn error_level(&self) -> int {
    unsafe { v8__Message__ErrorLevel(self) }
  }

  /// Returns the index within the line of the first character where
  /// the error occurred.
  pub fn get_start_column(&self) -> usize {
    unsafe { v8__Message__GetStartColumn(self) as usize }
  }

  /// Returns the index within the line of the last character where
  /// the error occurred.
  pub fn get_end_column(&self) -> usize {
    unsafe { v8__Message__GetEndColumn(self) as usize }
  }

  /// Passes on the value set by the embedder when it fed the script from which
  /// this Message was generated to V8.
  pub fn is_shared_cross_origin(&self) -> bool {
    unsafe { v8__Message__IsSharedCrossOrigin(self) }
  }

  pub fn is_opaque(&self) -> bool {
    unsafe { v8__Message__IsOpaque(self) }
  }
}

/// Creates an error message for the given exception.
/// Will try to reconstruct the original stack trace from the exception value,
/// or capture the current stack trace if not available.
pub fn create_message<'sc>(
  scope: &mut impl ToLocal<'sc>,
  exception: Local<Value>,
) -> Local<'sc, Message> {
  let isolate = scope.isolate();
  let ptr = unsafe { v8__Exception__CreateMessage(isolate, exception) };
  unsafe { scope.to_local(ptr) }.unwrap()
}

/// Returns the original stack trace that was captured at the creation time
/// of a given exception, or an empty handle if not available.
pub fn get_stack_trace<'sc>(
  scope: &mut impl ToLocal<'sc>,
  mut exception: Local<Value>,
) -> Option<Local<'sc, StackTrace>> {
  unsafe { scope.to_local(v8__Exception__GetStackTrace(&mut *exception)) }
}

pub fn range_error<'sc>(
  scope: &mut impl ToLocal<'sc>,
  mut message: Local<String>,
) -> Local<'sc, Value> {
  let isolate = scope.isolate();
  isolate.enter();
  let e = unsafe { v8__Exception__RangeError(&mut *message) };
  isolate.exit();
  unsafe { scope.to_local(e) }.unwrap()
}

pub fn reference_error<'sc>(
  scope: &mut impl ToLocal<'sc>,
  mut message: Local<String>,
) -> Local<'sc, Value> {
  let isolate = scope.isolate();
  isolate.enter();
  let e = unsafe { v8__Exception__ReferenceError(&mut *message) };
  isolate.exit();
  unsafe { scope.to_local(e) }.unwrap()
}

pub fn syntax_error<'sc>(
  scope: &mut impl ToLocal<'sc>,
  mut message: Local<String>,
) -> Local<'sc, Value> {
  let isolate = scope.isolate();
  isolate.enter();
  let e = unsafe { v8__Exception__SyntaxError(&mut *message) };
  isolate.exit();
  unsafe { scope.to_local(e) }.unwrap()
}

pub fn type_error<'sc>(
  scope: &mut impl ToLocal<'sc>,
  mut message: Local<String>,
) -> Local<'sc, Value> {
  let isolate = scope.isolate();
  isolate.enter();
  let e = unsafe { v8__Exception__TypeError(&mut *message) };
  isolate.exit();
  unsafe { scope.to_local(e) }.unwrap()
}

pub fn error<'sc>(
  scope: &mut impl ToLocal<'sc>,
  mut message: Local<String>,
) -> Local<'sc, Value> {
  let isolate = scope.isolate();
  isolate.enter();
  let e = unsafe { v8__Exception__Error(&mut *message) };
  isolate.exit();
  unsafe { scope.to_local(e) }.unwrap()
}
