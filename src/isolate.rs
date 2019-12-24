// Copyright 2018-2019 the Deno authors. All rights reserved. MIT license.
use crate::array_buffer::Allocator;
use crate::promise::PromiseRejectMessage;
use crate::support::Delete;
use crate::support::Opaque;
use crate::support::UniqueRef;
use crate::Context;
use crate::Local;
use crate::Message;
use crate::Module;
use crate::Object;
use crate::StartupData;
use crate::Value;
use std::ops::Deref;
use std::ops::DerefMut;
use std::ptr::NonNull;

type MessageCallback = extern "C" fn(Local<Message>, Local<Value>);

type PromiseRejectCallback = extern "C" fn(PromiseRejectMessage);

type HostInitializeImportMetaObjectCallback =
  extern "C" fn(Local<Context>, Local<Module>, Local<Object>);

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
    this: &mut Isolate,
    callback: MessageCallback,
  ) -> bool;
  fn v8__Isolate__SetPromiseRejectCallback(
    isolate: *mut Isolate,
    callback: PromiseRejectCallback,
  );
  fn v8__Isolate__SetHostInitializeImportMetaObjectCallback(
    isolate: *mut Isolate,
    callback: HostInitializeImportMetaObjectCallback,
  );
  fn v8__Isolate__ThrowException(
    isolate: &Isolate,
    exception: &Value,
  ) -> *mut Value;

  fn v8__Isolate__CreateParams__NEW() -> *mut CreateParams;
  fn v8__Isolate__CreateParams__DELETE(this: &mut CreateParams);
  fn v8__Isolate__CreateParams__SET__array_buffer_allocator(
    this: &mut CreateParams,
    value: *mut Allocator,
  );
  fn v8__Isolate__CreateParams__SET__snapshot_blob(
    this: &mut CreateParams,
    snapshot_blob: *mut StartupData,
  );
}

#[repr(C)]
/// Isolate represents an isolated instance of the V8 engine.  V8 isolates have
/// completely separate states.  Objects from one isolate must not be used in
/// other isolates.  The embedder can create multiple isolates and use them in
/// parallel in multiple threads.  An isolate can be entered by at most one
/// thread at any given time.  The Locker/Unlocker API must be used to
/// synchronize.
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
    unsafe { v8__Isolate__AddMessageListener(self, callback) }
  }

  /// Set callback to notify about promise reject with no handler, or
  /// revocation of such a previous notification once the handler is added.
  pub fn set_promise_reject_callback(
    &mut self,
    callback: PromiseRejectCallback,
  ) {
    unsafe { v8__Isolate__SetPromiseRejectCallback(self, callback) }
  }
  /// This specifies the callback called by the upcoming importa.meta
  /// language feature to retrieve host-defined meta data for a module.
  pub fn set_host_initialize_import_meta_object_callback(
    &mut self,
    callback: HostInitializeImportMetaObjectCallback,
  ) {
    unsafe {
      v8__Isolate__SetHostInitializeImportMetaObjectCallback(self, callback)
    }
  }

  /// Schedules an exception to be thrown when returning to JavaScript. When an
  /// exception has been scheduled it is illegal to invoke any JavaScript
  /// operation; the caller must return immediately and only after the exception
  /// has been handled does it become legal to invoke JavaScript operations.
  pub fn throw_exception<'sc>(
    &self,
    exception: Local<'_, Value>,
  ) -> Local<'sc, Value> {
    unsafe {
      let ptr = v8__Isolate__ThrowException(self, &exception);
      Local::from_raw_(ptr).unwrap()
    }
  }

  /// Disposes the isolate.  The isolate must not be entered by any
  /// thread to be disposable.
  pub unsafe fn dispose(&mut self) {
    v8__Isolate__Dispose(self)
  }
}

impl AsRef<Isolate> for Isolate {
  fn as_ref(&self) -> &Isolate {
    self
  }
}

impl AsMut<Isolate> for Isolate {
  fn as_mut(&mut self) -> &mut Isolate {
    self
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

/// Trait for retrieving the current isolate from a scope object.
pub trait InIsolate {
  // Do not implement this trait on unscoped Isolate references
  // (e.g. OwnedIsolate).
  fn isolate(&mut self) -> &mut Isolate;
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

  /// Hand startup data to V8, in case the embedder has chosen to build
  /// V8 with external startup data.
  ///
  /// Note:
  /// - By default the startup data is linked into the V8 library, in which
  ///   case this function is not meaningful.
  /// - If this needs to be called, it needs to be called before V8
  ///   tries to make use of its built-ins.
  /// - To avoid unnecessary copies of data, V8 will point directly into the
  ///   given data blob, so pretty please keep it around until V8 exit.
  /// - Compression of the startup blob might be useful, but needs to
  ///   handled entirely on the embedders' side.
  /// - The call will abort if the data is invalid.
  pub fn set_snapshot_blob(&mut self, snapshot_blob: &mut StartupData) {
    unsafe {
      v8__Isolate__CreateParams__SET__snapshot_blob(self, &mut *snapshot_blob)
    };
  }
}

impl Delete for CreateParams {
  fn delete(&'static mut self) {
    unsafe { v8__Isolate__CreateParams__DELETE(self) }
  }
}
