use std::ops::Deref;
use std::ops::DerefMut;
use std::ptr::NonNull;

use crate::array_buffer::Allocator;
use crate::exception::Message;
use crate::support::Delete;
use crate::support::Opaque;
use crate::support::UniqueRef;
use crate::Local;
use crate::Value;

type MessageCallback = extern "C" fn(Local<'_, Message>, Local<'_, Value>);

type PromiseRejectCallback = extern "C" fn(PromiseRejectMessage);

extern "C" {
  fn v8__Isolate__New(params: *mut CreateParams) -> *mut Isolate;
  fn v8__Isolate__Dispose(this: *mut Isolate);
  fn v8__Isolate__Enter(this: *mut Isolate);
  fn v8__Isolate__Exit(this: *mut Isolate);
  fn v8__Isolate__SetCaptureStackTraceForUncaughtExceptions(
    this: *mut Isolate,
    caputre: bool,
    frame_limit: i32,
  );
  fn v8__Isolate__AddMessageListener(
    this: &mut CxxIsolate,
    callback: MessageCallback,
  ) -> bool;

  fn v8__Isolate__CreateParams__NEW() -> *mut CreateParams;
  fn v8__Isolate__CreateParams__DELETE(this: &mut CreateParams);
  fn v8__Isolate__CreateParams__SET__array_buffer_allocator(
    this: &mut CreateParams,
    value: *mut Allocator,
  );

}

#[repr(C)]
pub struct Isolate(Opaque);

impl Isolate {
  /// Creates a new isolate.  Does not change the currently entered
  /// isolate.
  ///
  /// When an isolate is no longer used its resources should be freed
  /// by calling V8::dispose().  Using the delete operator is not allowed.
  ///
  /// V8::initialize() must have run prior to this.
  #[allow(clippy::new_ret_no_self)]
  pub fn new(params: UniqueRef<CreateParams>) -> OwnedIsolate {
    // TODO: support CreateParams.
    crate::V8::assert_initialized();
    let isolate_ptr = unsafe { v8__Isolate__New(params.into_raw()) };
    OwnedIsolate(NonNull::new(isolate_ptr).unwrap())
  }

  /// Initial configuration parameters for a new Isolate.
  pub fn create_params() -> UniqueRef<CreateParams> {
    CreateParams::new()
  }

  /// Sets this isolate as the entered one for the current thread.
  /// Saves the previously entered one (if any), so that it can be
  /// restored when exiting.  Re-entering an isolate is allowed.
  pub fn enter(&mut self) {
    unsafe { v8__Isolate__Enter(self) }
  }

  /// Exits this isolate by restoring the previously entered one in the
  /// current thread.  The isolate may still stay the same, if it was
  /// entered more than once.
  ///
  /// Requires: self == Isolate::GetCurrent().
  pub fn exit(&mut self) {
    unsafe { v8__Isolate__Exit(self) }
  }

  /// Tells V8 to capture current stack trace when uncaught exception occurs
  /// and report it to the message listeners. The option is off by default.
  pub fn set_capture_stack_trace_for_uncaught_exceptions(
    &mut self,
    capture: bool,
    frame_limit: i32,
  ) {
    unsafe {
      v8__Isolate__SetCaptureStackTraceForUncaughtExceptions(
        self,
        capture,
        frame_limit,
      )
    }
  }

  /// Adds a message listener (errors only).
  ///
  /// The same message listener can be added more than once and in that
  /// case it will be called more than once for each message.
  ///
  /// The exception object will be passed to the callback.
  pub fn add_message_listener(&mut self, callback: MessageCallback) -> bool {
    unsafe { v8__Isolate__AddMessageListener(self.0, callback) }
  }
}

/// Same as Isolate but gets disposed when it goes out of scope.
pub struct OwnedIsolate(NonNull<Isolate>);

impl Drop for OwnedIsolate {
  fn drop(&mut self) {
    unsafe { self.0.as_mut().dispose() }
  }
}

impl Deref for OwnedIsolate {
  type Target = Isolate;
  fn deref(&self) -> &Self::Target {
    unsafe { self.0.as_ref() }
  }
}

impl DerefMut for OwnedIsolate {
  fn deref_mut(&mut self) -> &mut Self::Target {
    unsafe { self.0.as_mut() }
  }
}

#[repr(C)]
pub struct CreateParams(Opaque);

impl CreateParams {
  pub fn new() -> UniqueRef<CreateParams> {
    unsafe { UniqueRef::from_raw(v8__Isolate__CreateParams__NEW()) }
  }

  pub fn set_array_buffer_allocator(&mut self, value: UniqueRef<Allocator>) {
    unsafe {
      v8__Isolate__CreateParams__SET__array_buffer_allocator(
        self,
        value.into_raw(),
      )
    };
  }
}

impl Delete for CreateParams {
  fn delete(&'static mut self) {
    unsafe { v8__Isolate__CreateParams__DELETE(self) }
  }
}
