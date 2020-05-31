use std::convert::TryInto;
use std::mem::MaybeUninit;
use std::ptr::null;

use crate::support::int;
use crate::support::MapFnFrom;
use crate::support::MapFnTo;
use crate::support::MaybeBool;
use crate::support::ToCFn;
use crate::support::UnitType;
use crate::Context;
use crate::Local;
use crate::Module;
use crate::String;
use crate::ToLocal;
use crate::Value;

/// Called during Module::instantiate_module. Provided with arguments:
/// (context, specifier, referrer). Return None on error.
///
/// Note: this callback has an unusual signature due to ABI incompatibilities
/// between Rust and C++. However end users can implement the callback as
/// follows; it'll be automatically converted.
///
/// ```rust,ignore
///   fn my_resolve_callback<'a>(
///      context: v8::Local<'a, v8::Context>,
///      specifier: v8::Local<'a, v8::String>,
///      referrer: v8::Local<'a, v8::Module>,
///   ) -> Option<v8::Local<'a, v8::Module>> {
///      // ...
///      Some(resolved_module)
///   }
/// ```

// System V AMD64 ABI: Local<Module> returned in a register.
#[cfg(not(target_os = "windows"))]
pub type ResolveCallback<'a> = extern "C" fn(
  Local<'a, Context>,
  Local<'a, String>,
  Local<'a, Module>,
) -> *const Module;

// Windows x64 ABI: Local<Module> returned on the stack.
#[cfg(target_os = "windows")]
pub type ResolveCallback<'a> = extern "C" fn(
  *mut *const Module,
  Local<'a, Context>,
  Local<'a, String>,
  Local<'a, Module>,
) -> *mut *const Module;

impl<'a, F> MapFnFrom<F> for ResolveCallback<'a>
where
  F: UnitType
    + Fn(
      Local<'a, Context>,
      Local<'a, String>,
      Local<'a, Module>,
    ) -> Option<Local<'a, Module>>,
{
  #[cfg(not(target_os = "windows"))]
  fn mapping() -> Self {
    let f = |context, specifier, referrer| {
      (F::get())(context, specifier, referrer)
        .map(|r| -> *const Module { &*r })
        .unwrap_or(null())
    };
    f.to_c_fn()
  }

  #[cfg(target_os = "windows")]
  fn mapping() -> Self {
    let f = |ret_ptr, context, specifier, referrer| {
      let r = (F::get())(context, specifier, referrer)
        .map(|r| -> *const Module { &*r })
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
  fn v8__Module__GetModuleRequestsLength(this: *const Module) -> int;
  fn v8__Module__GetModuleRequest(this: *const Module, i: int)
    -> *const String;
  fn v8__Module__GetModuleRequestLocation(
    this: *const Module,
    i: int,
    out: *mut MaybeUninit<Location>,
  ) -> Location;
  fn v8__Module__GetModuleNamespace(this: *const Module) -> *const Value;
  fn v8__Module__GetIdentityHash(this: *const Module) -> int;
  fn v8__Module__InstantiateModule(
    this: *const Module,
    context: *const Context,
    cb: ResolveCallback,
  ) -> MaybeBool;
  fn v8__Module__Evaluate(
    this: *const Module,
    context: *const Context,
  ) -> *const Value;
  fn v8__Location__GetLineNumber(this: *const Location) -> int;
  fn v8__Location__GetColumnNumber(this: *const Location) -> int;
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

  /// Returns the number of modules requested by this module.
  pub fn get_module_requests_length(&self) -> usize {
    unsafe { v8__Module__GetModuleRequestsLength(self) }
      .try_into()
      .unwrap()
  }

  /// Returns the ith module specifier in this module.
  /// i must be < self.get_module_requests_length() and >= 0.
  pub fn get_module_request(&self, i: usize) -> Local<String> {
    // Note: the returned value is not actually stored in a HandleScope,
    // therefore we don't need a scope object here.
    unsafe {
      Local::from_raw(v8__Module__GetModuleRequest(self, i.try_into().unwrap()))
    }
    .unwrap()
  }

  /// Returns the source location (line number and column number) of the ith
  /// module specifier's first occurrence in this module.
  pub fn get_module_request_location(&self, i: usize) -> Location {
    let mut out = MaybeUninit::<Location>::uninit();
    unsafe {
      v8__Module__GetModuleRequestLocation(
        self,
        i.try_into().unwrap(),
        &mut out,
      );
      out.assume_init()
    }
  }

  /// Returns the identity hash for this object.
  pub fn get_identity_hash(&self) -> int {
    unsafe { v8__Module__GetIdentityHash(self) }
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
  #[must_use]
  pub fn instantiate_module<'a>(
    &mut self,
    context: Local<Context>,
    callback: impl MapFnTo<ResolveCallback<'a>>,
  ) -> Option<bool> {
    unsafe {
      v8__Module__InstantiateModule(self, &*context, callback.map_fn_to())
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
  pub fn evaluate<'sc>(
    &self,
    scope: &mut impl ToLocal<'sc>,
    context: Local<Context>,
  ) -> Option<Local<'sc, Value>> {
    unsafe { scope.to_local(v8__Module__Evaluate(&*self, &*context)) }
  }
}
