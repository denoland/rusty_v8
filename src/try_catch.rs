use std::marker::PhantomData;
use std::mem::size_of;
use std::mem::size_of_val;
use std::mem::take;
use std::mem::MaybeUninit;

use crate::Context;
use crate::HandleScope;
use crate::Isolate;
use crate::Local;
use crate::Message;
use crate::Value;

extern "C" {
  // Note: the C++ CxxTryCatch object *must* live on the stack, and it must
  // not move after it is constructed.
  fn v8__TryCatch__CONSTRUCT(
    buf: &mut MaybeUninit<CxxTryCatch>,
    isolate: *mut Isolate,
  );

  fn v8__TryCatch__DESTRUCT(this: &mut CxxTryCatch);

  fn v8__TryCatch__HasCaught(this: &CxxTryCatch) -> bool;

  fn v8__TryCatch__CanContinue(this: &CxxTryCatch) -> bool;

  fn v8__TryCatch__HasTerminated(this: &CxxTryCatch) -> bool;

  fn v8__TryCatch__Exception(this: &CxxTryCatch) -> *mut Value;

  fn v8__TryCatch__StackTrace(
    this: &CxxTryCatch,
    context: Local<Context>,
  ) -> *mut Value;

  fn v8__TryCatch__Message(this: &CxxTryCatch) -> *mut Message;

  fn v8__TryCatch__Reset(this: &mut CxxTryCatch);

  fn v8__TryCatch__ReThrow(this: &mut CxxTryCatch) -> *mut Value;

  fn v8__TryCatch__IsVerbose(this: &CxxTryCatch) -> bool;

  fn v8__TryCatch__SetVerbose(this: &mut CxxTryCatch, value: bool);

  fn v8__TryCatch__SetCaptureMessage(this: &mut CxxTryCatch, value: bool);
}

// Note: the 'tc lifetime is there to ensure that after entering a TryCatchScope
// once, the same TryCatch object can't be entered again.

/// An external exception handler.
pub struct TryCatch<'tc>(TryCatchState<'tc>);

#[repr(transparent)]
pub struct TryCatchScope<'tc>(CxxTryCatch, PhantomData<&'tc ()>);

#[repr(C)]
struct CxxTryCatch([usize; 6]);

enum TryCatchState<'tc> {
  New { isolate: *mut Isolate },
  Uninit(MaybeUninit<TryCatchScope<'tc>>),
  Constructed(TryCatchScope<'tc>),
}

impl<'tc> TryCatch<'tc> {
  /// Creates a new try/catch block and registers it with v8. Note that
  /// all TryCatch blocks should be stack allocated because the memory
  /// location itself is compared against JavaScript try/catch blocks.
  pub fn new(scope: &mut impl AsMut<Isolate>) -> Self {
    Self(TryCatchState::New {
      isolate: scope.as_mut(),
    })
  }

  pub fn enter(&'tc mut self) -> &'tc mut TryCatchScope {
    use TryCatchState::*;
    let state = &mut self.0;

    let isolate = match take(state) {
      New { isolate } => isolate,
      _ => unreachable!(),
    };

    let buf = match state {
      Uninit(b) => b,
      _ => unreachable!(),
    };

    TryCatchScope::construct(buf, isolate);

    *state = match take(state) {
      Uninit(b) => Constructed(unsafe { b.assume_init() }),
      _ => unreachable!(),
    };

    match state {
      Constructed(v) => v,
      _ => unreachable!(),
    }
  }
}

impl<'tc> TryCatchScope<'tc> {
  /// Returns true if an exception has been caught by this try/catch block.
  pub fn has_caught(&self) -> bool {
    unsafe { v8__TryCatch__HasCaught(&self.0) }
  }

  /// For certain types of exceptions, it makes no sense to continue execution.
  ///
  /// If CanContinue returns false, the correct action is to perform any C++
  /// cleanup needed and then return. If CanContinue returns false and
  /// HasTerminated returns true, it is possible to call
  /// CancelTerminateExecution in order to continue calling into the engine.
  pub fn can_continue(&self) -> bool {
    unsafe { v8__TryCatch__CanContinue(&self.0) }
  }

  /// Returns true if an exception has been caught due to script execution
  /// being terminated.
  ///
  /// There is no JavaScript representation of an execution termination
  /// exception. Such exceptions are thrown when the TerminateExecution
  /// methods are called to terminate a long-running script.
  ///
  /// If such an exception has been thrown, HasTerminated will return true,
  /// indicating that it is possible to call CancelTerminateExecution in order
  /// to continue calling into the engine.
  pub fn has_terminated(&self) -> bool {
    unsafe { v8__TryCatch__HasTerminated(&self.0) }
  }

  /// Returns the exception caught by this try/catch block. If no exception has
  /// been caught an empty handle is returned.
  ///
  /// The returned handle is valid until this TryCatch block has been destroyed.
  pub fn exception(&self) -> Option<Local<'tc, Value>> {
    unsafe { Local::from_raw(v8__TryCatch__Exception(&self.0)) }
  }

  /// Returns the .stack property of the thrown object. If no .stack
  /// property is present an empty handle is returned.
  pub fn stack_trace<'sc>(
    &self,
    _scope: &mut impl AsMut<HandleScope<'sc>>,
    context: Local<Context>,
  ) -> Option<Local<'sc, Value>> {
    unsafe { Local::from_raw(v8__TryCatch__StackTrace(&self.0, context)) }
  }

  /// Returns the message associated with this exception. If there is
  /// no message associated an empty handle is returned.
  ///
  /// The returned handle is valid until this TryCatch block has been
  /// destroyed.
  pub fn message(&self) -> Option<Local<'tc, Message>>
  where
    Self: 'tc,
  {
    unsafe { Local::from_raw(v8__TryCatch__Message(&self.0)) }
  }

  /// Clears any exceptions that may have been caught by this try/catch block.
  /// After this method has been called, HasCaught() will return false. Cancels
  /// the scheduled exception if it is caught and ReThrow() is not called before.
  ///
  /// It is not necessary to clear a try/catch block before using it again; if
  /// another exception is thrown the previously caught exception will just be
  /// overwritten. However, it is often a good idea since it makes it easier
  /// to determine which operation threw a given exception.
  pub fn reset(&mut self) {
    unsafe { v8__TryCatch__Reset(&mut self.0) };
  }

  /// Throws the exception caught by this TryCatch in a way that avoids
  /// it being caught again by this same TryCatch. As with ThrowException
  /// it is illegal to execute any JavaScript operations after calling
  /// ReThrow; the caller must return immediately to where the exception
  /// is caught.
  pub fn rethrow<'a>(&'_ mut self) -> Option<Local<'a, Value>> {
    unsafe { Local::from_raw(v8__TryCatch__ReThrow(&mut self.0)) }
  }

  /// Returns true if verbosity is enabled.
  pub fn is_verbose(&self) -> bool {
    unsafe { v8__TryCatch__IsVerbose(&self.0) }
  }

  /// Set verbosity of the external exception handler.
  ///
  /// By default, exceptions that are caught by an external exception
  /// handler are not reported. Call SetVerbose with true on an
  /// external exception handler to have exceptions caught by the
  /// handler reported as if they were not caught.
  pub fn set_verbose(&mut self, value: bool) {
    unsafe { v8__TryCatch__SetVerbose(&mut self.0, value) };
  }

  /// Set whether or not this TryCatch should capture a Message object
  /// which holds source information about where the exception
  /// occurred. True by default.
  pub fn set_capture_message(&mut self, value: bool) {
    unsafe { v8__TryCatch__SetCaptureMessage(&mut self.0, value) };
  }

  fn construct(buf: &mut MaybeUninit<TryCatchScope>, isolate: *mut Isolate) {
    unsafe {
      assert_eq!(size_of_val(buf), size_of::<CxxTryCatch>());
      let buf = &mut *(buf as *mut _ as *mut MaybeUninit<CxxTryCatch>);
      v8__TryCatch__CONSTRUCT(buf, isolate);
    }
  }
}

impl Drop for CxxTryCatch {
  fn drop(&mut self) {
    unsafe { v8__TryCatch__DESTRUCT(self) }
  }
}

impl<'tc> Default for TryCatchState<'tc> {
  fn default() -> Self {
    Self::Uninit(MaybeUninit::uninit())
  }
}
