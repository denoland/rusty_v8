use std::hash::Hash;
use std::hash::Hasher;
use std::mem::MaybeUninit;
use std::ptr::null;

use crate::support::int;
use crate::support::MapFnFrom;
use crate::support::MapFnTo;
use crate::support::MaybeBool;
use crate::support::ToCFn;
use crate::support::UnitType;
use crate::Context;
use crate::FixedArray;
use crate::HandleScope;
use crate::Isolate;
use crate::Local;
use crate::Module;
use crate::ModuleRequest;
use crate::String;
use crate::UnboundModuleScript;
use crate::Value;

/// Called during Module::instantiate_module. Provided with arguments:
/// (context, specifier, import_assertions, referrer). Return None on error.
///
/// Note: this callback has an unusual signature due to ABI incompatibilities
/// between Rust and C++. However end users can implement the callback as
/// follows; it'll be automatically converted.
///
/// ```rust,ignore
///   fn my_resolve_callback<'a>(
///      context: v8::Local<'a, v8::Context>,
///      specifier: v8::Local<'a, v8::String>,
///      import_assertions: v8::Local<'a, v8::FixedArray>,
///      referrer: v8::Local<'a, v8::Module>,
///   ) -> Option<v8::Local<'a, v8::Module>> {
///      // ...
///      Some(resolved_module)
///   }
/// ```

// System V AMD64 ABI: Local<Module> returned in a register.
#[cfg(not(target_os = "windows"))]
pub type ResolveModuleCallback<'a> = extern "C" fn(
  Local<'a, Context>,
  Local<'a, String>,
  Local<'a, FixedArray>,
  Local<'a, Module>,
) -> *const Module;

// Windows x64 ABI: Local<Module> returned on the stack.
#[cfg(target_os = "windows")]
pub type ResolveModuleCallback<'a> = extern "C" fn(
  *mut *const Module,
  Local<'a, Context>,
  Local<'a, String>,
  Local<'a, FixedArray>,
  Local<'a, Module>,
) -> *mut *const Module;

impl<'a, F> MapFnFrom<F> for ResolveModuleCallback<'a>
where
  F: UnitType
    + Fn(
      Local<'a, Context>,
      Local<'a, String>,
      Local<'a, FixedArray>,
      Local<'a, Module>,
    ) -> Option<Local<'a, Module>>,
{
  #[cfg(not(target_os = "windows"))]
  fn mapping() -> Self {
    let f = |context, specifier, import_assertions, referrer| {
      (F::get())(context, specifier, import_assertions, referrer)
        .map(|r| -> *const Module { &*r })
        .unwrap_or(null())
    };
    f.to_c_fn()
  }

  #[cfg(target_os = "windows")]
  fn mapping() -> Self {
    let f = |ret_ptr, context, specifier, import_assertions, referrer| {
      let r = (F::get())(context, specifier, import_assertions, referrer)
        .map(|r| -> *const Module { &*r })
        .unwrap_or(null());
      unsafe { std::ptr::write(ret_ptr, r) }; // Write result to stack.
      ret_ptr // Return stack pointer to the return value.
    };
    f.to_c_fn()
  }
}

// System V AMD64 ABI: Local<Value> returned in a register.
#[cfg(not(target_os = "windows"))]
pub type SyntheticModuleEvaluationSteps<'a> =
  extern "C" fn(Local<'a, Context>, Local<'a, Module>) -> *const Value;

// Windows x64 ABI: Local<Value> returned on the stack.
#[cfg(target_os = "windows")]
pub type SyntheticModuleEvaluationSteps<'a> =
  extern "C" fn(
    *mut *const Value,
    Local<'a, Context>,
    Local<'a, Module>,
  ) -> *mut *const Value;

impl<'a, F> MapFnFrom<F> for SyntheticModuleEvaluationSteps<'a>
where
  F: UnitType
    + Fn(Local<'a, Context>, Local<'a, Module>) -> Option<Local<'a, Value>>,
{
  #[cfg(not(target_os = "windows"))]
  fn mapping() -> Self {
    let f = |context, module| {
      (F::get())(context, module)
        .map(|r| -> *const Value { &*r })
        .unwrap_or(null())
    };
    f.to_c_fn()
  }

  #[cfg(target_os = "windows")]
  fn mapping() -> Self {
    let f = |ret_ptr, context, module| {
      let r = (F::get())(context, module)
        .map(|r| -> *const Value { &*r })
        .unwrap_or(null());
      unsafe { std::ptr::write(ret_ptr, r) }; // Write result to stack.
      ret_ptr // Return stack pointer to the return value.
    };
    f.to_c_fn()
  }
}

extern "C" {
  fn v8__Module__GetStatus(this: *const Module) -> ModuleStatus;
  fn v8__Module__GetException(this: *const Module) -> *const Value;
  fn v8__Module__GetModuleRequests(this: *const Module) -> *const FixedArray;
  fn v8__Module__SourceOffsetToLocation(
    this: *const Module,
    offset: int,
    out: *mut MaybeUninit<Location>,
  ) -> Location;
  fn v8__Module__GetModuleNamespace(this: *const Module) -> *const Value;
  fn v8__Module__GetIdentityHash(this: *const Module) -> int;
  fn v8__Module__ScriptId(this: *const Module) -> int;
  fn v8__Module__InstantiateModule(
    this: *const Module,
    context: *const Context,
    cb: ResolveModuleCallback,
  ) -> MaybeBool;
  fn v8__Module__Evaluate(
    this: *const Module,
    context: *const Context,
  ) -> *const Value;
  fn v8__Module__IsSourceTextModule(this: *const Module) -> bool;
  fn v8__Module__IsSyntheticModule(this: *const Module) -> bool;
  fn v8__Module__CreateSyntheticModule(
    isolate: *const Isolate,
    module_name: *const String,
    export_names_len: usize,
    export_names_raw: *const *const String,
    evaluation_steps: SyntheticModuleEvaluationSteps,
  ) -> *const Module;
  fn v8__Module__SetSyntheticModuleExport(
    this: *const Module,
    isolate: *const Isolate,
    export_name: *const String,
    export_value: *const Value,
  ) -> MaybeBool;
  fn v8__Module__GetUnboundModuleScript(
    this: *const Module,
  ) -> *const UnboundModuleScript;
  fn v8__Location__GetLineNumber(this: *const Location) -> int;
  fn v8__Location__GetColumnNumber(this: *const Location) -> int;
  fn v8__ModuleRequest__GetSpecifier(
    this: *const ModuleRequest,
  ) -> *const String;
  fn v8__ModuleRequest__GetSourceOffset(this: *const ModuleRequest) -> int;
  fn v8__ModuleRequest__GetImportAssertions(
    this: *const ModuleRequest,
  ) -> *const FixedArray;
}

/// A location in JavaScript source.
#[repr(C)]
#[derive(Debug)]
pub struct Location([i32; 2]);

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

impl Module {
  /// Returns the module's current status.
  pub fn get_status(&self) -> ModuleStatus {
    unsafe { v8__Module__GetStatus(self) }
  }

  /// For a module in kErrored status, this returns the corresponding exception.
  pub fn get_exception(&self) -> Local<Value> {
    // Note: the returned value is not actually stored in a HandleScope,
    // therefore we don't need a scope object here.
    unsafe { Local::from_raw(v8__Module__GetException(self)) }.unwrap()
  }

  /// Returns the ModuleRequests for this module.
  pub fn get_module_requests(&self) -> Local<FixedArray> {
    unsafe { Local::from_raw(v8__Module__GetModuleRequests(self)) }.unwrap()
  }

  /// For the given source text offset in this module, returns the corresponding
  /// Location with line and column numbers.
  pub fn source_offset_to_location(&self, offset: int) -> Location {
    let mut out = MaybeUninit::<Location>::uninit();
    unsafe {
      v8__Module__SourceOffsetToLocation(self, offset, &mut out);
      out.assume_init()
    }
  }

  /// The `Module` specific equivalent of `Data::get_hash()`.
  /// This function is kept around for testing purposes only.
  #[doc(hidden)]
  pub fn get_identity_hash(&self) -> int {
    unsafe { v8__Module__GetIdentityHash(self) }
  }

  /// Returns the underlying script's id.
  ///
  /// The module must be a SourceTextModule and must not have an Errored status.
  pub fn script_id(&self) -> Option<int> {
    if !self.is_source_text_module() {
      return None;
    }
    if self.get_status() == ModuleStatus::Errored {
      return None;
    }
    Some(unsafe { v8__Module__ScriptId(self) })
  }

  /// Returns the namespace object of this module.
  ///
  /// The module's status must be at least kInstantiated.
  pub fn get_module_namespace(&self) -> Local<Value> {
    // Note: the returned value is not actually stored in a HandleScope,
    // therefore we don't need a scope object here.
    unsafe { Local::from_raw(v8__Module__GetModuleNamespace(self)).unwrap() }
  }

  /// Instantiates the module and its dependencies.
  ///
  /// Returns an empty Maybe<bool> if an exception occurred during
  /// instantiation. (In the case where the callback throws an exception, that
  /// exception is propagated.)
  ///
  /// NOTE: requires to set `--harmony-import-assertions` V8 flag.
  #[must_use]
  pub fn instantiate_module<'a>(
    &self,
    scope: &mut HandleScope,
    callback: impl MapFnTo<ResolveModuleCallback<'a>>,
  ) -> Option<bool> {
    unsafe {
      v8__Module__InstantiateModule(
        self,
        &*scope.get_current_context(),
        callback.map_fn_to(),
      )
    }
    .into()
  }

  /// Evaluates the module and its dependencies.
  ///
  /// If status is kInstantiated, run the module's code. On success, set status
  /// to kEvaluated and return the completion value; on failure, set status to
  /// kErrored and propagate the thrown exception (which is then also available
  /// via |GetException|).
  #[must_use]
  pub fn evaluate<'s>(
    &self,
    scope: &mut HandleScope<'s>,
  ) -> Option<Local<'s, Value>> {
    unsafe {
      scope
        .cast_local(|sd| v8__Module__Evaluate(&*self, sd.get_current_context()))
    }
  }

  /// Returns whether the module is a SourceTextModule.
  pub fn is_source_text_module(&self) -> bool {
    unsafe { v8__Module__IsSourceTextModule(&*self) }
  }

  /// Returns whether the module is a SyntheticModule.
  pub fn is_synthetic_module(&self) -> bool {
    unsafe { v8__Module__IsSyntheticModule(&*self) }
  }

  /// Creates a new SyntheticModule with the specified export names, where
  /// evaluation_steps will be executed upon module evaluation.
  /// export_names must not contain duplicates.
  /// module_name is used solely for logging/debugging and doesn't affect module
  /// behavior.
  pub fn create_synthetic_module<'s, 'a>(
    scope: &mut HandleScope<'s>,
    module_name: Local<String>,
    export_names: &[Local<String>],
    evaluation_steps: impl MapFnTo<SyntheticModuleEvaluationSteps<'a>>,
  ) -> Local<'s, Module> {
    let export_names = Local::slice_into_raw(export_names);
    let export_names_len = export_names.len();
    let export_names = export_names.as_ptr();
    unsafe {
      scope
        .cast_local(|sd| {
          v8__Module__CreateSyntheticModule(
            sd.get_isolate_ptr(),
            &*module_name,
            export_names_len,
            export_names,
            evaluation_steps.map_fn_to(),
          )
        })
        .unwrap()
    }
  }

  /// Set this module's exported value for the name export_name to the specified
  /// export_value. This method must be called only on Modules created via
  /// create_synthetic_module.  An error will be thrown if export_name is not one
  /// of the export_names that were passed in that create_synthetic_module call.
  /// Returns Some(true) on success, None if an error was thrown.
  #[must_use]
  pub fn set_synthetic_module_export(
    &self,
    scope: &mut HandleScope,
    export_name: Local<String>,
    export_value: Local<Value>,
  ) -> Option<bool> {
    unsafe {
      v8__Module__SetSyntheticModuleExport(
        &*self,
        scope.get_isolate_ptr(),
        &*export_name,
        &*export_value,
      )
    }
    .into()
  }

  pub fn get_unbound_module_script<'s>(
    &self,
    scope: &mut HandleScope<'s>,
  ) -> Local<'s, UnboundModuleScript> {
    unsafe {
      scope
        .cast_local(|_| v8__Module__GetUnboundModuleScript(self))
        .unwrap()
    }
  }
}

impl Hash for Module {
  fn hash<H: Hasher>(&self, state: &mut H) {
    state.write_i32(self.get_identity_hash());
  }
}

impl ModuleRequest {
  /// Returns the module specifier for this ModuleRequest.
  pub fn get_specifier(&self) -> Local<String> {
    unsafe { Local::from_raw(v8__ModuleRequest__GetSpecifier(self)) }.unwrap()
  }

  /// Returns the source code offset of this module request.
  /// Use Module::source_offset_to_location to convert this to line/column numbers.
  pub fn get_source_offset(&self) -> int {
    unsafe { v8__ModuleRequest__GetSourceOffset(self) }
  }

  /// Contains the import assertions for this request in the form:
  /// [key1, value1, source_offset1, key2, value2, source_offset2, ...].
  /// The keys and values are of type v8::String, and the source offsets are of
  /// type Int32. Use Module::source_offset_to_location to convert the source
  /// offsets to Locations with line/column numbers.
  ///
  /// All assertions present in the module request will be supplied in this
  /// list, regardless of whether they are supported by the host. Per
  /// https://tc39.es/proposal-import-assertions/#sec-hostgetsupportedimportassertions,
  /// hosts are expected to ignore assertions that they do not support (as
  /// opposed to, for example, triggering an error if an unsupported assertion is
  /// present).
  pub fn get_import_assertions(&self) -> Local<FixedArray> {
    unsafe { Local::from_raw(v8__ModuleRequest__GetImportAssertions(self)) }
      .unwrap()
  }
}
