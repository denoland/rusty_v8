use crate::support::int;
use crate::support::Opaque;
use crate::Context;
use crate::Local;
use crate::Value;

extern "C" {}

/// The different states a module can be in.
///
/// This corresponds to the states used in ECMAScript except that "evaluated"
/// is split into kEvaluated and kErrored, indicating success and failure,
/// respectively.
#[derive(Debug, PartialEq)]
#[repr(C)]
pub enum Status {
  Uninstantiated,
  Instantiating,
  Instantiated,
  Evaluating,
  Evaluated,
  Errored,
}

#[repr(C)]
pub struct Module(Opaque);

/// A compiled JavaScript module.
impl Module {
  /// Returns the module's current status.
  pub fn get_status(&self) -> Status {
    unimplemented!();
  }

  /// For a module in kErrored status, this returns the corresponding exception.
  pub fn get_exception(&self) -> Local<Value> {
    unimplemented!();
  }

  /// Returns the number of modules requested by this module.
  pub fn get_module_requests_length(&self) -> int {
    unimplemented!();
  }

  /// Returns the ith module specifier in this module.
  /// i must be < self.get_module_requests_length() and >= 0.
  pub fn get_module_request(&self, _i: usize) -> Local<String> {
    unimplemented!();
  }

  /// Returns the identity hash for this object.
  pub fn get_identity_hash(&self) -> int {
    unimplemented!();
  }

  /// Instantiates the module and its dependencies.
  ///
  /// Returns an empty Maybe<bool> if an exception occurred during
  /// instantiation. (In the case where the callback throws an exception, that
  /// exception is propagated.)
  #[must_use]
  pub fn instantiate_module(
    &self,
    _context: Local<Context>,
    _callback: Box<ResolveCallback>,
  ) -> Option<bool> {
    unimplemented!();
  }

  /// Evaluates the module and its dependencies.
  ///
  /// If status is kInstantiated, run the module's code. On success, set status
  /// to kEvaluated and return the completion value; on failure, set status to
  /// kErrored and propagate the thrown exception (which is then also available
  /// via |GetException|).
  #[must_use]
  pub fn evaluate(&self, _context: Local<Context>) -> Option<Local<Value>> {
    unimplemented!();
  }
}

type ResolveCallback =
  dyn Fn(Local<Context>, Local<String>, Local<Module>) -> Option<Local<Module>>;
