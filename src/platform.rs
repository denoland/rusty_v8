use crate::Isolate;
use crate::isolate::RealIsolate;
use crate::support::int;

use crate::support::Opaque;
use crate::support::Shared;
use crate::support::SharedPtrBase;
use crate::support::SharedRef;
use crate::support::UniquePtr;
use crate::support::UniqueRef;
use crate::support::long;

unsafe extern "C" {
  fn v8__Platform__NewDefaultPlatform(
    thread_pool_size: int,
    idle_task_support: bool,
  ) -> *mut Platform;
  fn v8__Platform__NewUnprotectedDefaultPlatform(
    thread_pool_size: int,
    idle_task_support: bool,
  ) -> *mut Platform;
  fn v8__Platform__NewSingleThreadedDefaultPlatform(
    idle_task_support: bool,
  ) -> *mut Platform;
  fn v8__Platform__NewCustomPlatform(
    thread_pool_size: int,
    idle_task_support: bool,
    context: *mut std::ffi::c_void,
  ) -> *mut Platform;
  fn v8__Platform__DELETE(this: *mut Platform);

  fn v8__Platform__PumpMessageLoop(
    platform: *mut Platform,
    isolate: *mut RealIsolate,
    wait_for_work: bool,
  ) -> bool;

  fn v8__Platform__RunIdleTasks(
    platform: *mut Platform,
    isolate: *mut RealIsolate,
    idle_time_in_seconds: f64,
  );

  fn v8__Platform__NotifyIsolateShutdown(
    platform: *mut Platform,
    isolate: *mut RealIsolate,
  );

  fn std__shared_ptr__v8__Platform__CONVERT__std__unique_ptr(
    unique_ptr: UniquePtr<Platform>,
  ) -> SharedPtrBase<Platform>;
  fn std__shared_ptr__v8__Platform__get(
    ptr: *const SharedPtrBase<Platform>,
  ) -> *mut Platform;
  fn std__shared_ptr__v8__Platform__COPY(
    ptr: *const SharedPtrBase<Platform>,
  ) -> SharedPtrBase<Platform>;
  fn std__shared_ptr__v8__Platform__reset(ptr: *mut SharedPtrBase<Platform>);
  fn std__shared_ptr__v8__Platform__use_count(
    ptr: *const SharedPtrBase<Platform>,
  ) -> long;
}

#[repr(C)]
#[derive(Debug)]
pub struct Platform(Opaque);

/// Trait for customizing platform behavior, following the same pattern as
/// [`V8InspectorClientImpl`](crate::inspector::V8InspectorClientImpl).
///
/// Implement this trait to receive callbacks for overridden C++ virtual
/// methods on the `DefaultPlatform` and its per-isolate `TaskRunner`.
///
/// The C++ `CustomPlatform` wraps each isolate's `TaskRunner` so that
/// every `PostTask` / `PostDelayedTask` / etc. call is forwarded to the
/// default implementation *and* notifies Rust through the corresponding
/// trait method.
///
/// All methods have default no-op implementations; override only what
/// you need.
///
/// Implementations must be `Send + Sync` as callbacks may fire from any
/// thread.
#[allow(unused_variables)]
pub trait PlatformImpl: Send + Sync {
  // ---- TaskRunner virtual methods ----

  /// Called when `TaskRunner::PostTask` is invoked for the given isolate.
  ///
  /// The task itself has already been forwarded to the default platform's
  /// queue and will be executed by `PumpMessageLoop`. This callback is a
  /// notification that a new task is available.
  ///
  /// May be called from ANY thread (V8 background threads, etc.).
  fn post_task(&self, isolate_ptr: *mut std::ffi::c_void) {}

  /// Called when `TaskRunner::PostNonNestableTask` is invoked.
  ///
  /// Same semantics as [`post_task`](Self::post_task).
  fn post_non_nestable_task(&self, isolate_ptr: *mut std::ffi::c_void) {}

  /// Called when `TaskRunner::PostDelayedTask` is invoked.
  ///
  /// The task has been forwarded to the default runner's delayed queue.
  /// `delay_in_seconds` is the delay before the task should execute.
  /// Embedders should schedule a wake-up after this delay.
  ///
  /// May be called from ANY thread.
  fn post_delayed_task(
    &self,
    isolate_ptr: *mut std::ffi::c_void,
    delay_in_seconds: f64,
  ) {
  }

  /// Called when `TaskRunner::PostNonNestableDelayedTask` is invoked.
  ///
  /// Same semantics as [`post_delayed_task`](Self::post_delayed_task).
  fn post_non_nestable_delayed_task(
    &self,
    isolate_ptr: *mut std::ffi::c_void,
    delay_in_seconds: f64,
  ) {
  }

  /// Called when `TaskRunner::PostIdleTask` is invoked.
  ///
  /// Same semantics as [`post_task`](Self::post_task).
  fn post_idle_task(&self, isolate_ptr: *mut std::ffi::c_void) {}

  // ---- Platform virtual methods ----

  /// Called when `Platform::NotifyIsolateShutdown` is invoked.
  ///
  /// The default `DefaultPlatform` cleanup runs after this callback
  /// returns.
  fn notify_isolate_shutdown(&self, isolate_ptr: *mut std::ffi::c_void) {}
}

// FFI callbacks called from C++ CustomPlatform/CustomTaskRunner.
// `context` is a raw pointer to a `Box<dyn PlatformImpl>`.

#[unsafe(no_mangle)]
unsafe extern "C" fn v8__Platform__CustomPlatform__BASE__PostTask(
  context: *mut std::ffi::c_void,
  isolate: *mut std::ffi::c_void,
) {
  let imp = unsafe { &*(context as *const Box<dyn PlatformImpl>) };
  imp.post_task(isolate);
}

#[unsafe(no_mangle)]
unsafe extern "C" fn v8__Platform__CustomPlatform__BASE__PostNonNestableTask(
  context: *mut std::ffi::c_void,
  isolate: *mut std::ffi::c_void,
) {
  let imp = unsafe { &*(context as *const Box<dyn PlatformImpl>) };
  imp.post_non_nestable_task(isolate);
}

#[unsafe(no_mangle)]
unsafe extern "C" fn v8__Platform__CustomPlatform__BASE__PostDelayedTask(
  context: *mut std::ffi::c_void,
  isolate: *mut std::ffi::c_void,
  delay_in_seconds: f64,
) {
  let imp = unsafe { &*(context as *const Box<dyn PlatformImpl>) };
  imp.post_delayed_task(isolate, delay_in_seconds);
}

#[unsafe(no_mangle)]
unsafe extern "C" fn v8__Platform__CustomPlatform__BASE__PostNonNestableDelayedTask(
  context: *mut std::ffi::c_void,
  isolate: *mut std::ffi::c_void,
  delay_in_seconds: f64,
) {
  let imp = unsafe { &*(context as *const Box<dyn PlatformImpl>) };
  imp.post_non_nestable_delayed_task(isolate, delay_in_seconds);
}

#[unsafe(no_mangle)]
unsafe extern "C" fn v8__Platform__CustomPlatform__BASE__PostIdleTask(
  context: *mut std::ffi::c_void,
  isolate: *mut std::ffi::c_void,
) {
  let imp = unsafe { &*(context as *const Box<dyn PlatformImpl>) };
  imp.post_idle_task(isolate);
}

#[unsafe(no_mangle)]
unsafe extern "C" fn v8__Platform__CustomPlatform__BASE__NotifyIsolateShutdown(
  context: *mut std::ffi::c_void,
  isolate: *mut std::ffi::c_void,
) {
  let imp = unsafe { &*(context as *const Box<dyn PlatformImpl>) };
  imp.notify_isolate_shutdown(isolate);
}

#[unsafe(no_mangle)]
unsafe extern "C" fn v8__Platform__CustomPlatform__BASE__DROP(
  context: *mut std::ffi::c_void,
) {
  unsafe {
    let _ = Box::from_raw(context as *mut Box<dyn PlatformImpl>);
  }
}

/// Returns a new instance of the default v8::Platform implementation.
///
/// |thread_pool_size| is the number of worker threads to allocate for
/// background jobs. If a value of zero is passed, a suitable default
/// based on the current number of processors online will be chosen.
/// If |idle_task_support| is enabled then the platform will accept idle
/// tasks (IdleTasksEnabled will return true) and will rely on the embedder
/// calling v8::platform::RunIdleTasks to process the idle tasks.
///
/// The default platform for v8 may include restrictions and caveats on thread
/// creation and initialization. This platform should only be used in cases
/// where v8 can be reliably initialized on the application's main thread, or
/// the parent thread to all threads in the system that will use v8.
///
/// One example of a restriction is the use of Memory Protection Keys (pkeys) on
/// modern Linux systems using modern Intel/AMD processors. This particular
/// technology requires that all threads using v8 are created as descendent
/// threads of the thread that called `v8::Initialize`.
#[inline(always)]
pub fn new_default_platform(
  thread_pool_size: u32,
  idle_task_support: bool,
) -> UniqueRef<Platform> {
  Platform::new(thread_pool_size, idle_task_support)
}

/// Creates a platform that is identical to the default platform, but does not
/// enforce thread-isolated allocations. This may reduce security in some cases,
/// so this method should be used with caution in cases where the threading
/// guarantees of `new_default_platform` cannot be upheld (generally for tests).
#[inline(always)]
pub fn new_unprotected_default_platform(
  thread_pool_size: u32,
  idle_task_support: bool,
) -> UniqueRef<Platform> {
  Platform::new_unprotected(thread_pool_size, idle_task_support)
}

/// The same as new_default_platform() but disables the worker thread pool.
/// It must be used with the --single-threaded V8 flag.
///
/// If |idle_task_support| is enabled then the platform will accept idle
/// tasks (IdleTasksEnabled will return true) and will rely on the embedder
/// calling v8::platform::RunIdleTasks to process the idle tasks.
#[inline(always)]
pub fn new_single_threaded_default_platform(
  idle_task_support: bool,
) -> UniqueRef<Platform> {
  Platform::new_single_threaded(idle_task_support)
}

/// Creates a custom platform backed by `DefaultPlatform` that delegates
/// virtual method overrides to the provided [`PlatformImpl`] trait object.
///
/// This follows the same pattern as
/// [`V8InspectorClient::new`](crate::inspector::V8InspectorClient::new).
///
/// Thread-isolated allocations are disabled (same as
/// `new_unprotected_default_platform`).
#[inline(always)]
pub fn new_custom_platform(
  thread_pool_size: u32,
  idle_task_support: bool,
  platform_impl: impl PlatformImpl + 'static,
) -> UniqueRef<Platform> {
  Platform::new_custom(thread_pool_size, idle_task_support, platform_impl)
}

impl Platform {
  /// Returns a new instance of the default v8::Platform implementation.
  ///
  /// |thread_pool_size| is the number of worker threads to allocate for
  /// background jobs. If a value of zero is passed, a suitable default
  /// based on the current number of processors online will be chosen.
  /// If |idle_task_support| is enabled then the platform will accept idle
  /// tasks (IdleTasksEnabled will return true) and will rely on the embedder
  /// calling v8::platform::RunIdleTasks to process the idle tasks.
  ///
  /// The default platform for v8 may include restrictions and caveats on thread
  /// creation and initialization. This platform should only be used in cases
  /// where v8 can be reliably initialized on the application's main thread, or
  /// the parent thread to all threads in the system that will use v8.
  ///
  /// One example of a restriction is the use of Memory Protection Keys (pkeys)
  /// on modern Linux systems using modern Intel/AMD processors. This particular
  /// technology requires that all threads using v8 are created as descendent
  /// threads of the thread that called `v8::Initialize`.
  #[inline(always)]
  pub fn new(
    thread_pool_size: u32,
    idle_task_support: bool,
  ) -> UniqueRef<Self> {
    unsafe {
      UniqueRef::from_raw(v8__Platform__NewDefaultPlatform(
        thread_pool_size.min(16) as i32,
        idle_task_support,
      ))
    }
  }

  /// Creates a platform that is identical to the default platform, but does not
  /// enforce thread-isolated allocations. This may reduce security in some
  /// cases, so this method should be used with caution in cases where the
  /// threading guarantees of `new_default_platform` cannot be upheld (generally
  /// for tests).
  #[inline(always)]
  pub fn new_unprotected(
    thread_pool_size: u32,
    idle_task_support: bool,
  ) -> UniqueRef<Self> {
    unsafe {
      UniqueRef::from_raw(v8__Platform__NewUnprotectedDefaultPlatform(
        thread_pool_size.min(16) as i32,
        idle_task_support,
      ))
    }
  }

  /// The same as new() but disables the worker thread pool.
  /// It must be used with the --single-threaded V8 flag.
  ///
  /// If |idle_task_support| is enabled then the platform will accept idle
  /// tasks (IdleTasksEnabled will return true) and will rely on the embedder
  /// calling v8::platform::RunIdleTasks to process the idle tasks.
  #[inline(always)]
  pub fn new_single_threaded(idle_task_support: bool) -> UniqueRef<Self> {
    unsafe {
      UniqueRef::from_raw(v8__Platform__NewSingleThreadedDefaultPlatform(
        idle_task_support,
      ))
    }
  }

  /// Creates a custom platform backed by `DefaultPlatform` that delegates
  /// virtual method overrides to the provided [`PlatformImpl`] trait object.
  ///
  /// The trait object is owned by the platform and will be dropped when the
  /// platform is destroyed.
  #[inline(always)]
  pub fn new_custom(
    thread_pool_size: u32,
    idle_task_support: bool,
    platform_impl: impl PlatformImpl + 'static,
  ) -> UniqueRef<Self> {
    // Double-box: inner Box<dyn> is a fat pointer, outer Box gives us a
    // thin pointer we can pass through C++ void*.
    let boxed: Box<dyn PlatformImpl> = Box::new(platform_impl);
    let context = Box::into_raw(Box::new(boxed)) as *mut std::ffi::c_void;
    // thread_pool_size clamping (0 → hardware_concurrency, max 16) is
    // handled on the C++ side in v8__Platform__NewCustomPlatform.
    unsafe {
      UniqueRef::from_raw(v8__Platform__NewCustomPlatform(
        thread_pool_size as i32,
        idle_task_support,
        context,
      ))
    }
  }
}

impl Platform {
  /// Pumps the message loop for the given isolate.
  ///
  /// The caller has to make sure that this is called from the right thread.
  /// Returns true if a task was executed, and false otherwise. If the call to
  /// PumpMessageLoop is nested within another call to PumpMessageLoop, only
  /// nestable tasks may run. Otherwise, any task may run. Unless requested through
  /// the |wait_for_work| parameter, this call does not block if no task is pending.
  #[inline(always)]
  pub fn pump_message_loop(
    platform: &SharedRef<Self>,
    isolate: &Isolate,
    wait_for_work: bool,
  ) -> bool {
    unsafe {
      v8__Platform__PumpMessageLoop(
        &**platform as *const Self as *mut _,
        isolate.as_real_ptr(),
        wait_for_work,
      )
    }
  }

  /// Runs pending idle tasks for at most |idle_time_in_seconds| seconds.
  ///
  /// The caller has to make sure that this is called from the right thread.
  /// This call does not block if no task is pending.
  #[inline(always)]
  pub fn run_idle_tasks(
    platform: &SharedRef<Self>,
    isolate: &Isolate,
    idle_time_in_seconds: f64,
  ) {
    unsafe {
      v8__Platform__RunIdleTasks(
        &**platform as *const Self as *mut _,
        isolate.as_real_ptr(),
        idle_time_in_seconds,
      );
    }
  }

  /// Notifies the given platform about the Isolate getting deleted soon. Has to
  /// be called for all Isolates which are deleted - unless we're shutting down
  /// the platform.
  ///
  /// The |platform| has to be created using |NewDefaultPlatform|.
  #[inline(always)]
  pub(crate) unsafe fn notify_isolate_shutdown(
    platform: &SharedRef<Self>,
    isolate: &Isolate,
  ) {
    unsafe {
      v8__Platform__NotifyIsolateShutdown(
        &**platform as *const Self as *mut _,
        isolate.as_real_ptr(),
      );
    }
  }
}

impl Shared for Platform {
  fn from_unique_ptr(unique_ptr: UniquePtr<Self>) -> SharedPtrBase<Self> {
    unsafe {
      std__shared_ptr__v8__Platform__CONVERT__std__unique_ptr(unique_ptr)
    }
  }
  fn get(ptr: &SharedPtrBase<Self>) -> *const Self {
    unsafe { std__shared_ptr__v8__Platform__get(ptr) }
  }
  fn clone(ptr: &SharedPtrBase<Self>) -> SharedPtrBase<Self> {
    unsafe { std__shared_ptr__v8__Platform__COPY(ptr) }
  }
  fn reset(ptr: &mut SharedPtrBase<Self>) {
    unsafe { std__shared_ptr__v8__Platform__reset(ptr) }
  }
  fn use_count(ptr: &SharedPtrBase<Self>) -> long {
    unsafe { std__shared_ptr__v8__Platform__use_count(ptr) }
  }
}

impl Drop for Platform {
  fn drop(&mut self) {
    unsafe { v8__Platform__DELETE(self) };
  }
}
