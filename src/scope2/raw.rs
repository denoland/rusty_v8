/// The `raw` module contains prototypes for all the `extern C` functions that
/// are used in this file, as well as definitions for the types they operate on.
use crate::{
  Context, Data, Function, Isolate, Local, Message, Object, OnFailure,
  Primitive, Value,
};
use std::alloc::Layout;
use std::num::NonZeroUsize;
use std::{
  mem::MaybeUninit,
  ptr::{self, NonNull},
};

#[derive(Clone, Copy, Debug)]
#[repr(transparent)]
pub(super) struct Address(NonZeroUsize);

#[derive(Debug)]
pub(super) struct ContextScope {
  entered_context: NonNull<Context>,
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

#[repr(C)]
#[derive(Debug)]
pub(super) struct HandleScope([MaybeUninit<usize>; 3]);

impl HandleScope {
  /// Creates an uninitialized `HandleScope`.
  ///
  /// This function is marked unsafe because the caller must ensure that the
  /// returned value isn't dropped before `init()` has been called.
  pub unsafe fn uninit() -> Self {
    Self([MaybeUninit::uninit(); 3])
  }

  /// This function is marked unsafe because `init()` must be called exactly
  /// once, no more and no less, after creating a `HandleScope` value with
  /// `HandleScope::uninit()`.
  pub unsafe fn init(&mut self, isolate: NonNull<Isolate>) {
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
  pub fn new(isolate: NonNull<Isolate>) -> Self {
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
    assert_eq!(Layout::new::<Self>(), Layout::new::<Local<T>>());
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
    Self(unsafe { MaybeUninit::uninit().assume_init() })
  }

  /// This function is marked unsafe because `init()` must be called exactly
  /// once, no more and no less, after creating a `TryCatch` value with
  /// `TryCatch::uninit()`.
  pub unsafe fn init(&mut self, isolate: NonNull<Isolate>) {
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
    Self(unsafe { MaybeUninit::uninit().assume_init() })
  }

  /// This function is marked unsafe because `init()` must be called exactly
  /// once, no more and no less, after creating a
  /// `DisallowJavascriptExecutionScope` value with
  /// `DisallowJavascriptExecutionScope::uninit()`.
  #[inline]
  pub unsafe fn init(
    &mut self,
    isolate: NonNull<Isolate>,
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
  pub unsafe fn init(&mut self, isolate: NonNull<Isolate>) {
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

unsafe extern "C" {
  pub(super) fn v8__Isolate__GetCurrentContext(
    isolate: *mut Isolate,
  ) -> *const Context;
  pub(super) fn v8__Isolate__GetEnteredOrMicrotaskContext(
    isolate: *mut Isolate,
  ) -> *const Context;
  pub(super) fn v8__Isolate__ThrowException(
    isolate: *mut Isolate,
    exception: *const Value,
  ) -> *const Value;
  pub(super) fn v8__Isolate__GetDataFromSnapshotOnce(
    this: *mut Isolate,
    index: usize,
  ) -> *const Data;
  pub(super) fn v8__Isolate__GetCurrentHostDefinedOptions(
    this: *mut Isolate,
  ) -> *const Data;

  pub(super) fn v8__Context__EQ(
    this: *const Context,
    other: *const Context,
  ) -> bool;
  pub(super) fn v8__Context__Enter(this: *const Context);
  pub(super) fn v8__Context__Exit(this: *const Context);
  pub(super) fn v8__Context__GetIsolate(this: *const Context) -> *mut Isolate;
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
    this: *mut Isolate,
    value: *const Value,
  );
  pub(super) fn v8__Context__GetContinuationPreservedEmbedderData(
    this: *mut Isolate,
  ) -> *const Value;

  pub(super) fn v8__HandleScope__CONSTRUCT(
    buf: *mut MaybeUninit<HandleScope>,
    isolate: *mut Isolate,
  );
  pub(super) fn v8__HandleScope__DESTRUCT(this: *mut HandleScope);

  pub(super) fn v8__Local__New(
    isolate: *mut Isolate,
    other: *const Data,
  ) -> *const Data;
  pub(super) fn v8__Undefined(isolate: *mut Isolate) -> *const Primitive;

  pub(super) fn v8__TryCatch__CONSTRUCT(
    buf: *mut MaybeUninit<TryCatch>,
    isolate: *mut Isolate,
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
    isolate: *mut Isolate,
    on_failure: OnFailure,
  );
  pub(super) fn v8__DisallowJavascriptExecutionScope__DESTRUCT(
    this: *mut DisallowJavascriptExecutionScope,
  );

  pub(super) fn v8__AllowJavascriptExecutionScope__CONSTRUCT(
    buf: *mut MaybeUninit<AllowJavascriptExecutionScope>,
    isolate: *mut Isolate,
  );
  pub(super) fn v8__AllowJavascriptExecutionScope__DESTRUCT(
    this: *mut AllowJavascriptExecutionScope,
  );

  pub(super) fn v8__Message__GetIsolate(this: *const Message) -> *mut Isolate;
  pub(super) fn v8__Object__GetIsolate(this: *const Object) -> *mut Isolate;
}
