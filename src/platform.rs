use crate::support::int;
use crate::Isolate;

use crate::support::long;
use crate::support::Opaque;
use crate::support::Shared;
use crate::support::SharedPtrBase;
use crate::support::SharedRef;
use crate::support::UniquePtr;
use crate::support::UniqueRef;

extern "C" {
  fn v8__Platform__NewDefaultPlatform(
    thread_pool_size: int,
    idle_task_support: bool,
  ) -> *mut Platform;
  fn v8__Platform__NewSingleThreadedDefaultPlatform(
    idle_task_support: bool,
  ) -> *mut Platform;
  fn v8__Platform__DELETE(this: *mut Platform);

  fn v8__Platform__PumpMessageLoop(
    platform: *mut Platform,
    isolate: *mut Isolate,
    wait_for_work: bool,
  ) -> bool;

  fn v8__Platform__RunIdleTasks(
    platform: *mut Platform,
    isolate: *mut Isolate,
    idle_time_in_seconds: f64,
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

/// Returns a new instance of the default v8::Platform implementation.
///
/// |thread_pool_size| is the number of worker threads to allocate for
/// background jobs. If a value of zero is passed, a suitable default
/// based on the current number of processors online will be chosen.
/// If |idle_task_support| is enabled then the platform will accept idle
/// tasks (IdleTasksEnabled will return true) and will rely on the embedder
/// calling v8::platform::RunIdleTasks to process the idle tasks.
pub fn new_default_platform(
  thread_pool_size: u32,
  idle_task_support: bool,
) -> UniqueRef<Platform> {
  Platform::new(thread_pool_size, idle_task_support)
}

/// The same as new_default_platform() but disables the worker thread pool.
/// It must be used with the --single-threaded V8 flag.
///
/// If |idle_task_support| is enabled then the platform will accept idle
/// tasks (IdleTasksEnabled will return true) and will rely on the embedder
/// calling v8::platform::RunIdleTasks to process the idle tasks.
pub fn new_single_threaded_default_platform(
  idle_task_support: bool,
) -> UniqueRef<Platform> {
  Platform::new_single_threaded(idle_task_support)
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
  pub fn new(
    thread_pool_size: u32,
    idle_task_support: bool,
  ) -> UniqueRef<Self> {
    unsafe {
      UniqueRef::from_raw(v8__Platform__NewDefaultPlatform(
        std::cmp::min(thread_pool_size, 16) as i32,
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
  pub fn new_single_threaded(idle_task_support: bool) -> UniqueRef<Self> {
    unsafe {
      UniqueRef::from_raw(v8__Platform__NewSingleThreadedDefaultPlatform(
        idle_task_support,
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
  pub fn pump_message_loop(
    platform: &SharedRef<Self>,
    isolate: &mut Isolate,
    wait_for_work: bool,
  ) -> bool {
    unsafe {
      v8__Platform__PumpMessageLoop(
        &**platform as *const Self as *mut _,
        isolate,
        wait_for_work,
      )
    }
  }

  /// Runs pending idle tasks for at most |idle_time_in_seconds| seconds.
  ///
  /// The caller has to make sure that this is called from the right thread.
  /// This call does not block if no task is pending.
  pub fn run_idle_tasks(
    platform: &SharedRef<Self>,
    isolate: &mut Isolate,
    idle_time_in_seconds: f64,
  ) {
    unsafe {
      v8__Platform__RunIdleTasks(
        &**platform as *const Self as *mut _,
        isolate,
        idle_time_in_seconds,
      )
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
