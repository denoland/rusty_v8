use std::mem::MaybeUninit;

use crate::Context;
use crate::Isolate;
use crate::Local;
use crate::Message;
use crate::Scope;
use crate::Value;

extern "C" {
  // Note: the C++ TryCatch object *must* live on the stack, and it must
  // not move after it is constructed.
  fn v8__TryCatch__CONSTRUCT(
    buf: &mut MaybeUninit<TryCatch>,
    isolate: *mut Isolate,
  );

  fn v8__TryCatch__DESTRUCT(this: &mut TryCatch);

  fn v8__TryCatch__HasCaught(this: &TryCatch) -> bool;

  fn v8__TryCatch__CanContinue(this: &TryCatch) -> bool;

  fn v8__TryCatch__HasTerminated(this: &TryCatch) -> bool;

  fn v8__TryCatch__Exception(this: &TryCatch) -> *mut Value;

  fn v8__TryCatch__StackTrace(
    this: &TryCatch,
    context: Local<Context>,
  ) -> *mut Value;

  fn v8__TryCatch__Message(this: &TryCatch) -> *mut Message;

  fn v8__TryCatch__Reset(this: &mut TryCatch);

  fn v8__TryCatch__ReThrow(this: &mut TryCatch) -> *mut Value;

  fn v8__TryCatch__IsVerbose(this: &TryCatch) -> bool;

  fn v8__TryCatch__SetVerbose(this: &mut TryCatch, value: bool);

  fn v8__TryCatch__SetCaptureMessage(this: &mut TryCatch, value: bool);
}

/// An external exception handler.
#[repr(C)]
pub struct TryCatch([usize; 6]);

impl TryCatch {
  /// Creates a new try/catch block. Note that all TryCatch blocks should be
  /// stack allocated because the memory location itself is compared against
  /// JavaScript try/catch blocks.
  #[allow(clippy::new_ret_no_self)]
  pub fn new<'s, F>(scope: &mut Scope, f: F)
  where
    F: FnOnce(&mut Scope, &mut TryCatch),
  {
    let mut tc: MaybeUninit<TryCatch> = MaybeUninit::uninit();
    assert_eq!(std::mem::size_of_val(&tc), std::mem::size_of::<TryCatch>());
    let isolate = scope.isolate();
    unsafe { v8__TryCatch__CONSTRUCT(&mut tc, isolate) };
    let mut tc = unsafe { tc.assume_init() };

    f(scope, &mut tc);

    unsafe {
      v8__TryCatch__DESTRUCT(&mut tc);
    }
    // drop(tc);
  }

  /// Returns true if an exception has been caught by this try/catch block.
  pub fn has_caught(&self) -> bool {
    unsafe { v8__TryCatch__HasCaught(self) }
  }

  /// For certain types of exceptions, it makes no sense to continue execution.
  ///
  /// If CanContinue returns false, the correct action is to perform any C++
  /// cleanup needed and then return. If CanContinue returns false and
  /// HasTerminated returns true, it is possible to call
  /// CancelTerminateExecution in order to continue calling into the engine.
  pub fn can_continue(&self) -> bool {
    unsafe { v8__TryCatch__CanContinue(self) }
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
    unsafe { v8__TryCatch__HasTerminated(self) }
  }

  /// Returns the exception caught by this try/catch block. If no exception has
  /// been caught an empty handle is returned.
  ///
  /// The returned handle is valid until this TryCatch block has been destroyed.
  pub fn exception(&self) -> Option<Local<Value>> {
    unsafe { Local::from_raw(v8__TryCatch__Exception(self)) }
  }

  /// Returns the .stack property of the thrown object. If no .stack
  /// property is present an empty handle is returned.
  pub fn stack_trace<'s>(
    &self,
    scope: &mut Scope,
    context: Local<Context>,
  ) -> Option<Local<'s, Value>> {
    unsafe { scope.to_local(v8__TryCatch__StackTrace(self, context)) }
  }

  /// Returns the message associated with this exception. If there is
  /// no message associated an empty handle is returned.
  ///
  /// The returned handle is valid until this TryCatch block has been
  /// destroyed.
  pub fn message(&self) -> Option<Local<Message>> {
    unsafe { Local::from_raw(v8__TryCatch__Message(self)) }
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
    unsafe { v8__TryCatch__Reset(self) };
  }

  /// Throws the exception caught by this TryCatch in a way that avoids
  /// it being caught again by this same TryCatch. As with ThrowException
  /// it is illegal to execute any JavaScript operations after calling
  /// ReThrow; the caller must return immediately to where the exception
  /// is caught.
  pub fn rethrow(&mut self) -> Option<Local<Value>> {
    unsafe { Local::from_raw(v8__TryCatch__ReThrow(self)) }
  }

  /// Returns true if verbosity is enabled.
  pub fn is_verbose(&self) -> bool {
    unsafe { v8__TryCatch__IsVerbose(self) }
  }

  /// Set verbosity of the external exception handler.
  ///
  /// By default, exceptions that are caught by an external exception
  /// handler are not reported. Call SetVerbose with true on an
  /// external exception handler to have exceptions caught by the
  /// handler reported as if they were not caught.
  pub fn set_verbose(&mut self, value: bool) {
    unsafe { v8__TryCatch__SetVerbose(self, value) };
  }

  /// Set whether or not this TryCatch should capture a Message object
  /// which holds source information about where the exception
  /// occurred. True by default.
  pub fn set_capture_message(&mut self, value: bool) {
    unsafe { v8__TryCatch__SetCaptureMessage(self, value) };
  }
}

impl Drop for TryCatch {
  fn drop(&mut self) {
    // unsafe { v8__TryCatch__DESTRUCT(self) }
  }
}
