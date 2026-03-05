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

  fn v8__Task__Run(task: *mut RawTask);
  fn v8__Task__DELETE(task: *mut RawTask);
  fn v8__IdleTask__Run(task: *mut RawIdleTask, deadline_in_seconds: f64);
  fn v8__IdleTask__DELETE(task: *mut RawIdleTask);

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

// Opaque C++ types for v8::Task and v8::IdleTask.
#[repr(C)]
struct RawTask(Opaque);
#[repr(C)]
struct RawIdleTask(Opaque);

/// An owned handle to a C++ `v8::Task`.
///
/// Call [`run()`](Task::run) to execute the task. The underlying C++ object
/// is deleted when this value is dropped (whether or not it was run).
pub struct Task(*mut RawTask);

// SAFETY: v8::Task instances are designed to be posted across threads.
unsafe impl Send for Task {}

impl Task {
  /// Execute the task. The task is consumed and the underlying C++ object
  /// is deleted after execution.
  pub fn run(self) {
    let ptr = self.0;
    std::mem::forget(self);
    unsafe {
      v8__Task__Run(ptr);
      v8__Task__DELETE(ptr);
    }
  }
}

impl Drop for Task {
  fn drop(&mut self) {
    unsafe { v8__Task__DELETE(self.0) }
  }
}

/// An owned handle to a C++ `v8::IdleTask`.
///
/// Call [`run()`](IdleTask::run) to execute the task with a deadline.
/// The underlying C++ object is deleted when this value is dropped.
pub struct IdleTask(*mut RawIdleTask);

// SAFETY: v8::IdleTask instances are designed to be posted across threads.
unsafe impl Send for IdleTask {}

impl IdleTask {
  /// Execute the idle task with the given deadline. The task is consumed
  /// and the underlying C++ object is deleted after execution.
  pub fn run(self, deadline_in_seconds: f64) {
    let ptr = self.0;
    std::mem::forget(self);
    unsafe {
      v8__IdleTask__Run(ptr, deadline_in_seconds);
      v8__IdleTask__DELETE(ptr);
    }
  }
}

impl Drop for IdleTask {
  fn drop(&mut self) {
    unsafe { v8__IdleTask__DELETE(self.0) }
  }
}

#[repr(C)]
#[derive(Debug)]
pub struct Platform(Opaque);

/// Trait representing the `v8::TaskRunner` virtual methods.
///
/// Implement this trait to control how V8 foreground tasks are executed.
/// For example, an embedder using tokio might spawn tasks on its runtime:
///
/// ```ignore
/// impl TaskRunnerImpl for MyRunner {
///   fn post_task(&self, task: Task) {
///     tokio::task::spawn_blocking(move || task.run());
///   }
///   fn post_delayed_task(&self, task: Task, delay: f64) {
///     tokio::spawn(async move {
///       tokio::time::sleep(Duration::from_secs_f64(delay)).await;
///       tokio::task::spawn_blocking(move || task.run());
///     });
///   }
/// }
/// ```
///
/// Implementations must be `Send + Sync` as methods may be called from
/// any thread.
#[allow(unused_variables)]
pub trait TaskRunnerImpl: Send + Sync {
  /// Called when V8 posts a foreground task for immediate execution.
  fn post_task(&self, task: Task);

  /// Called when V8 posts a foreground task to run after `delay_in_seconds`.
  fn post_delayed_task(&self, task: Task, delay_in_seconds: f64);

  /// Called when V8 posts a non-nestable foreground task.
  /// Default: delegates to [`post_task`](TaskRunnerImpl::post_task).
  fn post_non_nestable_task(&self, task: Task) {
    self.post_task(task);
  }

  /// Called when V8 posts a non-nestable delayed foreground task.
  /// Default: delegates to
  /// [`post_delayed_task`](TaskRunnerImpl::post_delayed_task).
  fn post_non_nestable_delayed_task(&self, task: Task, delay_in_seconds: f64) {
    self.post_delayed_task(task, delay_in_seconds);
  }

  /// Called when V8 posts an idle task.
  /// Default: drops the task (idle tasks not supported).
  fn post_idle_task(&self, task: IdleTask) {
    drop(task);
  }

  /// Whether this runner supports idle tasks.
  /// Default: `false`.
  fn idle_tasks_enabled(&self) -> bool {
    false
  }

  /// Whether this runner supports non-nestable tasks.
  /// Default: `true`.
  fn non_nestable_tasks_enabled(&self) -> bool {
    true
  }

  /// Whether this runner supports non-nestable delayed tasks.
  /// Default: `true`.
  fn non_nestable_delayed_tasks_enabled(&self) -> bool {
    true
  }
}

/// Trait representing `v8::Platform` virtual methods, following the same
/// pattern as
/// [`V8InspectorClientImpl`](crate::inspector::V8InspectorClientImpl).
///
/// Implement this trait to provide custom foreground task runners for
/// each isolate.
///
/// Implementations must be `Send + Sync` as methods may be called from
/// any thread.
#[allow(unused_variables)]
pub trait PlatformImpl: Send + Sync {
  /// Returns a custom foreground task runner for the given isolate.
  ///
  /// `isolate_ptr` is the raw `v8::Isolate*` pointer. The returned task
  /// runner will be cached per isolate and cleaned up on isolate shutdown.
  ///
  /// Return `None` to use the default `DefaultPlatform` task runner.
  fn get_foreground_task_runner(
    &self,
    isolate_ptr: *mut std::ffi::c_void,
  ) -> Option<Box<dyn TaskRunnerImpl>> {
    None
  }
}

// ── TaskRunnerImpl FFI callbacks ──────────────────────────────────────

#[unsafe(no_mangle)]
unsafe extern "C" fn v8__Platform__CustomTaskRunner__PostTask(
  context: *mut std::ffi::c_void,
  task: *mut RawTask,
) {
  let imp = unsafe { &*(context as *const Box<dyn TaskRunnerImpl>) };
  imp.post_task(Task(task));
}

#[unsafe(no_mangle)]
unsafe extern "C" fn v8__Platform__CustomTaskRunner__PostNonNestableTask(
  context: *mut std::ffi::c_void,
  task: *mut RawTask,
) {
  let imp = unsafe { &*(context as *const Box<dyn TaskRunnerImpl>) };
  imp.post_non_nestable_task(Task(task));
}

#[unsafe(no_mangle)]
unsafe extern "C" fn v8__Platform__CustomTaskRunner__PostDelayedTask(
  context: *mut std::ffi::c_void,
  task: *mut RawTask,
  delay_in_seconds: f64,
) {
  let imp = unsafe { &*(context as *const Box<dyn TaskRunnerImpl>) };
  imp.post_delayed_task(Task(task), delay_in_seconds);
}

#[unsafe(no_mangle)]
unsafe extern "C" fn v8__Platform__CustomTaskRunner__PostNonNestableDelayedTask(
  context: *mut std::ffi::c_void,
  task: *mut RawTask,
  delay_in_seconds: f64,
) {
  let imp = unsafe { &*(context as *const Box<dyn TaskRunnerImpl>) };
  imp.post_non_nestable_delayed_task(Task(task), delay_in_seconds);
}

#[unsafe(no_mangle)]
unsafe extern "C" fn v8__Platform__CustomTaskRunner__PostIdleTask(
  context: *mut std::ffi::c_void,
  task: *mut RawIdleTask,
) {
  let imp = unsafe { &*(context as *const Box<dyn TaskRunnerImpl>) };
  imp.post_idle_task(IdleTask(task));
}

#[unsafe(no_mangle)]
unsafe extern "C" fn v8__Platform__CustomTaskRunner__IdleTasksEnabled(
  context: *mut std::ffi::c_void,
) -> bool {
  let imp = unsafe { &*(context as *const Box<dyn TaskRunnerImpl>) };
  imp.idle_tasks_enabled()
}

#[unsafe(no_mangle)]
unsafe extern "C" fn v8__Platform__CustomTaskRunner__NonNestableTasksEnabled(
  context: *mut std::ffi::c_void,
) -> bool {
  let imp = unsafe { &*(context as *const Box<dyn TaskRunnerImpl>) };
  imp.non_nestable_tasks_enabled()
}

#[unsafe(no_mangle)]
unsafe extern "C" fn v8__Platform__CustomTaskRunner__NonNestableDelayedTasksEnabled(
  context: *mut std::ffi::c_void,
) -> bool {
  let imp = unsafe { &*(context as *const Box<dyn TaskRunnerImpl>) };
  imp.non_nestable_delayed_tasks_enabled()
}

#[unsafe(no_mangle)]
unsafe extern "C" fn v8__Platform__CustomTaskRunner__DROP(
  context: *mut std::ffi::c_void,
) {
  unsafe {
    let _ = Box::from_raw(context as *mut Box<dyn TaskRunnerImpl>);
  }
}

// ── PlatformImpl FFI callbacks ───────────────────────────────────────

#[unsafe(no_mangle)]
unsafe extern "C" fn v8__Platform__CustomPlatform__GetForegroundTaskRunner(
  context: *mut std::ffi::c_void,
  isolate: *mut std::ffi::c_void,
) -> *mut std::ffi::c_void {
  let imp = unsafe { &*(context as *const Box<dyn PlatformImpl>) };
  match imp.get_foreground_task_runner(isolate) {
    Some(runner) => {
      // Double-box: Box<dyn TaskRunnerImpl> is a fat pointer, outer Box
      // gives a thin pointer for C++ void*.
      Box::into_raw(Box::new(runner)) as *mut std::ffi::c_void
    }
    None => std::ptr::null_mut(),
  }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn v8__Platform__CustomPlatform__DROP(
  context: *mut std::ffi::c_void,
) {
  unsafe {
    let _ = Box::from_raw(context as *mut Box<dyn PlatformImpl>);
  }
}

// ── Public API ───────────────────────────────────────────────────────

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
/// `GetForegroundTaskRunner` to the provided [`PlatformImpl`] trait object.
///
/// The [`PlatformImpl`] returns [`TaskRunnerImpl`] instances that directly
/// handle V8's foreground task posting (e.g., via tokio).
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
  /// `GetForegroundTaskRunner` to the provided [`PlatformImpl`] trait object.
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
    unsafe {
      UniqueRef::from_raw(v8__Platform__NewCustomPlatform(
        thread_pool_size.min(16) as i32,
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
