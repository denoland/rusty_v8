use crate::support::int;
use crate::support::MaybeBool;
use crate::support::Opaque;
use crate::Context;
use crate::Local;
use crate::String;
use crate::Value;
use std::mem::MaybeUninit;

#[allow(non_camel_case_types)]
type v8__Module__ResolveCallback =
  extern "C" fn(Local<Context>, Local<String>, Local<Module>) -> *mut Module;

/// Called during Module::instantiate_module. Provided with arguments:
/// (context, specifier, referrer)
/// Return null on error.
/// Hint: to tranform Local<Module> to *mut Module do this:
///   &mut *module
pub type ResolveCallback =
  fn(Local<Context>, Local<String>, Local<Module>) -> *mut Module;

extern "C" {
  fn v8__Module__GetStatus(this: *const Module) -> ModuleStatus;
  fn v8__Module__GetException(this: *const Module) -> *mut Value;
  fn v8__Module__GetModuleRequestsLength(this: *const Module) -> int;
  fn v8__Module__GetModuleRequest(this: *const Module, i: usize)
    -> *mut String;
  fn v8__Module__GetModuleRequestLocation(
    this: *const Module,
    i: usize,
    out: &mut MaybeUninit<Location>,
  ) -> Location;
  fn v8__Module__GetIdentityHash(this: *const Module) -> int;
  fn v8__Module__InstantiateModule(
    this: *mut Module,
    context: Local<Context>,
    callback: v8__Module__ResolveCallback,
  ) -> MaybeBool;
  fn v8__Module__Evaluate(
    this: *mut Module,
    context: *mut Context,
  ) -> *mut Value;
  fn v8__Location__GetLineNumber(this: &Location) -> int;
  fn v8__Location__GetColumnNumber(this: &Location) -> int;
}

#[repr(C)]
/// A location in JavaScript source.
pub struct Location([usize; 1]);

impl Location {
  pub fn get_line_number(&self) -> int {
    unsafe { v8__Location__GetLineNumber(self) }
  }

  pub fn get_column_number(&self) -> int {
    unsafe { v8__Location__GetColumnNumber(self) }
  }
}

/// The different states a module can be in.
///
/// This corresponds to the states used in ECMAScript except that "evaluated"
/// is split into kEvaluated and kErrored, indicating success and failure,
/// respectively.
#[derive(Debug, PartialEq)]
#[repr(C)]
pub enum ModuleStatus {
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
  pub fn get_status(&self) -> ModuleStatus {
    unsafe { v8__Module__GetStatus(self) }
  }

  /// For a module in kErrored status, this returns the corresponding exception.
  pub fn get_exception(&self) -> Local<Value> {
    unsafe { Local::from_raw(v8__Module__GetException(self)).unwrap() }
  }

  /// Returns the number of modules requested by this module.
  pub fn get_module_requests_length(&self) -> int {
    unsafe { v8__Module__GetModuleRequestsLength(self) }
  }

  /// Returns the ith module specifier in this module.
  /// i must be < self.get_module_requests_length() and >= 0.
  pub fn get_module_request(&self, i: usize) -> Local<String> {
    unsafe { Local::from_raw(v8__Module__GetModuleRequest(self, i)).unwrap() }
  }

  /// Returns the source location (line number and column number) of the ith
  /// module specifier's first occurrence in this module.
  pub fn get_module_request_location(&self, i: usize) -> Location {
    let mut out = MaybeUninit::<Location>::uninit();
    unsafe {
      v8__Module__GetModuleRequestLocation(self, i, &mut out);
      out.assume_init()
    }
  }

  /// Returns the identity hash for this object.
  pub fn get_identity_hash(&self) -> int {
    unsafe { v8__Module__GetIdentityHash(self) }
  }

  /// Instantiates the module and its dependencies.
  ///
  /// Returns an empty Maybe<bool> if an exception occurred during
  /// instantiation. (In the case where the callback throws an exception, that
  /// exception is propagated.)
  #[must_use]
  pub fn instantiate_module(
    &mut self,
    context: Local<Context>,
    callback: ResolveCallback,
  ) -> Option<bool> {
    use std::sync::Mutex;
    lazy_static! {
      static ref RESOLVE_CALLBACK: Mutex<Option<ResolveCallback>> =
        Mutex::new(None);
      static ref INSTANTIATE_LOCK: Mutex<()> = Mutex::new(());
    }
    let instantiate_guard = INSTANTIATE_LOCK.lock().unwrap();

    {
      let mut guard = RESOLVE_CALLBACK.lock().unwrap();
      *guard = Some(callback);
    }

    extern "C" fn c_cb(
      context: Local<Context>,
      specifier: Local<String>,
      referrer: Local<Module>,
    ) -> *mut Module {
      let guard = RESOLVE_CALLBACK.lock().unwrap();
      let cb = guard.unwrap();
      cb(context, specifier, referrer)
    }
    let r =
      unsafe { v8__Module__InstantiateModule(self, context, c_cb) }.into();
    drop(instantiate_guard);
    r
  }

  /// Evaluates the module and its dependencies.
  ///
  /// If status is kInstantiated, run the module's code. On success, set status
  /// to kEvaluated and return the completion value; on failure, set status to
  /// kErrored and propagate the thrown exception (which is then also available
  /// via |GetException|).
  #[must_use]
  pub fn evaluate(
    &mut self,
    mut context: Local<Context>,
  ) -> Option<Local<Value>> {
    unsafe { Local::from_raw(v8__Module__Evaluate(&mut *self, &mut *context)) }
  }
}
