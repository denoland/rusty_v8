#![allow(non_snake_case)]

use crate::isolate::Isolate;
use crate::support::int;
use crate::Context;
use crate::Local;
use crate::Message;
use crate::StackFrame;
use crate::StackTrace;
use crate::String;
use crate::ToLocal;
use crate::Value;

extern "C" {
  fn v8__Message__Get(this: *const Message) -> *const String;
  fn v8__Message__GetSourceLine(
    this: *const Message,
    context: *const Context,
  ) -> *const String;
  fn v8__Message__GetScriptResourceName(this: *const Message) -> *const Value;
  fn v8__Message__GetLineNumber(
    this: *const Message,
    context: *const Context,
  ) -> int;
  fn v8__Message__GetStartPosition(this: *const Message) -> int;
  fn v8__Message__GetEndPosition(this: *const Message) -> int;
  fn v8__Message__GetWasmFunctionIndex(this: *const Message) -> int;
  fn v8__Message__ErrorLevel(this: *const Message) -> int;
  fn v8__Message__GetStartColumn(this: *const Message) -> int;
  fn v8__Message__GetEndColumn(this: *const Message) -> int;
  fn v8__Message__IsSharedCrossOrigin(this: *const Message) -> bool;
  fn v8__Message__IsOpaque(this: *const Message) -> bool;
  fn v8__Message__GetStackTrace(this: *const Message) -> *const StackTrace;

  fn v8__StackTrace__GetFrameCount(this: *const StackTrace) -> int;
  fn v8__StackTrace__GetFrame(
    this: *const StackTrace,
    isolate: *mut Isolate,
    index: u32,
  ) -> *const StackFrame;

  fn v8__StackFrame__GetLineNumber(this: *const StackFrame) -> int;
  fn v8__StackFrame__GetColumn(this: *const StackFrame) -> int;
  fn v8__StackFrame__GetScriptId(this: *const StackFrame) -> int;
  fn v8__StackFrame__GetScriptName(this: *const StackFrame) -> *const String;
  fn v8__StackFrame__GetScriptNameOrSourceURL(
    this: *const StackFrame,
  ) -> *const String;
  fn v8__StackFrame__GetFunctionName(this: *const StackFrame) -> *const String;
  fn v8__StackFrame__IsEval(this: *const StackFrame) -> bool;
  fn v8__StackFrame__IsConstructor(this: *const StackFrame) -> bool;
  fn v8__StackFrame__IsWasm(this: *const StackFrame) -> bool;
  fn v8__StackFrame__IsUserJavaScript(this: *const StackFrame) -> bool;

  fn v8__Exception__Error(message: *const String) -> *const Value;
  fn v8__Exception__RangeError(message: *const String) -> *const Value;
  fn v8__Exception__ReferenceError(message: *const String) -> *const Value;
  fn v8__Exception__SyntaxError(message: *const String) -> *const Value;
  fn v8__Exception__TypeError(message: *const String) -> *const Value;

  fn v8__Exception__CreateMessage(
    isolate: *mut Isolate,
    exception: *const Value,
  ) -> *const Message;
  fn v8__Exception__GetStackTrace(exception: *const Value)
    -> *const StackTrace;
}

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
      let ptr = v8__StackTrace__GetFrame(self, isolate, index as u32);
      scope.to_local(ptr)
    }
  }
}

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
    context: Local<Context>,
  ) -> Option<Local<'s, String>> {
    unsafe { scope.to_local(v8__Message__GetSourceLine(self, &*context)) }
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
  pub fn get_line_number(&self, context: Local<Context>) -> Option<usize> {
    let i = unsafe { v8__Message__GetLineNumber(self, &*context) };
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

/// Create new error objects by calling the corresponding error object
/// constructor with the message.
pub struct Exception;

impl Exception {
  pub fn error<'sc>(
    scope: &mut impl ToLocal<'sc>,
    message: Local<String>,
  ) -> Local<'sc, Value> {
    let isolate = scope.isolate();
    isolate.enter();
    let e = unsafe { v8__Exception__Error(&*message) };
    isolate.exit();
    unsafe { scope.to_local(e) }.unwrap()
  }

  pub fn range_error<'sc>(
    scope: &mut impl ToLocal<'sc>,
    message: Local<String>,
  ) -> Local<'sc, Value> {
    let isolate = scope.isolate();
    isolate.enter();
    let e = unsafe { v8__Exception__RangeError(&*message) };
    isolate.exit();
    unsafe { scope.to_local(e) }.unwrap()
  }

  pub fn reference_error<'sc>(
    scope: &mut impl ToLocal<'sc>,
    message: Local<String>,
  ) -> Local<'sc, Value> {
    let isolate = scope.isolate();
    isolate.enter();
    let e = unsafe { v8__Exception__ReferenceError(&*message) };
    isolate.exit();
    unsafe { scope.to_local(e) }.unwrap()
  }

  pub fn syntax_error<'sc>(
    scope: &mut impl ToLocal<'sc>,
    message: Local<String>,
  ) -> Local<'sc, Value> {
    let isolate = scope.isolate();
    isolate.enter();
    let e = unsafe { v8__Exception__SyntaxError(&*message) };
    isolate.exit();
    unsafe { scope.to_local(e) }.unwrap()
  }

  pub fn type_error<'sc>(
    scope: &mut impl ToLocal<'sc>,
    message: Local<String>,
  ) -> Local<'sc, Value> {
    let isolate = scope.isolate();
    isolate.enter();
    let e = unsafe { v8__Exception__TypeError(&*message) };
    isolate.exit();
    unsafe { scope.to_local(e) }.unwrap()
  }

  /// Creates an error message for the given exception.
  /// Will try to reconstruct the original stack trace from the exception value,
  /// or capture the current stack trace if not available.
  pub fn create_message<'sc>(
    scope: &mut impl ToLocal<'sc>,
    exception: Local<Value>,
  ) -> Local<'sc, Message> {
    let isolate = scope.isolate();
    let ptr = unsafe { v8__Exception__CreateMessage(isolate, &*exception) };
    unsafe { scope.to_local(ptr) }.unwrap()
  }

  /// Returns the original stack trace that was captured at the creation time
  /// of a given exception, or an empty handle if not available.
  pub fn get_stack_trace<'sc>(
    scope: &mut impl ToLocal<'sc>,
    exception: Local<Value>,
  ) -> Option<Local<'sc, StackTrace>> {
    unsafe { scope.to_local(v8__Exception__GetStackTrace(&*exception)) }
  }
}
