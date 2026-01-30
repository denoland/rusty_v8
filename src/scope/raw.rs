/// The `raw` module contains prototypes for all the `extern C` functions that
/// are used in the scope module, as well as definitions for the types they operate on.
use crate::{
  Context, Data, Function, Local, Message, OnFailure, Primitive, Value,
  isolate::RealIsolate,
};
use std::num::NonZeroUsize;
use std::{
  mem::MaybeUninit,
  ptr::{self, NonNull},
};

#[derive(Clone, Copy, Debug)]
#[repr(transparent)]
pub(super) struct Address(NonZeroUsize);

#[derive(Debug)]
#[repr(transparent)]
pub(super) struct ContextScope {
  pub(super) entered_context: NonNull<Context>,
}

impl ContextScope {
  pub fn new(context: Local<Context>) -> Self {
    unsafe { v8__Context__Enter(&*context) };
    Self {
      entered_context: context.as_non_null(),
    }
  }
}

impl Drop for ContextScope {
  #[inline(always)]
  fn drop(&mut self) {
    unsafe { v8__Context__Exit(self.entered_context.as_ptr()) };
  }
}

#[cfg(feature = "v8_enable_v8_checks")]
pub const HANDLE_SCOPE_SIZE: usize = 4;
#[cfg(not(feature = "v8_enable_v8_checks"))]
pub const HANDLE_SCOPE_SIZE: usize = 3;

#[repr(C)]
#[derive(Debug)]
pub(super) struct HandleScope([MaybeUninit<usize>; HANDLE_SCOPE_SIZE]);

impl HandleScope {
  /// Creates an uninitialized `HandleScope`.
  ///
  /// This function is marked unsafe because the caller must ensure that the
  /// returned value isn't dropped before `init()` has been called.
  #[inline(always)]
  pub unsafe fn uninit() -> Self {
    unsafe { MaybeUninit::uninit().assume_init() }
  }

  /// This function is marked unsafe because `init()` must be called exactly
  /// once, no more and no less, after creating a `HandleScope` value with
  /// `HandleScope::uninit()`.
  #[inline(always)]
  pub unsafe fn init(&mut self, isolate: NonNull<RealIsolate>) {
    let buf = NonNull::from(self).as_ptr().cast();
    unsafe {
      v8__HandleScope__CONSTRUCT(buf, isolate.as_ptr());
    }
  }
}

impl Drop for HandleScope {
  #[inline(always)]
  fn drop(&mut self) {
    unsafe { v8__HandleScope__DESTRUCT(self) };
  }
}

#[repr(transparent)]
#[derive(Debug)]
pub(super) struct EscapeSlot(NonNull<Address>);

impl EscapeSlot {
  pub fn new(isolate: NonNull<RealIsolate>) -> Self {
    unsafe {
      let undefined = v8__Undefined(isolate.as_ptr()) as *const _;
      let local = v8__Local__New(isolate.as_ptr(), undefined);
      let slot_address_ptr = local as *const Address as *mut _;
      let slot_address_nn = NonNull::new_unchecked(slot_address_ptr);
      Self(slot_address_nn)
    }
  }

  pub fn escape<'e, T>(self, value: Local<'_, T>) -> Local<'e, T>
  where
    for<'l> Local<'l, T>: Into<Local<'l, Data>>,
  {
    const {
      assert!(size_of::<Self>() == size_of::<Local<T>>());
      assert!(align_of::<Self>() == align_of::<Local<T>>());
    }
    unsafe {
      let undefined = Local::<Value>::from_non_null(self.0.cast());
      debug_assert!(undefined.is_undefined());
      let value_address = *(&*value as *const T as *const Address);
      ptr::write(self.0.as_ptr(), value_address);
      Local::from_non_null(self.0.cast())
    }
  }
}

#[repr(C)]
#[derive(Debug)]
pub(super) struct TryCatch([MaybeUninit<usize>; 6]);

impl TryCatch {
  /// Creates an uninitialized `TryCatch`.
  ///
  /// This function is marked unsafe because the caller must ensure that the
  /// returned value isn't dropped before `init()` has been called.
  pub unsafe fn uninit() -> Self {
    // SAFETY: All bit patterns are valid, since the struct is made up of MaybeUninit.
    Self(unsafe { MaybeUninit::uninit().assume_init() })
  }

  /// This function is marked unsafe because `init()` must be called exactly
  /// once, no more and no less, after creating a `TryCatch` value with
  /// `TryCatch::uninit()`.
  pub unsafe fn init(&mut self, isolate: NonNull<RealIsolate>) {
    let buf = NonNull::from(self).cast();
    unsafe {
      v8__TryCatch__CONSTRUCT(buf.as_ptr(), isolate.as_ptr());
    }
  }
}

impl Drop for TryCatch {
  #[inline(always)]
  fn drop(&mut self) {
    unsafe { v8__TryCatch__DESTRUCT(self) };
  }
}

#[repr(C)]
#[derive(Debug)]
pub(super) struct DisallowJavascriptExecutionScope([MaybeUninit<usize>; 2]);

impl DisallowJavascriptExecutionScope {
  /// Creates an uninitialized `DisallowJavascriptExecutionScope`.
  ///
  /// This function is marked unsafe because the caller must ensure that the
  /// returned value isn't dropped before `init()` has been called.
  #[inline]
  pub unsafe fn uninit() -> Self {
    // SAFETY: All bit patterns are valid, since the struct is made up of MaybeUninit.
    Self(unsafe { MaybeUninit::uninit().assume_init() })
  }

  /// This function is marked unsafe because `init()` must be called exactly
  /// once, no more and no less, after creating a
  /// `DisallowJavascriptExecutionScope` value with
  /// `DisallowJavascriptExecutionScope::uninit()`.
  #[inline]
  pub unsafe fn init(
    &mut self,
    isolate: NonNull<RealIsolate>,
    on_failure: OnFailure,
  ) {
    let buf = NonNull::from(self).cast();
    unsafe {
      v8__DisallowJavascriptExecutionScope__CONSTRUCT(
        buf.as_ptr(),
        isolate.as_ptr(),
        on_failure,
      );
    }
  }
}

impl Drop for DisallowJavascriptExecutionScope {
  #[inline(always)]
  fn drop(&mut self) {
    unsafe { v8__DisallowJavascriptExecutionScope__DESTRUCT(self) };
  }
}

#[repr(C)]
#[derive(Debug)]
pub(super) struct AllowJavascriptExecutionScope([MaybeUninit<usize>; 2]);

impl AllowJavascriptExecutionScope {
  /// Creates an uninitialized `AllowJavascriptExecutionScope`.
  ///
  /// This function is marked unsafe because the caller must ensure that the
  /// returned value isn't dropped before `init()` has been called.
  #[inline]
  pub unsafe fn uninit() -> Self {
    Self(unsafe { MaybeUninit::uninit().assume_init() })
  }

  /// This function is marked unsafe because `init()` must be called exactly
  /// once, no more and no less, after creating an
  /// `AllowJavascriptExecutionScope` value with
  /// `AllowJavascriptExecutionScope::uninit()`.
  #[inline]
  pub unsafe fn init(&mut self, isolate: NonNull<RealIsolate>) {
    unsafe {
      let buf = NonNull::from(self).cast();
      v8__AllowJavascriptExecutionScope__CONSTRUCT(
        buf.as_ptr(),
        isolate.as_ptr(),
      );
    }
  }
}

impl Drop for AllowJavascriptExecutionScope {
  #[inline(always)]
  fn drop(&mut self) {
    unsafe { v8__AllowJavascriptExecutionScope__DESTRUCT(self) };
  }
}

/// Raw V8 Locker binding.
///
/// This is a low-level wrapper around `v8::Locker`. It must be used with
/// proper two-phase initialization: first call `uninit()`, then `init()`.
///
/// # Memory Layout
///
/// This struct is `#[repr(C)]` and sized to match `v8::Locker` exactly
/// (verified by the `locker_size_matches_v8` test). The size is 2 * sizeof(usize)
/// which equals 16 bytes on 64-bit platforms.
///
/// # Safety Invariants
///
/// 1. **Initialization**: After calling `uninit()`, you MUST call `init()` before
///    the `Locker` is dropped. Dropping an uninitialized `Locker` is undefined
///    behavior because `Drop` will call the C++ destructor on garbage data.
///
/// 2. **Isolate State**: The isolate passed to `init()` must be in "entered" state
///    (via `v8::Isolate::Enter()`) before calling `init()`.
///
/// 3. **Single Initialization**: `init()` must be called exactly once. Calling it
///    multiple times is undefined behavior.
///
/// 4. **Thread Affinity**: Once initialized, the `Locker` must be used and dropped
///    on the same thread where it was created.
#[repr(C)]
#[derive(Debug)]
pub(crate) struct Locker([MaybeUninit<usize>; 2]);

#[test]
fn locker_size_matches_v8() {
  assert_eq!(
    std::mem::size_of::<Locker>(),
    unsafe { v8__Locker__SIZE() },
    "Locker size mismatch"
  );
}

impl Locker {
  /// Creates an uninitialized `Locker`.
  ///
  /// # Safety
  ///
  /// The returned `Locker` is in an invalid state. You MUST call `init()` before:
  /// - Using the `Locker` in any way
  /// - Dropping the `Locker` (including via panic unwinding)
  ///
  /// Failure to initialize before drop will cause undefined behavior because
  /// `Drop::drop` will call the C++ destructor on uninitialized memory.
  #[inline]
  pub unsafe fn uninit() -> Self {
    Self(unsafe { MaybeUninit::uninit().assume_init() })
  }

  /// Initializes the `Locker` for the given isolate.
  ///
  /// # Safety
  ///
  /// - This must be called exactly once after `uninit()`
  /// - The isolate must be valid and in "entered" state
  /// - The isolate must not be locked by another `Locker`
  /// - After this call, the `Locker` owns the V8 lock until dropped
  #[inline]
  pub unsafe fn init(&mut self, isolate: NonNull<RealIsolate>) {
    let buf = NonNull::from(self).cast();
    unsafe { v8__Locker__CONSTRUCT(buf.as_ptr(), isolate.as_ptr()) };
  }

  /// Returns `true` if the given isolate is currently locked by any `Locker`.
  ///
  /// This is safe to call from any thread.
  pub fn is_locked(isolate: NonNull<RealIsolate>) -> bool {
    unsafe { v8__Locker__IsLocked(isolate.as_ptr()) }
  }
}

impl Drop for Locker {
  /// Releases the V8 lock.
  ///
  /// # Safety (internal)
  ///
  /// This assumes the `Locker` was properly initialized via `init()`.
  /// Dropping an uninitialized `Locker` is undefined behavior.
  #[inline(always)]
  fn drop(&mut self) {
    unsafe { v8__Locker__DESTRUCT(self) };
  }
}

unsafe extern "C" {
  pub(super) fn v8__Isolate__GetCurrent() -> *mut RealIsolate;
  pub(super) fn v8__Isolate__GetCurrentContext(
    isolate: *mut RealIsolate,
  ) -> *const Context;
  pub(super) fn v8__Isolate__GetEnteredOrMicrotaskContext(
    isolate: *mut RealIsolate,
  ) -> *const Context;
  pub(super) fn v8__Isolate__ThrowException(
    isolate: *mut RealIsolate,
    exception: *const Value,
  ) -> *const Value;
  pub(super) fn v8__Isolate__GetDataFromSnapshotOnce(
    this: *mut RealIsolate,
    index: usize,
  ) -> *const Data;
  pub(super) fn v8__Isolate__GetCurrentHostDefinedOptions(
    this: *mut RealIsolate,
  ) -> *const Data;

  pub(super) fn v8__Context__Enter(this: *const Context);
  pub(super) fn v8__Context__Exit(this: *const Context);
  pub(super) fn v8__Context__GetDataFromSnapshotOnce(
    this: *const Context,
    index: usize,
  ) -> *const Data;
  pub(super) fn v8__Context__SetPromiseHooks(
    this: *const Context,
    init_hook: *const Function,
    before_hook: *const Function,
    after_hook: *const Function,
    resolve_hook: *const Function,
  );
  pub(super) fn v8__Context__SetContinuationPreservedEmbedderData(
    this: *mut RealIsolate,
    value: *const Value,
  );
  pub(super) fn v8__Context__GetContinuationPreservedEmbedderData(
    this: *mut RealIsolate,
  ) -> *const Value;

  pub(super) fn v8__HandleScope__CONSTRUCT(
    buf: *mut MaybeUninit<HandleScope>,
    isolate: *mut RealIsolate,
  );
  pub(super) fn v8__HandleScope__DESTRUCT(this: *mut HandleScope);

  pub(super) fn v8__Local__New(
    isolate: *mut RealIsolate,
    other: *const Data,
  ) -> *const Data;
  pub(super) fn v8__Undefined(isolate: *mut RealIsolate) -> *const Primitive;

  pub(super) fn v8__TryCatch__CONSTRUCT(
    buf: *mut MaybeUninit<TryCatch>,
    isolate: *mut RealIsolate,
  );
  pub(super) fn v8__TryCatch__DESTRUCT(this: *mut TryCatch);
  pub(super) fn v8__TryCatch__HasCaught(this: *const TryCatch) -> bool;
  pub(super) fn v8__TryCatch__CanContinue(this: *const TryCatch) -> bool;
  pub(super) fn v8__TryCatch__HasTerminated(this: *const TryCatch) -> bool;
  pub(super) fn v8__TryCatch__IsVerbose(this: *const TryCatch) -> bool;
  pub(super) fn v8__TryCatch__SetVerbose(this: *mut TryCatch, value: bool);
  pub(super) fn v8__TryCatch__SetCaptureMessage(
    this: *mut TryCatch,
    value: bool,
  );
  pub(super) fn v8__TryCatch__Reset(this: *mut TryCatch);
  pub(super) fn v8__TryCatch__Exception(this: *const TryCatch) -> *const Value;
  pub(super) fn v8__TryCatch__StackTrace(
    this: *const TryCatch,
    context: *const Context,
  ) -> *const Value;
  pub(super) fn v8__TryCatch__Message(this: *const TryCatch) -> *const Message;
  pub(super) fn v8__TryCatch__ReThrow(this: *mut TryCatch) -> *const Value;

  pub(super) fn v8__DisallowJavascriptExecutionScope__CONSTRUCT(
    buf: *mut MaybeUninit<DisallowJavascriptExecutionScope>,
    isolate: *mut RealIsolate,
    on_failure: OnFailure,
  );
  pub(super) fn v8__DisallowJavascriptExecutionScope__DESTRUCT(
    this: *mut DisallowJavascriptExecutionScope,
  );

  pub(super) fn v8__AllowJavascriptExecutionScope__CONSTRUCT(
    buf: *mut MaybeUninit<AllowJavascriptExecutionScope>,
    isolate: *mut RealIsolate,
  );
  pub(super) fn v8__AllowJavascriptExecutionScope__DESTRUCT(
    this: *mut AllowJavascriptExecutionScope,
  );

  pub(super) fn v8__Locker__CONSTRUCT(
    buf: *mut MaybeUninit<Locker>,
    isolate: *mut RealIsolate,
  );
  pub(super) fn v8__Locker__DESTRUCT(this: *mut Locker);
  pub(super) fn v8__Locker__IsLocked(isolate: *mut RealIsolate) -> bool;

  #[cfg(test)]
  fn v8__Locker__SIZE() -> usize;
}
