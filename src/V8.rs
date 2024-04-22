// Copyright 2019-2021 the Deno authors. All rights reserved. MIT license.
use once_cell::sync::Lazy;
use std::ffi::CStr;
use std::ffi::CString;
use std::sync::Mutex;
use std::sync::Once;
use std::vec::Vec;

use crate::support::char;
use crate::support::int;
use crate::support::SharedRef;
use crate::support::UnitType;
use crate::CreateParams;
use crate::Isolate;
use crate::OwnedIsolate;
use crate::Platform;

extern "C" {
  fn v8__V8__SetFlagsFromCommandLine(
    argc: *mut int,
    argv: *mut *mut char,
    usage: *const char,
  );
  fn v8__V8__SetFlagsFromString(flags: *const u8, length: usize);
  fn v8__V8__SetEntropySource(callback: EntropySource);
  fn v8__V8__GetVersion() -> *const char;
  fn v8__V8__InitializePlatform(platform: *mut Platform);
  fn v8__V8__Initialize();
  fn v8__V8__Dispose() -> bool;
  fn v8__V8__DisposePlatform();
}

/// EntropySource is used as a callback function when v8 needs a source
/// of entropy.
#[derive(Copy, Clone)]
#[repr(transparent)]
pub struct EntropySource(RawEntropySource);

pub trait IntoEntropySource:
  UnitType + Into<EntropySource> + FnOnce(&mut [u8]) -> bool
{
}

impl<F> IntoEntropySource for F where
  F: UnitType + Into<EntropySource> + FnOnce(&mut [u8]) -> bool
{
}

type RawEntropySource = extern "C" fn(*mut u8, usize) -> bool;

impl<F> From<F> for EntropySource
where
  F: UnitType + FnOnce(&mut [u8]) -> bool,
{
  fn from(_: F) -> Self {
    #[inline(always)]
    extern "C" fn adapter<F: IntoEntropySource>(
      buffer: *mut u8,
      length: usize,
    ) -> bool {
      let buffer = unsafe { std::slice::from_raw_parts_mut(buffer, length) };
      (F::get())(buffer)
    }

    Self(adapter::<F>)
  }
}

#[derive(Debug)]
enum GlobalState {
  Uninitialized,
  PlatformInitialized(SharedRef<Platform>),
  Initialized(SharedRef<Platform>),
  Disposed(#[allow(dead_code)] SharedRef<Platform>),
  PlatformShutdown,
}
use GlobalState::*;

static GLOBAL_STATE: Lazy<Mutex<GlobalState>> =
  Lazy::new(|| Mutex::new(Uninitialized));

pub fn assert_initialized() {
  let global_state_guard = GLOBAL_STATE.lock().unwrap();
  match *global_state_guard {
    Initialized(_) => {}
    _ => panic!("Invalid global state"),
  };
}

/// Pass the command line arguments to v8.
/// The first element of args (which usually corresponds to the binary name) is
/// ignored.
/// Returns a vector of command line arguments that V8 did not understand.
/// TODO: Check whether this is safe to do after globally initializing v8.
pub fn set_flags_from_command_line(args: Vec<String>) -> Vec<String> {
  set_flags_from_command_line_with_usage(args, None)
}

/// The example code below is here to avoid the V8 usage string and options
/// that are printed to stdout by this function. Placing this here instead of
/// in a test allows the output to be suppressed.
///
/// # Examples
///
/// ```
///     let r = v8::V8::set_flags_from_command_line_with_usage(
///       vec!["binaryname".to_string(), "--help".to_string()],
///       Some("Usage: binaryname --startup-src=file\n\n"),
///     );
///     assert_eq!(r, vec!["binaryname".to_string()]);
/// ```
pub fn set_flags_from_command_line_with_usage(
  args: Vec<String>,
  usage: Option<&str>,
) -> Vec<String> {
  // deno_set_v8_flags(int* argc, char** argv) mutates argc and argv to remove
  // flags that v8 understands.

  // Make a new array, that can be modified by V8::SetFlagsFromCommandLine(),
  // containing mutable raw pointers to the individual command line args.
  let mut raw_argv = args
    .iter()
    .map(|arg| CString::new(arg.as_str()).unwrap().into_bytes_with_nul())
    .collect::<Vec<_>>();
  let mut c_argv = raw_argv
    .iter_mut()
    .map(|arg| arg.as_mut_ptr() as *mut char)
    .collect::<Vec<_>>();

  // Store the length of the c_argv array in a local variable. We'll pass
  // a pointer to this local variable to deno_set_v8_flags(), which then
  // updates its value.
  let mut c_argv_len = c_argv.len() as int;
  // Let v8 parse the arguments it recognizes and remove them from c_argv.
  let c_usage = match usage {
    Some(str) => CString::new(str).unwrap().into_raw() as *const char,
    None => std::ptr::null(),
  };
  unsafe {
    v8__V8__SetFlagsFromCommandLine(
      &mut c_argv_len,
      c_argv.as_mut_ptr(),
      c_usage,
    );
  };
  // If c_argv_len was updated we have to change the length of c_argv to match.
  c_argv.truncate(c_argv_len as usize);
  // Copy the modified arguments list into a proper rust vec and return it.
  c_argv
    .iter()
    .map(|ptr| unsafe {
      let cstr = CStr::from_ptr(*ptr as *const char);
      let slice = cstr.to_str().unwrap();
      slice.to_string()
    })
    .collect()
}

/// Sets V8 flags from a string.
pub fn set_flags_from_string(flags: &str) {
  unsafe {
    v8__V8__SetFlagsFromString(flags.as_ptr(), flags.len());
  }
}

/// Allows the host application to provide a callback which can be used
/// as a source of entropy for random number generators.
pub fn set_entropy_source(
  callback: impl UnitType + Into<EntropySource> + FnOnce(&mut [u8]) -> bool,
) {
  unsafe { v8__V8__SetEntropySource(callback.into()) };
}

/// Get the version string.
pub fn get_version() -> &'static str {
  let version = unsafe { v8__V8__GetVersion() };
  let c_str = unsafe { CStr::from_ptr(version) };
  c_str.to_str().unwrap()
}

/// Sets the v8::Platform to use. This should be invoked before V8 is
/// initialized.
pub fn initialize_platform(platform: SharedRef<Platform>) {
  let mut global_state_guard = GLOBAL_STATE.lock().unwrap();
  *global_state_guard = match *global_state_guard {
    Uninitialized => PlatformInitialized(platform.clone()),
    _ => panic!("Invalid global state"),
  };

  {
    unsafe {
      v8__V8__InitializePlatform(&*platform as *const Platform as *mut _)
    };
  }
}

/// Initializes V8. This function needs to be called before the first Isolate
/// is created. It always returns true.
pub fn initialize() {
  let mut global_state_guard = GLOBAL_STATE.lock().unwrap();
  *global_state_guard = match *global_state_guard {
    PlatformInitialized(ref platform) => Initialized(platform.clone()),
    _ => panic!("Invalid global state"),
  };
  unsafe { v8__V8__Initialize() }
}

/// Sets the v8::Platform to use. This should be invoked before V8 is
/// initialized.
pub fn get_current_platform() -> SharedRef<Platform> {
  let global_state_guard = GLOBAL_STATE.lock().unwrap();
  match *global_state_guard {
    Initialized(ref platform) => platform.clone(),
    _ => panic!("Invalid global state"),
  }
}

/// Releases any resources used by v8 and stops any utility threads
/// that may be running.  Note that disposing v8 is permanent, it
/// cannot be reinitialized.
///
/// It should generally not be necessary to dispose v8 before exiting
/// a process, this should happen automatically.  It is only necessary
/// to use if the process needs the resources taken up by v8.
///
/// # Safety
///
/// Calling this function before completely disposing all isolates will lead
/// to a crash.
pub unsafe fn dispose() -> bool {
  let mut global_state_guard = GLOBAL_STATE.lock().unwrap();
  *global_state_guard = match *global_state_guard {
    Initialized(ref platform) => Disposed(platform.clone()),
    _ => panic!("Invalid global state"),
  };
  assert!(v8__V8__Dispose());
  true
}

/// Clears all references to the v8::Platform. This should be invoked after
/// V8 was disposed. If it is called if V8 is not disposed, it will panic.
pub fn dispose_platform() {
  let mut global_state_guard = GLOBAL_STATE.lock().unwrap();
  // First check that the global state is disposed. If it is we dispose the
  // platform. We then drop the platform.
  *global_state_guard = match *global_state_guard {
    Disposed(_) => {
      unsafe { v8__V8__DisposePlatform() };
      PlatformShutdown
    }
    _ => panic!("Invalid global state"),
  };
}

/// You can use this to initialize v8 isolates
#[derive(Debug, Clone, Copy)]
pub struct PlatformToken(());

#[derive(Debug, Clone, Copy, Default)]
pub struct V8Builder {
  /// If None is passed it will disable the worker pool.
  thread_pool_size: Option<u32>,

  idle_task_support: bool,
  unprotected: bool,
}

impl V8Builder {
  pub fn new() -> Self {
    Self {
      thread_pool_size: Some(0),
      idle_task_support: true,
      unprotected: false,
    }
  }

  /// If |idle_task_support| is enabled then the platform will accept idle
  /// tasks (IdleTasksEnabled will return true) and will rely on the embedder
  /// calling v8::platform::RunIdleTasks to process the idle tasks.
  pub fn no_idle_task(mut self) -> Self {
    self.idle_task_support = false;
    self
  }

  /// |thread_pool_size| is the number of worker threads to allocate for
  /// background jobs. If a value of zero is passed, a suitable default
  /// based on the current number of processors online will be chosen.
  pub fn thread_pool_size(mut self, size: u32) -> Self {
    self.thread_pool_size = Some(size);
    self
  }

  /// Disables the worker thread pool.
  pub fn disable_thread_pool(mut self) -> Self {
    self.thread_pool_size = None;
    self
  }

  /// Creates a platform that is identical to the default platform, but does not
  /// enforce thread-isolated allocations. This may reduce security in some cases,
  /// so this method should be used with caution in cases where the threading
  /// guarantees of `new_default_platform` cannot be upheld (generally for tests).
  pub fn unprotected(mut self) -> Self {
    self.unprotected = true;
    self
  }

  /// Will return a [PlatformToken], used to construct isolates.
  /// If this has been called before it will return None.
  pub fn build(self) -> Option<PlatformToken> {
    static ONCE: Once = Once::new();
    let mut pf = None;

    ONCE.call_once(|| {
      let platform = if let Some(thread_pool_size) = self.thread_pool_size {
        if self.unprotected {
          Platform::new_unprotected(thread_pool_size, self.idle_task_support)
        } else {
          Platform::new(thread_pool_size, self.idle_task_support)
        }
      } else {
        if self.unprotected {
          panic!("Can't make single threaded unprotected platform")
        }
        Platform::new_single_threaded(self.idle_task_support)
      };

      initialize_platform(platform.into());
      initialize();
      pf = Some(PlatformToken(()))
    });
    pf
  }
}

impl PlatformToken {
  pub fn isolate(token: &Self) -> OwnedIsolate {
    Self::isolate_with_params(token, Default::default())
  }

  pub fn isolate_with_params(
    _: &PlatformToken,
    params: CreateParams,
  ) -> OwnedIsolate {
    static HAS_EXECUTED: Mutex<bool> = Mutex::new(false);
    let mut g = HAS_EXECUTED.lock().unwrap();
    if !(*g) {
      *g = true;
      Isolate::new(params)
    } else {
      drop(g);
      Isolate::new(params)
    }
  }
}
