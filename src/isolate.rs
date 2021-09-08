// Copyright 2019-2021 the Deno authors. All rights reserved. MIT license.
use crate::function::FunctionCallbackInfo;
use crate::isolate_create_params::raw;
use crate::isolate_create_params::CreateParams;
use crate::promise::PromiseRejectMessage;
use crate::scope::data::ScopeData;
use crate::support::BuildTypeIdHasher;
use crate::support::MapFnFrom;
use crate::support::MapFnTo;
use crate::support::Opaque;
use crate::support::ToCFn;
use crate::support::UnitType;
use crate::wasm::trampoline;
use crate::wasm::WasmStreaming;
use crate::Array;
use crate::CallbackScope;
use crate::Context;
use crate::FixedArray;
use crate::Function;
use crate::HandleScope;
use crate::Local;
use crate::Message;
use crate::Module;
use crate::Object;
use crate::Promise;
use crate::ScriptOrModule;
use crate::String;
use crate::Value;

use std::any::Any;
use std::any::TypeId;

use std::collections::HashMap;
use std::ffi::c_void;
use std::fmt::{self, Debug, Formatter};
use std::mem::MaybeUninit;
use std::ops::Deref;
use std::ops::DerefMut;
use std::os::raw::c_char;
use std::ptr::null_mut;
use std::ptr::NonNull;
use std::sync::Arc;
use std::sync::Mutex;

/// Policy for running microtasks:
///   - explicit: microtasks are invoked with the
///               Isolate::PerformMicrotaskCheckpoint() method;
///   - auto: microtasks are invoked when the script call depth decrements
///           to zero.
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)]
pub enum MicrotasksPolicy {
  Explicit = 0,
  // Scoped = 1 (RAII) is omitted for now, doesn't quite map to idiomatic Rust.
  Auto = 2,
}

/// PromiseHook with type Init is called when a new promise is
/// created. When a new promise is created as part of the chain in the
/// case of Promise.then or in the intermediate promises created by
/// Promise.{race, all}/AsyncFunctionAwait, we pass the parent promise
/// otherwise we pass undefined.
///
/// PromiseHook with type Resolve is called at the beginning of
/// resolve or reject function defined by CreateResolvingFunctions.
///
/// PromiseHook with type Before is called at the beginning of the
/// PromiseReactionJob.
///
/// PromiseHook with type After is called right at the end of the
/// PromiseReactionJob.
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)]
pub enum PromiseHookType {
  Init,
  Resolve,
  Before,
  After,
}

pub type MessageCallback = extern "C" fn(Local<Message>, Local<Value>);

pub type PromiseHook =
  extern "C" fn(PromiseHookType, Local<Promise>, Local<Value>);

pub type PromiseRejectCallback = extern "C" fn(PromiseRejectMessage);

/// HostInitializeImportMetaObjectCallback is called the first time import.meta
/// is accessed for a module. Subsequent access will reuse the same value.
///
/// The method combines two implementation-defined abstract operations into one:
/// HostGetImportMetaProperties and HostFinalizeImportMeta.
///
/// The embedder should use v8::Object::CreateDataProperty to add properties on
/// the meta object.
pub type HostInitializeImportMetaObjectCallback =
  extern "C" fn(Local<Context>, Local<Module>, Local<Object>);

/// HostImportModuleDynamicallyWithImportAssertionsCallback is called when we require the
/// embedder to load a module. This is used as part of the dynamic
/// import syntax.
///
/// The referrer contains metadata about the script/module that calls
/// import.
///
/// The specifier is the name of the module that should be imported.
///
/// The embedder must compile, instantiate, evaluate the Module, and
/// obtain it's namespace object.
///
/// The Promise returned from this function is forwarded to userland
/// JavaScript. The embedder must resolve this promise with the module
/// namespace object. In case of an exception, the embedder must reject
/// this promise with the exception. If the promise creation itself
/// fails (e.g. due to stack overflow), the embedder must propagate
/// that exception by returning an empty MaybeLocal.
pub type HostImportModuleDynamicallyWithImportAssertionsCallback =
  extern "C" fn(
    Local<Context>,
    Local<ScriptOrModule>,
    Local<String>,
    Local<FixedArray>,
  ) -> *mut Promise;

pub type InterruptCallback =
  extern "C" fn(isolate: &mut Isolate, data: *mut c_void);

pub type NearHeapLimitCallback = extern "C" fn(
  data: *mut c_void,
  current_heap_limit: usize,
  initial_heap_limit: usize,
) -> usize;

pub type OomErrorCallback =
  extern "C" fn(location: *const c_char, is_heap_oom: bool);

/// Collection of V8 heap information.
///
/// Instances of this class can be passed to v8::Isolate::GetHeapStatistics to
/// get heap statistics from V8.
// Must be >= sizeof(v8::HeapStatistics), see v8__HeapStatistics__CONSTRUCT().
#[repr(C)]
#[derive(Debug)]
pub struct HeapStatistics([usize; 16]);

// Windows x64 ABI: MaybeLocal<Value> returned on the stack.
#[cfg(target_os = "windows")]
pub type PrepareStackTraceCallback<'s> = extern "C" fn(
  *mut *const Value,
  Local<'s, Context>,
  Local<'s, Value>,
  Local<'s, Array>,
) -> *mut *const Value;

// System V ABI: MaybeLocal<Value> returned in a register.
#[cfg(not(target_os = "windows"))]
pub type PrepareStackTraceCallback<'s> = extern "C" fn(
  Local<'s, Context>,
  Local<'s, Value>,
  Local<'s, Array>,
) -> *const Value;

extern "C" {
  fn v8__Isolate__New(params: *const raw::CreateParams) -> *mut Isolate;
  fn v8__Isolate__Dispose(this: *mut Isolate);
  fn v8__Isolate__SetData(this: *mut Isolate, slot: u32, data: *mut c_void);
  fn v8__Isolate__GetData(this: *const Isolate, slot: u32) -> *mut c_void;
  fn v8__Isolate__GetNumberOfDataSlots(this: *const Isolate) -> u32;
  fn v8__Isolate__Enter(this: *mut Isolate);
  fn v8__Isolate__Exit(this: *mut Isolate);
  fn v8__Isolate__ClearKeptObjects(isolate: *mut Isolate);
  fn v8__Isolate__LowMemoryNotification(isolate: *mut Isolate);
  fn v8__Isolate__GetHeapStatistics(this: *mut Isolate, s: *mut HeapStatistics);
  fn v8__Isolate__SetCaptureStackTraceForUncaughtExceptions(
    this: *mut Isolate,
    caputre: bool,
    frame_limit: i32,
  );
  fn v8__Isolate__AddMessageListener(
    isolate: *mut Isolate,
    callback: MessageCallback,
  ) -> bool;
  fn v8__Isolate__AddNearHeapLimitCallback(
    isolate: *mut Isolate,
    callback: NearHeapLimitCallback,
    data: *mut c_void,
  );
  fn v8__Isolate__RemoveNearHeapLimitCallback(
    isolate: *mut Isolate,
    callback: NearHeapLimitCallback,
    heap_limit: usize,
  );
  fn v8__Isolate__SetOOMErrorHandler(
    isolate: *mut Isolate,
    callback: OomErrorCallback,
  );
  fn v8__Isolate__SetPrepareStackTraceCallback(
    isolate: *mut Isolate,
    callback: PrepareStackTraceCallback,
  );
  fn v8__Isolate__SetPromiseHook(isolate: *mut Isolate, hook: PromiseHook);
  fn v8__Isolate__SetPromiseRejectCallback(
    isolate: *mut Isolate,
    callback: PromiseRejectCallback,
  );
  fn v8__Isolate__SetHostInitializeImportMetaObjectCallback(
    isolate: *mut Isolate,
    callback: HostInitializeImportMetaObjectCallback,
  );
  fn v8__Isolate__SetHostImportModuleDynamicallyCallback(
    isolate: *mut Isolate,
    callback: HostImportModuleDynamicallyWithImportAssertionsCallback,
  );
  fn v8__Isolate__RequestInterrupt(
    isolate: *const Isolate,
    callback: InterruptCallback,
    data: *mut c_void,
  );
  fn v8__Isolate__TerminateExecution(isolate: *const Isolate);
  fn v8__Isolate__IsExecutionTerminating(isolate: *const Isolate) -> bool;
  fn v8__Isolate__CancelTerminateExecution(isolate: *const Isolate);
  fn v8__Isolate__GetMicrotasksPolicy(
    isolate: *const Isolate,
  ) -> MicrotasksPolicy;
  fn v8__Isolate__SetMicrotasksPolicy(
    isolate: *mut Isolate,
    policy: MicrotasksPolicy,
  );
  fn v8__Isolate__PerformMicrotaskCheckpoint(isolate: *mut Isolate);
  fn v8__Isolate__EnqueueMicrotask(
    isolate: *mut Isolate,
    function: *const Function,
  );
  fn v8__Isolate__SetAllowAtomicsWait(isolate: *mut Isolate, allow: bool);
  fn v8__Isolate__SetWasmStreamingCallback(
    isolate: *mut Isolate,
    callback: extern "C" fn(*const FunctionCallbackInfo),
  );
  fn v8__Isolate__HasPendingBackgroundTasks(isolate: *const Isolate) -> bool;

  fn v8__HeapProfiler__TakeHeapSnapshot(
    isolate: *mut Isolate,
    callback: extern "C" fn(*mut c_void, *const u8, usize) -> bool,
    arg: *mut c_void,
  );

  fn v8__HeapStatistics__CONSTRUCT(s: *mut MaybeUninit<HeapStatistics>);
  fn v8__HeapStatistics__total_heap_size(s: *const HeapStatistics) -> usize;
  fn v8__HeapStatistics__total_heap_size_executable(
    s: *const HeapStatistics,
  ) -> usize;
  fn v8__HeapStatistics__total_physical_size(s: *const HeapStatistics)
    -> usize;
  fn v8__HeapStatistics__total_available_size(
    s: *const HeapStatistics,
  ) -> usize;
  fn v8__HeapStatistics__total_global_handles_size(
    s: *const HeapStatistics,
  ) -> usize;
  fn v8__HeapStatistics__used_global_handles_size(
    s: *const HeapStatistics,
  ) -> usize;
  fn v8__HeapStatistics__used_heap_size(s: *const HeapStatistics) -> usize;
  fn v8__HeapStatistics__heap_size_limit(s: *const HeapStatistics) -> usize;
  fn v8__HeapStatistics__malloced_memory(s: *const HeapStatistics) -> usize;
  fn v8__HeapStatistics__external_memory(s: *const HeapStatistics) -> usize;
  fn v8__HeapStatistics__peak_malloced_memory(
    s: *const HeapStatistics,
  ) -> usize;
  fn v8__HeapStatistics__number_of_native_contexts(
    s: *const HeapStatistics,
  ) -> usize;
  fn v8__HeapStatistics__number_of_detached_contexts(
    s: *const HeapStatistics,
  ) -> usize;
  fn v8__HeapStatistics__does_zap_garbage(s: *const HeapStatistics) -> usize;
}

/// Isolate represents an isolated instance of the V8 engine.  V8 isolates have
/// completely separate states.  Objects from one isolate must not be used in
/// other isolates.  The embedder can create multiple isolates and use them in
/// parallel in multiple threads.  An isolate can be entered by at most one
/// thread at any given time.  The Locker/Unlocker API must be used to
/// synchronize.
///
/// rusty_v8 note: Unlike in the C++ API, the Isolate is entered when it is
/// constructed and exited when dropped.
#[repr(C)]
#[derive(Debug)]
pub struct Isolate(Opaque);

impl Isolate {
  const ANNEX_SLOT: u32 = 0;
  const CURRENT_SCOPE_DATA_SLOT: u32 = 1;
  const INTERNAL_SLOT_COUNT: u32 = 2;

  /// Creates a new isolate.  Does not change the currently entered
  /// isolate.
  ///
  /// When an isolate is no longer used its resources should be freed
  /// by calling V8::dispose().  Using the delete operator is not allowed.
  ///
  /// V8::initialize() must have run prior to this.
  #[allow(clippy::new_ret_no_self)]
  pub fn new(params: CreateParams) -> OwnedIsolate {
    crate::V8::assert_initialized();
    let (raw_create_params, create_param_allocations) = params.finalize();
    let cxx_isolate = unsafe { v8__Isolate__New(&raw_create_params) };
    let mut owned_isolate = OwnedIsolate::new(cxx_isolate);
    ScopeData::new_root(&mut owned_isolate);
    owned_isolate.create_annex(create_param_allocations);
    unsafe {
      owned_isolate.enter();
    }
    owned_isolate
  }

  /// Initial configuration parameters for a new Isolate.
  pub fn create_params() -> CreateParams {
    CreateParams::default()
  }

  pub fn thread_safe_handle(&self) -> IsolateHandle {
    IsolateHandle::new(self)
  }

  /// See [`IsolateHandle::terminate_execution`]
  pub fn terminate_execution(&self) -> bool {
    self.thread_safe_handle().terminate_execution()
  }

  /// See [`IsolateHandle::cancel_terminate_execution`]
  pub fn cancel_terminate_execution(&self) -> bool {
    self.thread_safe_handle().cancel_terminate_execution()
  }

  /// See [`IsolateHandle::is_execution_terminating`]
  pub fn is_execution_terminating(&self) -> bool {
    self.thread_safe_handle().is_execution_terminating()
  }

  pub(crate) fn create_annex(
    &mut self,
    create_param_allocations: Box<dyn Any>,
  ) {
    let annex_arc = Arc::new(IsolateAnnex::new(self, create_param_allocations));
    let annex_ptr = Arc::into_raw(annex_arc);
    unsafe {
      assert!(v8__Isolate__GetData(self, Self::ANNEX_SLOT).is_null());
      v8__Isolate__SetData(self, Self::ANNEX_SLOT, annex_ptr as *mut c_void);
    };
  }

  fn get_annex(&self) -> &IsolateAnnex {
    unsafe {
      &*(v8__Isolate__GetData(self, Self::ANNEX_SLOT) as *const _
        as *const IsolateAnnex)
    }
  }

  fn get_annex_mut(&mut self) -> &mut IsolateAnnex {
    unsafe {
      &mut *(v8__Isolate__GetData(self, Self::ANNEX_SLOT) as *mut IsolateAnnex)
    }
  }

  fn get_annex_arc(&self) -> Arc<IsolateAnnex> {
    let annex_ptr = self.get_annex();
    let annex_arc = unsafe { Arc::from_raw(annex_ptr) };
    Arc::into_raw(annex_arc.clone());
    annex_arc
  }

  /// Associate embedder-specific data with the isolate. `slot` has to be
  /// between 0 and `Isolate::get_number_of_data_slots()`.
  unsafe fn set_data(&mut self, slot: u32, ptr: *mut c_void) {
    v8__Isolate__SetData(self, slot + Self::INTERNAL_SLOT_COUNT, ptr)
  }

  /// Retrieve embedder-specific data from the isolate.
  /// Returns NULL if SetData has never been called for the given `slot`.
  fn get_data(&self, slot: u32) -> *mut c_void {
    unsafe { v8__Isolate__GetData(self, slot + Self::INTERNAL_SLOT_COUNT) }
  }

  /// Returns the maximum number of available embedder data slots. Valid slots
  /// are in the range of 0 - `Isolate::get_number_of_data_slots() - 1`.
  fn get_number_of_data_slots(&self) -> u32 {
    unsafe {
      v8__Isolate__GetNumberOfDataSlots(self) - Self::INTERNAL_SLOT_COUNT
    }
  }

  /// Returns a pointer to the `ScopeData` struct for the current scope.
  pub(crate) fn get_current_scope_data(&self) -> Option<NonNull<ScopeData>> {
    let scope_data_ptr =
      unsafe { v8__Isolate__GetData(self, Self::CURRENT_SCOPE_DATA_SLOT) };
    NonNull::new(scope_data_ptr).map(NonNull::cast)
  }

  /// Updates the slot that stores a `ScopeData` pointer for the current scope.
  pub(crate) fn set_current_scope_data(
    &mut self,
    scope_data: Option<NonNull<ScopeData>>,
  ) {
    let scope_data_ptr = scope_data
      .map(NonNull::cast)
      .map(NonNull::as_ptr)
      .unwrap_or_else(null_mut);
    unsafe {
      v8__Isolate__SetData(self, Self::CURRENT_SCOPE_DATA_SLOT, scope_data_ptr)
    };
  }

  /// Get a reference to embedder data added with `set_slot()`.
  pub fn get_slot<T: 'static>(&self) -> Option<&T> {
    let b = self.get_annex().slots.get(&TypeId::of::<T>())?;
    let r = <dyn Any>::downcast_ref::<T>(&**b).unwrap();
    Some(r)
  }

  /// Get a mutable reference to embedder data added with `set_slot()`.
  pub fn get_slot_mut<T: 'static>(&mut self) -> Option<&mut T> {
    let b = self.get_annex_mut().slots.get_mut(&TypeId::of::<T>())?;
    let r = <dyn Any>::downcast_mut::<T>(&mut **b).unwrap();
    Some(r)
  }

  /// Use with Isolate::get_slot and Isolate::get_slot_mut to associate state
  /// with an Isolate.
  ///
  /// This method gives ownership of value to the Isolate. Exactly one object of
  /// each type can be associated with an Isolate. If called more than once with
  /// an object of the same type, the earlier version will be dropped and
  /// replaced.
  ///
  /// Returns true if value was set without replacing an existing value.
  ///
  /// The value will be dropped when the isolate is dropped.
  pub fn set_slot<T: 'static>(&mut self, value: T) -> bool {
    self
      .get_annex_mut()
      .slots
      .insert(Any::type_id(&value), Box::new(value))
      .is_none()
  }

  /// Removes the embedder data added with `set_slot()` and returns it if it exists.
  pub fn remove_slot<T: 'static>(&mut self) -> Option<T> {
    let b = self.get_annex_mut().slots.remove(&TypeId::of::<T>())?;
    let v: T = *b.downcast::<T>().unwrap();
    Some(v)
  }

  /// Sets this isolate as the entered one for the current thread.
  /// Saves the previously entered one (if any), so that it can be
  /// restored when exiting.  Re-entering an isolate is allowed.
  ///
  /// rusty_v8 note: Unlike in the C++ API, the isolate is entered when it is
  /// constructed and exited when dropped.
  pub unsafe fn enter(&mut self) {
    v8__Isolate__Enter(self)
  }

  /// Exits this isolate by restoring the previously entered one in the
  /// current thread.  The isolate may still stay the same, if it was
  /// entered more than once.
  ///
  /// Requires: self == Isolate::GetCurrent().
  ///
  /// rusty_v8 note: Unlike in the C++ API, the isolate is entered when it is
  /// constructed and exited when dropped.
  pub unsafe fn exit(&mut self) {
    v8__Isolate__Exit(self)
  }

  /// Clears the set of objects held strongly by the heap. This set of
  /// objects are originally built when a WeakRef is created or
  /// successfully dereferenced.
  ///
  /// This is invoked automatically after microtasks are run. See
  /// MicrotasksPolicy for when microtasks are run.
  ///
  /// This needs to be manually invoked only if the embedder is manually
  /// running microtasks via a custom MicrotaskQueue class's PerformCheckpoint.
  /// In that case, it is the embedder's responsibility to make this call at a
  /// time which does not interrupt synchronous ECMAScript code execution.
  pub fn clear_kept_objects(&mut self) {
    unsafe { v8__Isolate__ClearKeptObjects(self) }
  }

  /// Optional notification that the system is running low on memory.
  /// V8 uses these notifications to attempt to free memory.
  pub fn low_memory_notification(&mut self) {
    unsafe { v8__Isolate__LowMemoryNotification(self) }
  }

  /// Get statistics about the heap memory usage.
  pub fn get_heap_statistics(&mut self, s: &mut HeapStatistics) {
    unsafe { v8__Isolate__GetHeapStatistics(self, s) }
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

  /// This specifies the callback called when the stack property of Error
  /// is accessed.
  ///
  /// PrepareStackTraceCallback is called when the stack property of an error is
  /// first accessed. The return value will be used as the stack value. If this
  /// callback is registed, the |Error.prepareStackTrace| API will be disabled.
  /// |sites| is an array of call sites, specified in
  /// https://v8.dev/docs/stack-trace-api
  pub fn set_prepare_stack_trace_callback<'s>(
    &mut self,
    callback: impl MapFnTo<PrepareStackTraceCallback<'s>>,
  ) {
    // Note: the C++ API returns a MaybeLocal but V8 asserts at runtime when
    // it's empty. That is, you can't return None and that's why the Rust API
    // expects Local<Value> instead of Option<Local<Value>>.
    unsafe {
      v8__Isolate__SetPrepareStackTraceCallback(self, callback.map_fn_to())
    };
  }

  /// Set the PromiseHook callback for various promise lifecycle
  /// events.
  pub fn set_promise_hook(&mut self, hook: PromiseHook) {
    unsafe { v8__Isolate__SetPromiseHook(self, hook) }
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

  /// This specifies the callback called by the upcoming dynamic
  /// import() language feature to load modules.
  pub fn set_host_import_module_dynamically_callback(
    &mut self,
    callback: HostImportModuleDynamicallyWithImportAssertionsCallback,
  ) {
    unsafe {
      v8__Isolate__SetHostImportModuleDynamicallyCallback(self, callback)
    }
  }

  /// Add a callback to invoke in case the heap size is close to the heap limit.
  /// If multiple callbacks are added, only the most recently added callback is
  /// invoked.
  #[allow(clippy::not_unsafe_ptr_arg_deref)] // False positive.
  pub fn add_near_heap_limit_callback(
    &mut self,
    callback: NearHeapLimitCallback,
    data: *mut c_void,
  ) {
    unsafe { v8__Isolate__AddNearHeapLimitCallback(self, callback, data) };
  }

  /// Remove the given callback and restore the heap limit to the given limit.
  /// If the given limit is zero, then it is ignored. If the current heap size
  /// is greater than the given limit, then the heap limit is restored to the
  /// minimal limit that is possible for the current heap size.
  pub fn remove_near_heap_limit_callback(
    &mut self,
    callback: NearHeapLimitCallback,
    heap_limit: usize,
  ) {
    unsafe {
      v8__Isolate__RemoveNearHeapLimitCallback(self, callback, heap_limit)
    };
  }

  pub fn set_oom_error_handler(&mut self, callback: OomErrorCallback) {
    unsafe { v8__Isolate__SetOOMErrorHandler(self, callback) };
  }

  /// Returns the policy controlling how Microtasks are invoked.
  pub fn get_microtasks_policy(&self) -> MicrotasksPolicy {
    unsafe { v8__Isolate__GetMicrotasksPolicy(self) }
  }

  /// Returns the policy controlling how Microtasks are invoked.
  pub fn set_microtasks_policy(&mut self, policy: MicrotasksPolicy) {
    unsafe { v8__Isolate__SetMicrotasksPolicy(self, policy) }
  }

  /// Runs the default MicrotaskQueue until it gets empty and perform other
  /// microtask checkpoint steps, such as calling ClearKeptObjects. Asserts that
  /// the MicrotasksPolicy is not kScoped. Any exceptions thrown by microtask
  /// callbacks are swallowed.
  pub fn perform_microtask_checkpoint(&mut self) {
    unsafe { v8__Isolate__PerformMicrotaskCheckpoint(self) }
  }

  /// An alias for PerformMicrotaskCheckpoint.
  #[deprecated(note = "Use Isolate::perform_microtask_checkpoint() instead")]
  pub fn run_microtasks(&mut self) {
    self.perform_microtask_checkpoint()
  }

  /// Enqueues the callback to the default MicrotaskQueue
  pub fn enqueue_microtask(&mut self, microtask: Local<Function>) {
    unsafe { v8__Isolate__EnqueueMicrotask(self, &*microtask) }
  }

  /// Set whether calling Atomics.wait (a function that may block) is allowed in
  /// this isolate. This can also be configured via
  /// CreateParams::allow_atomics_wait.
  pub fn set_allow_atomics_wait(&mut self, allow: bool) {
    unsafe { v8__Isolate__SetAllowAtomicsWait(self, allow) }
  }

  /// Embedder injection point for `WebAssembly.compileStreaming(source)`.
  /// The expectation is that the embedder sets it at most once.
  ///
  /// The callback receives the source argument (string, Promise, etc.)
  /// and an instance of [WasmStreaming]. The [WasmStreaming] instance
  /// can outlive the callback and is used to feed data chunks to V8
  /// asynchronously.
  pub fn set_wasm_streaming_callback<F>(&mut self, _: F)
  where
    F: UnitType + Fn(&mut HandleScope, Local<Value>, WasmStreaming),
  {
    unsafe { v8__Isolate__SetWasmStreamingCallback(self, trampoline::<F>()) }
  }

  /// Returns true if there is ongoing background work within V8 that will
  /// eventually post a foreground task, like asynchronous WebAssembly
  /// compilation.
  pub fn has_pending_background_tasks(&self) -> bool {
    unsafe { v8__Isolate__HasPendingBackgroundTasks(self) }
  }

  /// Disposes the isolate.  The isolate must not be entered by any
  /// thread to be disposable.
  unsafe fn dispose(&mut self) {
    // Drop the scope stack.
    ScopeData::drop_root(self);

    // Set the `isolate` pointer inside the annex struct to null, so any
    // IsolateHandle that outlives the isolate will know that it can't call
    // methods on the isolate.
    let annex = self.get_annex_mut();
    {
      let _lock = annex.isolate_mutex.lock().unwrap();
      annex.isolate = null_mut();
    }

    // Clear slots and drop owned objects that were taken out of `CreateParams`.
    annex.create_param_allocations = Box::new(());
    annex.slots.clear();

    // Subtract one from the Arc<IsolateAnnex> reference count.
    Arc::from_raw(annex);
    self.set_data(0, null_mut());

    // No test case in rusty_v8 show this, but there have been situations in
    // deno where dropping Annex before the states causes a segfault.
    v8__Isolate__Dispose(self)
  }

  /// Take a heap snapshot. The callback is invoked one or more times
  /// with byte slices containing the snapshot serialized as JSON.
  /// It's the callback's responsibility to reassemble them into
  /// a single document, e.g., by writing them to a file.
  /// Note that Chrome DevTools refuses to load snapshots without
  /// a .heapsnapshot suffix.
  pub fn take_heap_snapshot<F>(&mut self, mut callback: F)
  where
    F: FnMut(&[u8]) -> bool,
  {
    extern "C" fn trampoline<F>(
      arg: *mut c_void,
      data: *const u8,
      size: usize,
    ) -> bool
    where
      F: FnMut(&[u8]) -> bool,
    {
      let p = arg as *mut F;
      let callback = unsafe { &mut *p };
      let slice = unsafe { std::slice::from_raw_parts(data, size) };
      callback(slice)
    }

    let arg = &mut callback as *mut F as *mut c_void;
    unsafe { v8__HeapProfiler__TakeHeapSnapshot(self, trampoline::<F>, arg) }
  }
}

pub(crate) struct IsolateAnnex {
  create_param_allocations: Box<dyn Any>,
  slots: HashMap<TypeId, Box<dyn Any>, BuildTypeIdHasher>,
  // The `isolate` and `isolate_mutex` fields are there so an `IsolateHandle`
  // (which may outlive the isolate itself) can determine whether the isolate
  // is still alive, and if so, get a reference to it. Safety rules:
  // - The 'main thread' must lock the mutex and reset `isolate` to null just
  //   before the isolate is disposed.
  // - Any other thread must lock the mutex while it's reading/using the
  //   `isolate` pointer.
  isolate: *mut Isolate,
  isolate_mutex: Mutex<()>,
}

impl IsolateAnnex {
  fn new(
    isolate: &mut Isolate,
    create_param_allocations: Box<dyn Any>,
  ) -> Self {
    Self {
      create_param_allocations,
      slots: HashMap::default(),
      isolate,
      isolate_mutex: Mutex::new(()),
    }
  }
}

impl Debug for IsolateAnnex {
  fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
    f.debug_struct("IsolateAnnex")
      .field("isolate", &self.isolate)
      .field("isolate_mutex", &self.isolate_mutex)
      .finish()
  }
}

/// IsolateHandle is a thread-safe reference to an Isolate. It's main use is to
/// terminate execution of a running isolate from another thread.
///
/// It is created with Isolate::thread_safe_handle().
///
/// IsolateHandle is Cloneable, Send, and Sync.
#[derive(Clone, Debug)]
pub struct IsolateHandle(Arc<IsolateAnnex>);

unsafe impl Send for IsolateHandle {}
unsafe impl Sync for IsolateHandle {}

impl IsolateHandle {
  // This function is marked unsafe because it must be called only with either
  // IsolateAnnex::mutex locked, or from the main thread associated with the V8
  // isolate.
  pub(crate) unsafe fn get_isolate_ptr(&self) -> *mut Isolate {
    self.0.isolate
  }

  fn new(isolate: &Isolate) -> Self {
    Self(isolate.get_annex_arc())
  }

  /// Forcefully terminate the current thread of JavaScript execution
  /// in the given isolate.
  ///
  /// This method can be used by any thread even if that thread has not
  /// acquired the V8 lock with a Locker object.
  ///
  /// Returns false if Isolate was already destroyed.
  pub fn terminate_execution(&self) -> bool {
    let _lock = self.0.isolate_mutex.lock().unwrap();
    if self.0.isolate.is_null() {
      false
    } else {
      unsafe { v8__Isolate__TerminateExecution(self.0.isolate) };
      true
    }
  }

  /// Resume execution capability in the given isolate, whose execution
  /// was previously forcefully terminated using TerminateExecution().
  ///
  /// When execution is forcefully terminated using TerminateExecution(),
  /// the isolate can not resume execution until all JavaScript frames
  /// have propagated the uncatchable exception which is generated.  This
  /// method allows the program embedding the engine to handle the
  /// termination event and resume execution capability, even if
  /// JavaScript frames remain on the stack.
  ///
  /// This method can be used by any thread even if that thread has not
  /// acquired the V8 lock with a Locker object.
  ///
  /// Returns false if Isolate was already destroyed.
  pub fn cancel_terminate_execution(&self) -> bool {
    let _lock = self.0.isolate_mutex.lock().unwrap();
    if self.0.isolate.is_null() {
      false
    } else {
      unsafe { v8__Isolate__CancelTerminateExecution(self.0.isolate) };
      true
    }
  }

  /// Is V8 terminating JavaScript execution.
  ///
  /// Returns true if JavaScript execution is currently terminating
  /// because of a call to TerminateExecution.  In that case there are
  /// still JavaScript frames on the stack and the termination
  /// exception is still active.
  ///
  /// Returns false if Isolate was already destroyed.
  pub fn is_execution_terminating(&self) -> bool {
    let _lock = self.0.isolate_mutex.lock().unwrap();
    if self.0.isolate.is_null() {
      false
    } else {
      unsafe { v8__Isolate__IsExecutionTerminating(self.0.isolate) }
    }
  }

  /// Request V8 to interrupt long running JavaScript code and invoke
  /// the given |callback| passing the given |data| to it. After |callback|
  /// returns control will be returned to the JavaScript code.
  /// There may be a number of interrupt requests in flight.
  /// Can be called from another thread without acquiring a |Locker|.
  /// Registered |callback| must not reenter interrupted Isolate.
  ///
  /// Returns false if Isolate was already destroyed.
  // Clippy warns that this method is dereferencing a raw pointer, but it is
  // not: https://github.com/rust-lang/rust-clippy/issues/3045
  #[allow(clippy::not_unsafe_ptr_arg_deref)]
  pub fn request_interrupt(
    &self,
    callback: InterruptCallback,
    data: *mut c_void,
  ) -> bool {
    let _lock = self.0.isolate_mutex.lock().unwrap();
    if self.0.isolate.is_null() {
      false
    } else {
      unsafe { v8__Isolate__RequestInterrupt(self.0.isolate, callback, data) };
      true
    }
  }
}

/// Same as Isolate but gets disposed when it goes out of scope.
#[derive(Debug)]
pub struct OwnedIsolate {
  cxx_isolate: NonNull<Isolate>,
}

impl OwnedIsolate {
  pub(crate) fn new(cxx_isolate: *mut Isolate) -> Self {
    let cxx_isolate = NonNull::new(cxx_isolate).unwrap();
    Self { cxx_isolate }
  }
}

impl Drop for OwnedIsolate {
  fn drop(&mut self) {
    unsafe {
      self.exit();
      self.cxx_isolate.as_mut().dispose()
    }
  }
}

impl Deref for OwnedIsolate {
  type Target = Isolate;
  fn deref(&self) -> &Self::Target {
    unsafe { self.cxx_isolate.as_ref() }
  }
}

impl DerefMut for OwnedIsolate {
  fn deref_mut(&mut self) -> &mut Self::Target {
    unsafe { self.cxx_isolate.as_mut() }
  }
}

impl HeapStatistics {
  pub fn total_heap_size(&self) -> usize {
    unsafe { v8__HeapStatistics__total_heap_size(self) }
  }

  pub fn total_heap_size_executable(&self) -> usize {
    unsafe { v8__HeapStatistics__total_heap_size_executable(self) }
  }

  pub fn total_physical_size(&self) -> usize {
    unsafe { v8__HeapStatistics__total_physical_size(self) }
  }

  pub fn total_available_size(&self) -> usize {
    unsafe { v8__HeapStatistics__total_available_size(self) }
  }

  pub fn total_global_handles_size(&self) -> usize {
    unsafe { v8__HeapStatistics__total_global_handles_size(self) }
  }

  pub fn used_global_handles_size(&self) -> usize {
    unsafe { v8__HeapStatistics__used_global_handles_size(self) }
  }

  pub fn used_heap_size(&self) -> usize {
    unsafe { v8__HeapStatistics__used_heap_size(self) }
  }

  pub fn heap_size_limit(&self) -> usize {
    unsafe { v8__HeapStatistics__heap_size_limit(self) }
  }

  pub fn malloced_memory(&self) -> usize {
    unsafe { v8__HeapStatistics__malloced_memory(self) }
  }

  pub fn external_memory(&self) -> usize {
    unsafe { v8__HeapStatistics__external_memory(self) }
  }

  pub fn peak_malloced_memory(&self) -> usize {
    unsafe { v8__HeapStatistics__peak_malloced_memory(self) }
  }

  pub fn number_of_native_contexts(&self) -> usize {
    unsafe { v8__HeapStatistics__number_of_native_contexts(self) }
  }

  pub fn number_of_detached_contexts(&self) -> usize {
    unsafe { v8__HeapStatistics__number_of_detached_contexts(self) }
  }

  /// Returns a 0/1 boolean, which signifies whether the V8 overwrite heap
  /// garbage with a bit pattern.
  pub fn does_zap_garbage(&self) -> usize {
    unsafe { v8__HeapStatistics__does_zap_garbage(self) }
  }
}

impl Default for HeapStatistics {
  fn default() -> Self {
    let mut s = MaybeUninit::<Self>::uninit();
    unsafe {
      v8__HeapStatistics__CONSTRUCT(&mut s);
      s.assume_init()
    }
  }
}

impl<'s, F> MapFnFrom<F> for PrepareStackTraceCallback<'s>
where
  F: UnitType
    + Fn(
      &mut HandleScope<'s>,
      Local<'s, Value>,
      Local<'s, Array>,
    ) -> Local<'s, Value>,
{
  // Windows x64 ABI: MaybeLocal<Value> returned on the stack.
  #[cfg(target_os = "windows")]
  fn mapping() -> Self {
    let f = |ret_ptr, context, error, sites| {
      let mut scope: CallbackScope = unsafe { CallbackScope::new(context) };
      let r = (F::get())(&mut scope, error, sites);
      unsafe { std::ptr::write(ret_ptr, &*r as *const _) };
      ret_ptr
    };
    f.to_c_fn()
  }

  // System V ABI: MaybeLocal<Value> returned in a register.
  #[cfg(not(target_os = "windows"))]
  fn mapping() -> Self {
    let f = |context, error, sites| {
      let mut scope: CallbackScope = unsafe { CallbackScope::new(context) };
      let r = (F::get())(&mut scope, error, sites);
      &*r as *const _
    };
    f.to_c_fn()
  }
}
