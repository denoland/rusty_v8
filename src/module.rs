use crate::support::int;
use crate::support::MaybeBool;
use crate::support::Opaque;
use crate::Context;
use crate::Isolate;
use crate::Local;
use crate::String;
use crate::Value;
use crate::local::MaybeLocal;
use std::mem::MaybeUninit;

// Ideally the return value would be Option<Local<Module>>... but not FFI-safe
type ResolveCallback =
  extern "C" fn(Local<Context>, Local<String>, Local<Module>) -> MaybeLocal;

/// Callback defined in the embedder.  This is responsible for setting
/// the module's exported values with calls to SetSyntheticModuleExport().
/// The callback must return a Value to indicate success (where no
/// exception was thrown) and return an empy MaybeLocal to indicate falure
/// (where an exception was thrown).
///
// Ideally the return value would be Option<Local<Value>>... but not FFI-safe
pub type SyntheticModuleEvaluationSteps =
  extern "C" fn(Local<Context>, Local<Module>) -> MaybeLocal;

extern "C" {
  fn v8__Module__GetStatus(this: *const Module) -> Status;
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
    callback: ResolveCallback,
  ) -> MaybeBool;
  fn v8__Module__Evaluate(
    this: *mut Module,
    context: *mut Context,
  ) -> *mut Value;
  fn v8__Module__CreateSyntheticModule(
    isolate: &mut Isolate,
    module_name: *const String,
    export_names: *const *const String,
    export_names_len: usize,
    evaluation_steps: SyntheticModuleEvaluationSteps,
  ) -> *mut Module;
  fn v8__Module__SetSyntheticModuleExport(
    this: *mut Module,
    isolate: &Isolate,
    export_name: *mut String,
    export_value: *mut Value,
  ) -> MaybeBool;

  fn v8__Location__GetLineNumber(this: &Location) -> int;
  fn v8__Location__GetColumnNumber(this: &Location) -> int;
}

#[repr(C)]
/// A location in JavaScript source.
pub struct Location([usize; 2]);

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
    unsafe { v8__Module__InstantiateModule(self, context, callback) }.into()
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

  /// Creates a new SyntheticModule with the specified export names, where
  /// evaluation_steps will be executed upon module evaluation.
  /// export_names must not contain duplicates.
  /// module_name is used solely for logging/debugging and doesn't affect module
  /// behavior.
  pub fn create_synthetic_module<'sc>(
    isolate: &mut Isolate,
    module_name: Local<String>,
    export_names: Vec<Local<String>>,
    evaluation_steps: SyntheticModuleEvaluationSteps,
  ) -> Local<'sc, Module> {
    let mut exports_: Vec<*const String> = vec![];
    for name in export_names {
      exports_.push(&*name);
    }
    unsafe {
      Local::from_raw(v8__Module__CreateSyntheticModule(
        &mut *isolate,
        &*module_name,
        exports_.as_ptr(),
        exports_.len(),
        evaluation_steps,
      ))
      .unwrap()
    }
  }

  /// Set this module's exported value for the name export_name to the specified
  /// export_value. This method must be called only on Modules created via
  /// CreateSyntheticModule.  An error will be thrown if export_name is not one
  /// of the export_names that were passed in that CreateSyntheticModule call.
  /// Returns Just(true) on success, Nothing<bool>() if an error was thrown.
  #[must_use]
  pub fn set_synthetic_module_export(
    &mut self,
    isolate: &Isolate,
    mut export_name: Local<String>,
    mut export_value: Local<Value>,
  ) -> Option<bool> {
    unsafe {
      v8__Module__SetSyntheticModuleExport(
        self,
        isolate,
        &mut *export_name,
        &mut *export_value,
      )
      .into()
    }
  }
}
