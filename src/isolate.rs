// Copyright 2019-2020 the Deno authors. All rights reserved. MIT license.
use crate::isolate_create_params::raw;
use crate::isolate_create_params::CreateParams;
use crate::promise::PromiseRejectMessage;
use crate::scope::data::ScopeData;
use crate::support::Opaque;
use crate::Context;
use crate::Function;
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
use std::cell::{Ref, RefCell, RefMut};
use std::collections::HashMap;
use std::ffi::c_void;
use std::ops::Deref;
use std::ops::DerefMut;
use std::ptr::null_mut;
use std::ptr::NonNull;
use std::sync::Arc;
use std::sync::Mutex;

pub type MessageCallback = extern "C" fn(Local<Message>, Local<Value>);

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

/// HostImportModuleDynamicallyCallback is called when we require the
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
pub type HostImportModuleDynamicallyCallback = extern "C" fn(
  Local<Context>,
  Local<ScriptOrModule>,
  Local<String>,
) -> *mut Promise;

pub type InterruptCallback =
  extern "C" fn(isolate: &mut Isolate, data: *mut c_void);

pub type NearHeapLimitCallback = extern "C" fn(
  data: *mut c_void,
  current_heap_limit: usize,
  initial_heap_limit: usize,
) -> usize;

#[repr(C)]
pub struct ModifyCodeGenerationFromStringsResult<'s> {
  pub codegen_allowed: bool,
  pub modified_source: Option<Local<'s, String>>,
}

pub type ModifyCodeGenerationFromStringsCallback<'s> =
  extern "C" fn(
    ctx: Local<'s, Context>,
    code: Local<'s, Value>,
  ) -> ModifyCodeGenerationFromStringsResult<'s>;

extern "C" {
  fn v8__Isolate__New(params: *const raw::CreateParams) -> *mut Isolate;
  fn v8__Isolate__Dispose(this: *mut Isolate);
  fn v8__Isolate__SetData(this: *mut Isolate, slot: u32, data: *mut c_void);
  fn v8__Isolate__GetData(this: *const Isolate, slot: u32) -> *mut c_void;
  fn v8__Isolate__GetNumberOfDataSlots(this: *const Isolate) -> u32;
  fn v8__Isolate__Enter(this: *mut Isolate);
  fn v8__Isolate__Exit(this: *mut Isolate);
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
    callback: HostImportModuleDynamicallyCallback,
  );
  // We use Option<NonNull<T>> which _is_ FFI-safe.
  // See https://doc.rust-lang.org/nomicon/other-reprs.html
  #[allow(improper_ctypes)]
  fn v8__Isolate__SetModifyCodeGenerationFromStringsCallback(
    isolate: *mut Isolate,
    callback: ModifyCodeGenerationFromStringsCallback,
  );
  fn v8__Isolate__RequestInterrupt(
    isolate: *const Isolate,
    callback: InterruptCallback,
    data: *mut c_void,
  );
  fn v8__Isolate__TerminateExecution(isolate: *const Isolate);
  fn v8__Isolate__IsExecutionTerminating(isolate: *const Isolate) -> bool;
  fn v8__Isolate__CancelTerminateExecution(isolate: *const Isolate);
  fn v8__Isolate__RunMicrotasks(isolate: *mut Isolate);
  fn v8__Isolate__EnqueueMicrotask(
    isolate: *mut Isolate,
    function: *const Function,
  );

  fn v8__HeapProfiler__TakeHeapSnapshot(
    isolate: *mut Isolate,
    callback: extern "C" fn(*mut c_void, *const u8, usize) -> bool,
    arg: *mut c_void,
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
    owned_isolate
  }

  /// Initial configuration parameters for a new Isolate.
  pub fn create_params() -> CreateParams {
    CreateParams::default()
  }

  pub fn thread_safe_handle(&mut self) -> IsolateHandle {
    IsolateHandle::new(self)
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

  /// Get mutable reference to embedder data.
  pub fn get_slot_mut<T: 'static>(&self) -> Option<RefMut<T>> {
    let cell = self.get_annex().slots.get(&TypeId::of::<T>())?;
    let ref_mut = cell.try_borrow_mut().ok()?;
    let ref_mut = RefMut::map(ref_mut, |box_any| {
      let mut_any = &mut **box_any;
      Any::downcast_mut::<T>(mut_any).unwrap()
    });
    Some(ref_mut)
  }

  /// Get reference to embedder data.
  pub fn get_slot<T: 'static>(&self) -> Option<Ref<T>> {
    let cell = self.get_annex().slots.get(&TypeId::of::<T>())?;
    let r = cell.try_borrow().ok()?;
    Some(Ref::map(r, |box_any| {
      let a = &**box_any;
      Any::downcast_ref::<T>(a).unwrap()
    }))
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
      .insert(Any::type_id(&value), RefCell::new(Box::new(value)))
      .is_none()
  }

  /// Sets this isolate as the entered one for the current thread.
  /// Saves the previously entered one (if any), so that it can be
  /// restored when exiting.  Re-entering an isolate is allowed.
  pub(crate) fn enter_isolate(&mut self) {
    unsafe { v8__Isolate__Enter(self) }
  }

  /// Exits this isolate by restoring the previously entered one in the
  /// current thread.  The isolate may still stay the same, if it was
  /// entered more than once.
  ///
  /// Requires: self == Isolate::GetCurrent().
  pub(crate) fn exit_isolate(&mut self) {
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

  /// This specifies the callback called by the upcoming dynamic
  /// import() language feature to load modules.
  pub fn set_host_import_module_dynamically_callback(
    &mut self,
    callback: HostImportModuleDynamicallyCallback,
  ) {
    unsafe {
      v8__Isolate__SetHostImportModuleDynamicallyCallback(self, callback)
    }
  }

  /// This specifies the callback called by v8 when JS is trying to dynamically execute
  /// code using `eval` or the `Function` constructor.
  ///
  /// The callback can decide whether to allow code generation and, if so, modify
  /// the source code beforehand.
  pub fn set_modify_code_generation_from_strings_callback(
    &mut self,
    callback: ModifyCodeGenerationFromStringsCallback,
  ) {
    unsafe {
      v8__Isolate__SetModifyCodeGenerationFromStringsCallback(self, callback)
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

  /// Runs the default MicrotaskQueue until it gets empty.
  /// Any exceptions thrown by microtask callbacks are swallowed.
  pub fn run_microtasks(&mut self) {
    unsafe { v8__Isolate__RunMicrotasks(self) }
  }

  /// Enqueues the callback to the default MicrotaskQueue
  pub fn enqueue_microtask(&mut self, microtask: Local<Function>) {
    unsafe { v8__Isolate__EnqueueMicrotask(self, &*microtask) }
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
  slots: HashMap<TypeId, RefCell<Box<dyn Any>>>,
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
      slots: HashMap::new(),
      isolate,
      isolate_mutex: Mutex::new(()),
    }
  }
}

/// IsolateHandle is a thread-safe reference to an Isolate. It's main use is to
/// terminate execution of a running isolate from another thread.
///
/// It is created with Isolate::thread_safe_handle().
///
/// IsolateHandle is Cloneable, Send, and Sync.
#[derive(Clone)]
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

  fn new(isolate: &mut Isolate) -> Self {
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
    unsafe { self.cxx_isolate.as_mut().dispose() }
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

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn block_eval() {
    extern "C" fn block_code_execution(
      _ctx: Local<Context>,
      _src: Local<Value>,
    ) -> ModifyCodeGenerationFromStringsResult<'static> {
      ModifyCodeGenerationFromStringsResult {
        codegen_allowed: false,
        modified_source: None,
      }
    }

    let mut isolate = Isolate::new(CreateParams::default());
    isolate
      .set_modify_code_generation_from_strings_callback(block_code_execution);
  }
}
