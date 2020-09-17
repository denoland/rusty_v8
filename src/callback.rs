// Copyright 2019-2020 the Deno authors. All rights reserved. MIT license.

//! # Why these `impl_some_callback!()` macros?
//!
//! It appears that Rust is unable to use information that is available from
//! super traits for type and/or lifetime inference purposes. This causes it to
//! rejects well formed closures for no reason at all. These macros provide
//! a workaround for this issue.
//!
//! ```rust,ignore
//! // First, require that *all* implementors of `MyCallback` also implement a
//! // `Fn(...)` supertrait.
//! trait MyCallback: Sized + for<'a> Fn(&'a u32) -> &'a u32 {}
//!
//! // Povide a blanket `MyCallback` impl for all compatible types. This makes
//! // `MyCallback` essentially an alias of `Fn(&u32) -> &u32`.
//! impl<F> MyCallback for F where F: Sized + for<'a> Fn(&'a u32) -> &'a u32 {}
//!
//! // Define two functions with a parameter of the trait we just defined.
//! // One function uses the shorthand 'MyCallback', the other uses exactly the
//! // same `for<'a> Fn(...)` notation as we did when specifying the supertrait.
//! fn do_this(callback: impl MyCallback) {
//!   let val = 123u32;
//!   let _ = callback(&val);
//! }
//! fn do_that(callback: impl for<'a> Fn(&'a u32) -> &'a u32) {
//!   let val = 456u32;
//!   let _ = callback(&val);
//! }
//!
//! // Both of the above functions will accept an ordinary function (with a
//! // matching signature) as the only argument.
//! fn test_cb(a: &u32) -> &u32 { a }
//! do_this(test_cb);   // Ok!
//! do_that(test_cb);   // Ok!
//!
//! // However, when attempting to do the same with a closure, Rust loses it
//! // as it tries to reconcile the type of the closure with `impl MyCallback`.
//! do_this(|a| a);     // "Type mismatch resolving..."
//! do_that(|a| a);     // Ok!
//!
//! // Note that even when we explicitly define the closure's argument and
//! // return types, the Rust compiler still wants nothing to do with it...
//! //   ⚠ Type mismatch resolving
//! //   ⚠ `for<'a> <[closure] as FnOnce<(&'a u32,)>>::Output == &'a u32`.
//! //   ⚠ Expected bound lifetime parameter 'a, found concrete lifetime.
//! do_this(|a: &u32| -> &u32 { a });
//!
//! // The function signature used in this example is short and simple, but
//! // real world `Fn` traits tend to get long and complicated. These macros
//! // are there to make closure syntax possible without replicating the full
//! // `Fn(...)` trait definition over and over again.
//! macro_rules! impl_my_callback {
//!   () => {
//!     impl MyCallback + for<'a> Fn(&'a u32) -> &'a u32
//!   };
//! }
//!
//! // It lets us define a function with a `MyCallback` parameter as follows:
//! fn do_such(callback: impl_my_callback!()) {
//!   let val = 789u32;
//!   let _ = callback(&val);
//! }
//!
//! // And, as expected, we can pass either a function or a closure for the
//! // first argument.
//! do_such(test_cb);   // Ok!
//! do_such(|a| a);     // Ok!
//! ```

#![allow(clippy::needless_lifetimes)]
#![allow(clippy::too_many_arguments)]

use std::cell::Cell;
use std::ffi::c_void;
use std::ffi::CStr;
use std::os::raw::c_char;
use std::os::raw::c_double;
use std::os::raw::c_int;
use std::os::raw::c_uchar;
use std::slice;

use crate::function::FunctionCallbackInfo;
use crate::function::PropertyCallbackInfo;
use crate::scope::CallbackScope;
use crate::support::Opaque;
use crate::support::UnitType;
use crate::Array;
use crate::Boolean;
use crate::Context;
use crate::FunctionCallbackArguments;
use crate::HandleScope;
use crate::Integer;
use crate::Isolate;
use crate::Local;
use crate::Message;
use crate::Module;
use crate::Name;
use crate::Object;
use crate::Promise;
use crate::PromiseRejectMessage;
use crate::PropertyCallbackArguments;
use crate::ReturnValue;
use crate::ScriptOrModule;
use crate::SharedArrayBuffer;
use crate::StartupData;
use crate::String;
use crate::Value;

// Placeholder types that don't have Rust bindings yet.
#[repr(C)]
pub struct AtomicsWaitWakeHandle(Opaque);
#[repr(C)]
pub struct JitCodeEvent(Opaque);
#[repr(C)]
pub struct PropertyDescriptor(Opaque);

#[cfg(target_family = "windows")]
#[repr(C)]
pub struct EXCEPTION_POINTERS(Opaque);

#[repr(C)]
pub enum AccessType {
  _TODO,
}
#[repr(C)]
pub enum AtomicsWaitEvent {
  _TODO,
}
#[repr(C)]
pub enum CrashKeyId {
  _TODO,
}
#[repr(C)]
pub enum GCCallbackFlags {
  _TODO,
}
#[repr(C)]
pub enum GCType {
  _TODO,
}
#[repr(C)]
pub enum PromiseHookType {
  _TODO,
}
#[repr(C)]
pub enum UseCounterFeature {
  _TODO,
}

/// Rust representation of a C++ `std::string`.
#[repr(C)]
pub struct CxxString(Opaque);

impl<'a> From<&'a CxxString> for &'a [u8] {
  fn from(_cxx_str: &'a CxxString) -> Self {
    unimplemented!()
  }
}

impl<'a> From<&'a CxxString> for &'a CStr {
  fn from(_cxx_str: &'a CxxString) -> Self {
    unimplemented!()
  }
}

// Notes:
// * This enum should really be #[repr(bool)] but Rust doesn't support that.
// * It must have the same layout as the C++ struct. Do not reorder fields!
#[repr(u8)]
pub enum ModifyCodeGenerationFromStringsResult<'s> {
  /// Block the codegen algorithm.
  Block,
  /// Proceed with the codegen algorithm. Otherwise, block it.
  Allow {
    /// Overwrite the original source with this string, if present.
    /// Use the original source if empty.
    modified_source: Option<Local<'s, String>>,
  },
}

// === ModuleResolveCallback ===

/// # Example
///
/// ```
/// # use rusty_v8 as v8;
/// #
/// fn module_resolve_callback_example<'s>(
///   scope: &mut v8::HandleScope<'s>,
///   specifier: v8::Local<'s, v8::String>,
///   referrer: v8::Local<'s, v8::Module>,
/// ) -> Option<v8::Local<'s, v8::Module>> {
///   todo!()
/// }
/// ```
pub trait ModuleResolveCallback:
  UnitType
  + for<'s> FnOnce(
    &mut HandleScope<'s>,
    Local<'s, String>,
    Local<'s, Module>,
  ) -> Option<Local<'s, Module>>
{
}

impl<F> ModuleResolveCallback for F where
  F: UnitType
    + for<'s> FnOnce(
      &mut HandleScope<'s>,
      Local<'s, String>,
      Local<'s, Module>,
    ) -> Option<Local<'s, Module>>
{
}

#[macro_export]
macro_rules! impl_module_resolve_callback {
  () => {
    impl $crate::ModuleResolveCallback
    + for<'__s> ::std::ops::FnOnce(
      &mut $crate::HandleScope<'__s>,
      $crate::Local<'__s, $crate::String>,
      $crate::Local<'__s, $crate::Module>,
    ) -> ::std::option::Option<$crate::Local<'__s, $crate::Module>>
  };
}

#[cfg(target_family = "unix")]
#[repr(transparent)]
pub(crate) struct RawModuleResolveCallback(
  for<'s> extern "C" fn(
    Local<'s, Context>,
    Local<'s, String>,
    Local<'s, Module>,
  ) -> Option<Local<'s, Module>>,
);

#[cfg(all(target_family = "windows", target_arch = "x86_64"))]
#[repr(transparent)]
pub(crate) struct RawModuleResolveCallback(
  for<'s> extern "C" fn(
    *mut Option<Local<'s, Module>>,
    Local<'s, Context>,
    Local<'s, String>,
    Local<'s, Module>,
  ) -> *mut Option<Local<'s, Module>>,
);

impl<F: ModuleResolveCallback> From<F> for RawModuleResolveCallback {
  fn from(_: F) -> Self {
    #[inline(always)]
    fn signature_adapter<'s, F: ModuleResolveCallback>(
      context: Local<'s, Context>,
      specifier: Local<'s, String>,
      referrer: Local<'s, Module>,
    ) -> Option<Local<'s, Module>> {
      let scope = &mut unsafe { CallbackScope::new(context) };
      (F::get())(scope, specifier, referrer)
    }

    #[cfg(target_family = "unix")]
    #[inline(always)]
    extern "C" fn abi_adapter<'s, F: ModuleResolveCallback>(
      context: Local<'s, Context>,
      specifier: Local<'s, String>,
      referrer: Local<'s, Module>,
    ) -> Option<Local<'s, Module>> {
      signature_adapter::<F>(context, specifier, referrer)
    }

    #[cfg(all(target_family = "windows", target_arch = "x86_64"))]
    #[inline(always)]
    extern "C" fn abi_adapter<'s, F: ModuleResolveCallback>(
      return_value: *mut Option<Local<'s, Module>>,
      context: Local<'s, Context>,
      specifier: Local<'s, String>,
      referrer: Local<'s, Module>,
    ) -> *mut Option<Local<'s, Module>> {
      unsafe {
        std::ptr::write(
          return_value,
          signature_adapter::<F>(context, specifier, referrer),
        );
        return_value
      }
    }

    Self(abi_adapter::<F>)
  }
}

#[cfg(test)]
fn mock_module_resolve_callback<'s>(
  _scope: &mut HandleScope<'s>,
  _specifier: Local<'s, String>,
  _referrer: Local<'s, Module>,
) -> Option<Local<'s, Module>> {
  unimplemented!()
}

#[test]
fn module_resolve_callback_as_type_param() {
  fn pass_as_type_param<F: ModuleResolveCallback>(
    _: F,
  ) -> RawModuleResolveCallback {
    F::get().into()
  }
  let _ = pass_as_type_param(mock_module_resolve_callback);
}

#[test]
fn module_resolve_callback_as_impl_trait() {
  fn pass_as_impl_trait(
    f: impl ModuleResolveCallback,
  ) -> RawModuleResolveCallback {
    f.into()
  }
  let _ = pass_as_impl_trait(mock_module_resolve_callback);
}

#[test]
fn module_resolve_callback_as_impl_macro() {
  fn pass_as_impl_macro(
    f: impl_module_resolve_callback!(),
  ) -> RawModuleResolveCallback {
    f.into()
  }
  let _ = pass_as_impl_macro(mock_module_resolve_callback);
  let _ = pass_as_impl_macro(|_scope, _specifier, _referrer| unimplemented!());
}

// === SyntheticModuleEvaluationSteps ===

/// # Example
///
/// ```
/// # use rusty_v8 as v8;
/// #
/// fn synthetic_module_evaluation_steps_example<'s>(
///   scope: &mut v8::HandleScope<'s>,
///   module: v8::Local<'s, v8::Module>,
/// ) -> Option<v8::Local<'s, v8::Value>> {
///   todo!()
/// }
/// ```
pub trait SyntheticModuleEvaluationSteps:
  UnitType
  + for<'s> FnOnce(
    &mut HandleScope<'s>,
    Local<'s, Module>,
  ) -> Option<Local<'s, Value>>
{
}

impl<F> SyntheticModuleEvaluationSteps for F where
  F: UnitType
    + for<'s> FnOnce(
      &mut HandleScope<'s>,
      Local<'s, Module>,
    ) -> Option<Local<'s, Value>>
{
}

#[macro_export]
macro_rules! impl_synthetic_module_evaluation_steps {
  () => {
    impl $crate::SyntheticModuleEvaluationSteps
    + for<'__s> ::std::ops::FnOnce(
      &mut $crate::HandleScope<'__s>,
      $crate::Local<'__s, $crate::Module>,
    ) -> ::std::option::Option<$crate::Local<'__s, $crate::Value>>
  };
}

#[cfg(target_family = "unix")]
#[repr(transparent)]
pub(crate) struct RawSyntheticModuleEvaluationSteps(
  for<'s> extern "C" fn(
    Local<'s, Context>,
    Local<'s, Module>,
  ) -> Option<Local<'s, Value>>,
);

#[cfg(all(target_family = "windows", target_arch = "x86_64"))]
#[repr(transparent)]
pub(crate) struct RawSyntheticModuleEvaluationSteps(
  for<'s> extern "C" fn(
    *mut Option<Local<'s, Value>>,
    Local<'s, Context>,
    Local<'s, Module>,
  ) -> *mut Option<Local<'s, Value>>,
);

impl<F: SyntheticModuleEvaluationSteps> From<F>
  for RawSyntheticModuleEvaluationSteps
{
  fn from(_: F) -> Self {
    #[inline(always)]
    fn signature_adapter<'s, F: SyntheticModuleEvaluationSteps>(
      context: Local<'s, Context>,
      module: Local<'s, Module>,
    ) -> Option<Local<'s, Value>> {
      let scope = &mut unsafe { CallbackScope::new(context) };
      (F::get())(scope, module)
    }

    #[cfg(target_family = "unix")]
    #[inline(always)]
    extern "C" fn abi_adapter<'s, F: SyntheticModuleEvaluationSteps>(
      context: Local<'s, Context>,
      module: Local<'s, Module>,
    ) -> Option<Local<'s, Value>> {
      signature_adapter::<F>(context, module)
    }

    #[cfg(all(target_family = "windows", target_arch = "x86_64"))]
    #[inline(always)]
    extern "C" fn abi_adapter<'s, F: SyntheticModuleEvaluationSteps>(
      return_value: *mut Option<Local<'s, Value>>,
      context: Local<'s, Context>,
      module: Local<'s, Module>,
    ) -> *mut Option<Local<'s, Value>> {
      unsafe {
        std::ptr::write(return_value, signature_adapter::<F>(context, module));
        return_value
      }
    }

    Self(abi_adapter::<F>)
  }
}

#[cfg(test)]
fn mock_synthetic_module_evaluation_steps<'s>(
  _scope: &mut HandleScope<'s>,
  _module: Local<'s, Module>,
) -> Option<Local<'s, Value>> {
  unimplemented!()
}

#[test]
fn synthetic_module_evaluation_steps_as_type_param() {
  fn pass_as_type_param<F: SyntheticModuleEvaluationSteps>(
    _: F,
  ) -> RawSyntheticModuleEvaluationSteps {
    F::get().into()
  }
  let _ = pass_as_type_param(mock_synthetic_module_evaluation_steps);
}

#[test]
fn synthetic_module_evaluation_steps_as_impl_trait() {
  fn pass_as_impl_trait(
    f: impl SyntheticModuleEvaluationSteps,
  ) -> RawSyntheticModuleEvaluationSteps {
    f.into()
  }
  let _ = pass_as_impl_trait(mock_synthetic_module_evaluation_steps);
}

#[test]
fn synthetic_module_evaluation_steps_as_impl_macro() {
  fn pass_as_impl_macro(
    f: impl_synthetic_module_evaluation_steps!(),
  ) -> RawSyntheticModuleEvaluationSteps {
    f.into()
  }
  let _ = pass_as_impl_macro(mock_synthetic_module_evaluation_steps);
  let _ = pass_as_impl_macro(|_scope, _module| unimplemented!());
}

/// Accessor[Getter|Setter] are used as callback functions when
/// setting|getting a particular property. See Object and ObjectTemplate's
/// method SetAccessor.
///
/// # Example
///
/// ```
/// # use rusty_v8 as v8;
/// #
/// fn accessor_getter_callback_example<'s>(
///   scope: &mut v8::HandleScope<'s>,
///   property: v8::Local<'s, v8::String>,
///   arguments: v8::PropertyCallbackArguments<'s>,
///   return_value: v8::ReturnValue<'s, v8::Value>,
/// ) {
///   todo!();
/// }
/// ```
pub trait AccessorGetterCallback:
  UnitType
  + for<'s> FnOnce(
    &mut HandleScope<'s>,
    Local<'s, String>,
    PropertyCallbackArguments<'s>,
    ReturnValue<'s, Value>,
  )
{
}

impl<F> AccessorGetterCallback for F where
  F: UnitType
    + for<'s> FnOnce(
      &mut HandleScope<'s>,
      Local<'s, String>,
      PropertyCallbackArguments<'s>,
      ReturnValue<'s, Value>,
    )
{
}

#[macro_export]
macro_rules! impl_accessor_getter_callback {
  () => {
    impl $crate::AccessorGetterCallback
    + for<'__s> ::std::ops::FnOnce(
      &mut $crate::HandleScope<'__s>,
      $crate::Local<'__s, $crate::String>,
      $crate::PropertyCallbackArguments<'__s>,
      $crate::ReturnValue<'__s, $crate::Value>,
    )
  };
}

#[repr(transparent)]
pub(crate) struct RawAccessorGetterCallback(
  for<'s> extern "C" fn(Local<'s, String>, *const PropertyCallbackInfo),
);

impl<F: AccessorGetterCallback> From<F> for RawAccessorGetterCallback {
  fn from(_: F) -> Self {
    #[inline(always)]
    extern "C" fn adapter<'s, F: AccessorGetterCallback>(
      property: Local<'s, String>,
      info: *const PropertyCallbackInfo,
    ) {
      let scope = &mut unsafe { CallbackScope::new(&*info) };
      let arguments =
        PropertyCallbackArguments::from_property_callback_info(info);
      let return_value =
        ReturnValue::<Value>::from_property_callback_info(info);
      (F::get())(scope, property, arguments, return_value)
    }

    Self(adapter::<F>)
  }
}

#[cfg(test)]
fn mock_accessor_getter_callback<'s>(
  _scope: &mut HandleScope<'s>,
  _property: Local<'s, String>,
  _arguments: PropertyCallbackArguments<'s>,
  _return_value: ReturnValue<'s, Value>,
) {
  unimplemented!()
}

#[test]
fn accessor_getter_callback_as_type_param() {
  fn pass_as_type_param<F: AccessorGetterCallback>(
    _: F,
  ) -> RawAccessorGetterCallback {
    F::get().into()
  }
  let _ = pass_as_type_param(mock_accessor_getter_callback);
}

#[test]
fn accessor_getter_callback_as_impl_trait() {
  fn pass_as_impl_trait(
    f: impl AccessorGetterCallback,
  ) -> RawAccessorGetterCallback {
    f.into()
  }
  let _ = pass_as_impl_trait(mock_accessor_getter_callback);
}

#[test]
fn accessor_getter_callback_as_impl_macro() {
  fn pass_as_impl_macro(
    f: impl_accessor_getter_callback!(),
  ) -> RawAccessorGetterCallback {
    f.into()
  }
  let _ = pass_as_impl_macro(mock_accessor_getter_callback);
  let _ = pass_as_impl_macro(
    |_scope, _property, _arguments, _return_value| unimplemented!(),
  );
}

// === AccessorNameGetterCallback ===

/// # Example
///
/// ```
/// # use rusty_v8 as v8;
/// #
/// fn accessor_name_getter_callback_example<'s>(
///   scope: &mut v8::HandleScope<'s>,
///   property: v8::Local<'s, v8::Name>,
///   arguments: v8::PropertyCallbackArguments<'s>,
///   return_value: v8::ReturnValue<'s, v8::Value>,
/// ) {
///   todo!();
/// }
/// ```
pub trait AccessorNameGetterCallback:
  UnitType
  + for<'s> FnOnce(
    &mut HandleScope<'s>,
    Local<'s, Name>,
    PropertyCallbackArguments<'s>,
    ReturnValue<'s, Value>,
  )
{
}

impl<F> AccessorNameGetterCallback for F where
  F: UnitType
    + for<'s> FnOnce(
      &mut HandleScope<'s>,
      Local<'s, Name>,
      PropertyCallbackArguments<'s>,
      ReturnValue<'s, Value>,
    )
{
}

#[macro_export]
macro_rules! impl_accessor_name_getter_callback {
  () => {
    impl $crate::AccessorNameGetterCallback
    + for<'__s> ::std::ops::FnOnce(
      &mut $crate::HandleScope<'__s>,
      $crate::Local<'__s, $crate::Name>,
      $crate::PropertyCallbackArguments<'__s>,
      $crate::ReturnValue<'__s, $crate::Value>,
    )
  };
}

#[repr(transparent)]
pub(crate) struct RawAccessorNameGetterCallback(
  for<'s> extern "C" fn(Local<'s, Name>, *const PropertyCallbackInfo),
);

impl<F: AccessorNameGetterCallback> From<F> for RawAccessorNameGetterCallback {
  fn from(_: F) -> Self {
    #[inline(always)]
    extern "C" fn adapter<'s, F: AccessorNameGetterCallback>(
      property: Local<'s, Name>,
      info: *const PropertyCallbackInfo,
    ) {
      let scope = &mut unsafe { CallbackScope::new(&*info) };
      let arguments =
        PropertyCallbackArguments::from_property_callback_info(info);
      let return_value =
        ReturnValue::<Value>::from_property_callback_info(info);
      (F::get())(scope, property, arguments, return_value)
    }

    Self(adapter::<F>)
  }
}

#[cfg(test)]
fn mock_accessor_name_getter_callback<'s>(
  _scope: &mut HandleScope<'s>,
  _property: Local<'s, Name>,
  _arguments: PropertyCallbackArguments<'s>,
  _return_value: ReturnValue<'s, Value>,
) {
  unimplemented!()
}

#[test]
fn accessor_name_getter_callback_as_type_param() {
  fn pass_as_type_param<F: AccessorNameGetterCallback>(
    _: F,
  ) -> RawAccessorNameGetterCallback {
    F::get().into()
  }
  let _ = pass_as_type_param(mock_accessor_name_getter_callback);
}

#[test]
fn accessor_name_getter_callback_as_impl_trait() {
  fn pass_as_impl_trait(
    f: impl AccessorNameGetterCallback,
  ) -> RawAccessorNameGetterCallback {
    f.into()
  }
  let _ = pass_as_impl_trait(mock_accessor_name_getter_callback);
}

#[test]
fn accessor_name_getter_callback_as_impl_macro() {
  fn pass_as_impl_macro(
    f: impl_accessor_name_getter_callback!(),
  ) -> RawAccessorNameGetterCallback {
    f.into()
  }
  let _ = pass_as_impl_macro(mock_accessor_name_getter_callback);
  let _ = pass_as_impl_macro(
    |_scope, _property, _arguments, _return_value| unimplemented!(),
  );
}

// === AccessorSetterCallback ===

/// # Example
///
/// ```
/// # use rusty_v8 as v8;
/// #
/// fn accessor_setter_callback_example<'s>(
///   scope: &mut v8::HandleScope<'s>,
///   property: v8::Local<'s, v8::String>,
///   value: v8::Local<'s, v8::Value>,
///   arguments: v8::PropertyCallbackArguments<'s>,
/// ) {
///   todo!();
/// }
/// ```
pub trait AccessorSetterCallback:
  UnitType
  + for<'s> FnOnce(
    &mut HandleScope<'s>,
    Local<'s, String>,
    Local<'s, Value>,
    PropertyCallbackArguments<'s>,
  )
{
}

impl<F> AccessorSetterCallback for F where
  F: UnitType
    + for<'s> FnOnce(
      &mut HandleScope<'s>,
      Local<'s, String>,
      Local<'s, Value>,
      PropertyCallbackArguments<'s>,
    )
{
}

#[macro_export]
macro_rules! impl_accessor_setter_callback {
  () => {
    impl $crate::AccessorSetterCallback
    + for<'__s> ::std::ops::FnOnce(
      &mut $crate::HandleScope<'__s>,
      $crate::Local<'__s, $crate::String>,
      $crate::Local<'__s, $crate::Value>,
      $crate::PropertyCallbackArguments<'__s>,
    )
  };
}

#[repr(transparent)]
pub(crate) struct RawAccessorSetterCallback(
  for<'s> extern "C" fn(
    Local<'s, String>,
    Local<'s, Value>,
    *const PropertyCallbackInfo,
  ),
);

impl<F: AccessorSetterCallback> From<F> for RawAccessorSetterCallback {
  fn from(_: F) -> Self {
    #[inline(always)]
    extern "C" fn adapter<'s, F: AccessorSetterCallback>(
      property: Local<'s, String>,
      value: Local<'s, Value>,
      info: *const PropertyCallbackInfo,
    ) {
      let scope = &mut unsafe { CallbackScope::new(&*info) };
      let arguments =
        PropertyCallbackArguments::from_property_callback_info(info);
      (F::get())(scope, property, value, arguments)
    }

    Self(adapter::<F>)
  }
}

#[cfg(test)]
fn mock_accessor_setter_callback<'s>(
  _scope: &mut HandleScope<'s>,
  _property: Local<'s, String>,
  _value: Local<'s, Value>,
  _arguments: PropertyCallbackArguments<'s>,
) {
  unimplemented!()
}

#[test]
fn accessor_setter_callback_as_type_param() {
  fn pass_as_type_param<F: AccessorSetterCallback>(
    _: F,
  ) -> RawAccessorSetterCallback {
    F::get().into()
  }
  let _ = pass_as_type_param(mock_accessor_setter_callback);
}

#[test]
fn accessor_setter_callback_as_impl_trait() {
  fn pass_as_impl_trait(
    f: impl AccessorSetterCallback,
  ) -> RawAccessorSetterCallback {
    f.into()
  }
  let _ = pass_as_impl_trait(mock_accessor_setter_callback);
}

#[test]
fn accessor_setter_callback_as_impl_macro() {
  fn pass_as_impl_macro(
    f: impl_accessor_setter_callback!(),
  ) -> RawAccessorSetterCallback {
    f.into()
  }
  let _ = pass_as_impl_macro(mock_accessor_setter_callback);
  let _ = pass_as_impl_macro(
    |_scope, _property, _value, _arguments| unimplemented!(),
  );
}

// === AccessorNameSetterCallback ===

/// # Example
///
/// ```
/// # use rusty_v8 as v8;
/// #
/// fn accessor_name_setter_callback_example<'s>(
///   scope: &mut v8::HandleScope<'s>,
///   property: v8::Local<'s, v8::Name>,
///   value: v8::Local<'s, v8::Value>,
///   arguments: v8::PropertyCallbackArguments<'s>,
/// ) {
///   todo!();
/// }
/// ```
pub trait AccessorNameSetterCallback:
  UnitType
  + for<'s> FnOnce(
    &mut HandleScope<'s>,
    Local<'s, Name>,
    Local<'s, Value>,
    PropertyCallbackArguments<'s>,
  )
{
}

impl<F> AccessorNameSetterCallback for F where
  F: UnitType
    + for<'s> FnOnce(
      &mut HandleScope<'s>,
      Local<'s, Name>,
      Local<'s, Value>,
      PropertyCallbackArguments<'s>,
    )
{
}

#[macro_export]
macro_rules! impl_accessor_name_setter_callback {
  () => {
    impl $crate::AccessorNameSetterCallback
    + for<'__s> ::std::ops::FnOnce(
      &mut $crate::HandleScope<'__s>,
      $crate::Local<'__s, $crate::Name>,
      $crate::Local<'__s, $crate::Value>,
      $crate::PropertyCallbackArguments<'__s>,
    )
  };
}

#[repr(transparent)]
pub(crate) struct RawAccessorNameSetterCallback(
  for<'s> extern "C" fn(
    Local<'s, Name>,
    Local<'s, Value>,
    *const PropertyCallbackInfo,
  ),
);

impl<F: AccessorNameSetterCallback> From<F> for RawAccessorNameSetterCallback {
  fn from(_: F) -> Self {
    #[inline(always)]
    extern "C" fn adapter<'s, F: AccessorNameSetterCallback>(
      property: Local<'s, Name>,
      value: Local<'s, Value>,
      info: *const PropertyCallbackInfo,
    ) {
      let scope = &mut unsafe { CallbackScope::new(&*info) };
      let arguments =
        PropertyCallbackArguments::from_property_callback_info(info);
      (F::get())(scope, property, value, arguments)
    }

    Self(adapter::<F>)
  }
}

#[cfg(test)]
fn mock_accessor_name_setter_callback<'s>(
  _scope: &mut HandleScope<'s>,
  _property: Local<'s, Name>,
  _value: Local<'s, Value>,
  _arguments: PropertyCallbackArguments<'s>,
) {
  unimplemented!()
}

#[test]
fn accessor_name_setter_callback_as_type_param() {
  fn pass_as_type_param<F: AccessorNameSetterCallback>(
    _: F,
  ) -> RawAccessorNameSetterCallback {
    F::get().into()
  }
  let _ = pass_as_type_param(mock_accessor_name_setter_callback);
}

#[test]
fn accessor_name_setter_callback_as_impl_trait() {
  fn pass_as_impl_trait(
    f: impl AccessorNameSetterCallback,
  ) -> RawAccessorNameSetterCallback {
    f.into()
  }
  let _ = pass_as_impl_trait(mock_accessor_name_setter_callback);
}

#[test]
fn accessor_name_setter_callback_as_impl_macro() {
  fn pass_as_impl_macro(
    f: impl_accessor_name_setter_callback!(),
  ) -> RawAccessorNameSetterCallback {
    f.into()
  }
  let _ = pass_as_impl_macro(mock_accessor_name_setter_callback);
  let _ = pass_as_impl_macro(
    |_scope, _property, _value, _arguments| unimplemented!(),
  );
}

// === FunctionCallback ===

/// # Example
///
/// ```
/// # use rusty_v8 as v8;
/// #
/// fn function_callback_example<'s>(
///   scope: &mut v8::HandleScope<'s>,
///   arguments: v8::FunctionCallbackArguments<'s>,
///   return_value: v8::ReturnValue<'s, v8::Value>,
/// ) {
///   todo!();
/// }
/// ```
pub trait FunctionCallback:
  UnitType
  + for<'s> FnOnce(
    &mut HandleScope<'s>,
    FunctionCallbackArguments<'s>,
    ReturnValue<'s, Value>,
  )
{
}

impl<F> FunctionCallback for F where
  F: UnitType
    + for<'s> FnOnce(
      &mut HandleScope<'s>,
      FunctionCallbackArguments<'s>,
      ReturnValue<'s, Value>,
    )
{
}

#[macro_export]
macro_rules! impl_function_callback {
  () => {
    impl $crate::FunctionCallback
    + for<'__s> ::std::ops::FnOnce(
      &mut $crate::HandleScope<'__s>,
      $crate::FunctionCallbackArguments<'__s>,
      $crate::ReturnValue<'__s, $crate::Value>,
    )
  };
}

#[repr(transparent)]
pub(crate) struct RawFunctionCallback(
  extern "C" fn(*const FunctionCallbackInfo),
);

impl<F: FunctionCallback> From<F> for RawFunctionCallback {
  fn from(_: F) -> Self {
    #[inline(always)]
    extern "C" fn adapter<F: FunctionCallback>(
      info: *const FunctionCallbackInfo,
    ) {
      let scope = &mut unsafe { CallbackScope::new(&*info) };
      let arguments =
        FunctionCallbackArguments::from_function_callback_info(info);
      let return_value =
        ReturnValue::<Value>::from_function_callback_info(info);
      (F::get())(scope, arguments, return_value)
    }

    Self(adapter::<F>)
  }
}

#[cfg(test)]
fn mock_function_callback<'s>(
  _scope: &mut HandleScope<'s>,
  _arguments: FunctionCallbackArguments<'s>,
  _return_value: ReturnValue<'s, Value>,
) {
  unimplemented!()
}

#[test]
fn function_callback_as_type_param() {
  fn pass_as_type_param<F: FunctionCallback>(_: F) -> RawFunctionCallback {
    F::get().into()
  }
  let _ = pass_as_type_param(mock_function_callback);
}

#[test]
fn function_callback_as_impl_trait() {
  fn pass_as_impl_trait(f: impl FunctionCallback) -> RawFunctionCallback {
    f.into()
  }
  let _ = pass_as_impl_trait(mock_function_callback);
}

#[test]
fn function_callback_as_impl_macro() {
  fn pass_as_impl_macro(f: impl_function_callback!()) -> RawFunctionCallback {
    f.into()
  }
  let _ = pass_as_impl_macro(mock_function_callback);
  let _ =
    pass_as_impl_macro(|_scope, _arguments, _return_value| unimplemented!());
}

/// This callback is used only if the memory block for a BackingStore cannot be
/// allocated with an ArrayBuffer::Allocator. In such cases the destructor of
/// the BackingStore invokes the callback to free the memory block.
///
/// # Example
///
/// ```
/// fn backing_store_deleter_callback_example(
///   data: &mut [u8],
///   deleter_data: *mut (),
/// ) {
///   todo!();
/// }
/// ```
pub trait BackingStoreDeleterCallback:
  UnitType + FnOnce(&mut [u8], *mut ())
{
}

impl<F> BackingStoreDeleterCallback for F where
  F: UnitType + FnOnce(&mut [u8], *mut ())
{
}

#[macro_export]
macro_rules! impl_backing_store_deleter_callback {
  () => {
    impl $crate::BackingStoreDeleterCallback
    + ::std::ops::FnOnce(
      &mut [u8],
      *mut (),
    )
  };
}

#[repr(transparent)]
pub(crate) struct RawBackingStoreDeleterCallback(
  extern "C" fn(*mut c_void, usize, *mut c_void),
);

impl<F: BackingStoreDeleterCallback> From<F>
  for RawBackingStoreDeleterCallback
{
  fn from(_: F) -> Self {
    #[inline(always)]
    extern "C" fn adapter<F: BackingStoreDeleterCallback>(
      data: *mut c_void,
      length: usize,
      deleter_data: *mut c_void,
    ) {
      let data = unsafe { slice::from_raw_parts_mut(data as *mut u8, length) };
      let deleter_data = deleter_data as *mut ();
      (F::get())(data, deleter_data)
    }

    Self(adapter::<F>)
  }
}

#[cfg(test)]
fn mock_backing_store_deleter_callback(
  _data: &mut [u8],
  _deleter_data: *mut (),
) {
  unimplemented!()
}

#[test]
fn backing_store_deleter_callback_as_type_param() {
  fn pass_as_type_param<F: BackingStoreDeleterCallback>(
    _: F,
  ) -> RawBackingStoreDeleterCallback {
    F::get().into()
  }
  let _ = pass_as_type_param(mock_backing_store_deleter_callback);
}

#[test]
fn backing_store_deleter_callback_as_impl_trait() {
  fn pass_as_impl_trait(
    f: impl BackingStoreDeleterCallback,
  ) -> RawBackingStoreDeleterCallback {
    f.into()
  }
  let _ = pass_as_impl_trait(mock_backing_store_deleter_callback);
}

#[test]
fn backing_store_deleter_callback_as_impl_macro() {
  fn pass_as_impl_macro(
    f: impl_backing_store_deleter_callback!(),
  ) -> RawBackingStoreDeleterCallback {
    f.into()
  }
  let _ = pass_as_impl_macro(mock_backing_store_deleter_callback);
  let _ = pass_as_impl_macro(|_data, _deleter_data| unimplemented!());
}

// === ArrayBufferContentsDeleterCallback ===

/// # Example
///
/// ```
/// fn array_buffer_contents_deleter_callback_example(
///   buffer: &mut [u8],
///   info: *mut (),
/// ) {
///   todo!();
/// }
/// ```
pub trait ArrayBufferContentsDeleterCallback:
  UnitType + FnOnce(&mut [u8], *mut ())
{
}

impl<F> ArrayBufferContentsDeleterCallback for F where
  F: UnitType + FnOnce(&mut [u8], *mut ())
{
}

#[macro_export]
macro_rules! impl_array_buffer_contents_deleter_callback {
  () => {
    impl $crate::ArrayBufferContentsDeleterCallback
    + ::std::ops::FnOnce(
      &mut [u8],
      *mut (),
    )
  };
}

#[repr(transparent)]
pub(crate) struct RawArrayBufferContentsDeleterCallback(
  extern "C" fn(*mut c_void, usize, *mut c_void),
);

impl<F: ArrayBufferContentsDeleterCallback> From<F>
  for RawArrayBufferContentsDeleterCallback
{
  fn from(_: F) -> Self {
    #[inline(always)]
    extern "C" fn adapter<F: ArrayBufferContentsDeleterCallback>(
      buffer: *mut c_void,
      length: usize,
      info: *mut c_void,
    ) {
      let buffer =
        unsafe { slice::from_raw_parts_mut(buffer as *mut u8, length) };
      let info = info as *mut ();
      (F::get())(buffer, info)
    }

    Self(adapter::<F>)
  }
}

#[cfg(test)]
fn mock_array_buffer_contents_deleter_callback(
  _buffer: &mut [u8],
  _info: *mut (),
) {
  unimplemented!()
}

#[test]
fn array_buffer_contents_deleter_callback_as_type_param() {
  fn pass_as_type_param<F: ArrayBufferContentsDeleterCallback>(
    _: F,
  ) -> RawArrayBufferContentsDeleterCallback {
    F::get().into()
  }
  let _ = pass_as_type_param(mock_array_buffer_contents_deleter_callback);
}

#[test]
fn array_buffer_contents_deleter_callback_as_impl_trait() {
  fn pass_as_impl_trait(
    f: impl ArrayBufferContentsDeleterCallback,
  ) -> RawArrayBufferContentsDeleterCallback {
    f.into()
  }
  let _ = pass_as_impl_trait(mock_array_buffer_contents_deleter_callback);
}

#[test]
fn array_buffer_contents_deleter_callback_as_impl_macro() {
  fn pass_as_impl_macro(
    f: impl_array_buffer_contents_deleter_callback!(),
  ) -> RawArrayBufferContentsDeleterCallback {
    f.into()
  }
  let _ = pass_as_impl_macro(mock_array_buffer_contents_deleter_callback);
  let _ = pass_as_impl_macro(|_buffer, _info| unimplemented!());
}

// === SharedArrayBufferContentsDeleterCallback ===

/// # Example
///
/// ```
/// fn shared_array_buffer_contents_deleter_callback_example(
///   buffer: &mut [u8],
///   info: *mut (),
/// ) {
///   todo!();
/// }
/// ```
pub trait SharedArrayBufferContentsDeleterCallback:
  UnitType + FnOnce(&mut [u8], *mut ())
{
}

impl<F> SharedArrayBufferContentsDeleterCallback for F where
  F: UnitType + FnOnce(&mut [u8], *mut ())
{
}

#[macro_export]
macro_rules! impl_shared_array_buffer_contents_deleter_callback {
  () => {
    impl $crate::SharedArrayBufferContentsDeleterCallback
    + ::std::ops::FnOnce(
      &mut [u8],
      *mut (),
    )
  };
}

#[repr(transparent)]
pub(crate) struct RawSharedArrayBufferContentsDeleterCallback(
  extern "C" fn(*mut c_void, usize, *mut c_void),
);

impl<F: SharedArrayBufferContentsDeleterCallback> From<F>
  for RawSharedArrayBufferContentsDeleterCallback
{
  fn from(_: F) -> Self {
    #[inline(always)]
    extern "C" fn adapter<F: SharedArrayBufferContentsDeleterCallback>(
      buffer: *mut c_void,
      length: usize,
      info: *mut c_void,
    ) {
      let buffer =
        unsafe { slice::from_raw_parts_mut(buffer as *mut u8, length) };
      let info = info as *mut ();
      (F::get())(buffer, info)
    }

    Self(adapter::<F>)
  }
}

#[cfg(test)]
fn mock_shared_array_buffer_contents_deleter_callback(
  _buffer: &mut [u8],
  _info: *mut (),
) {
  unimplemented!()
}

#[test]
fn shared_array_buffer_contents_deleter_callback_as_type_param() {
  fn pass_as_type_param<F: SharedArrayBufferContentsDeleterCallback>(
    _: F,
  ) -> RawSharedArrayBufferContentsDeleterCallback {
    F::get().into()
  }
  let _ =
    pass_as_type_param(mock_shared_array_buffer_contents_deleter_callback);
}

#[test]
fn shared_array_buffer_contents_deleter_callback_as_impl_trait() {
  fn pass_as_impl_trait(
    f: impl SharedArrayBufferContentsDeleterCallback,
  ) -> RawSharedArrayBufferContentsDeleterCallback {
    f.into()
  }
  let _ =
    pass_as_impl_trait(mock_shared_array_buffer_contents_deleter_callback);
}

#[test]
fn shared_array_buffer_contents_deleter_callback_as_impl_macro() {
  fn pass_as_impl_macro(
    f: impl_shared_array_buffer_contents_deleter_callback!(),
  ) -> RawSharedArrayBufferContentsDeleterCallback {
    f.into()
  }
  let _ =
    pass_as_impl_macro(mock_shared_array_buffer_contents_deleter_callback);
  let _ = pass_as_impl_macro(|_buffer, _info| unimplemented!());
}

/// Interceptor for get requests on an object.
///
/// Use `info.GetReturnValue().Set()` to set the return value of the
/// intercepted get request.
///
/// \param property The name of the property for which the request was
/// intercepted.
/// \param info Information about the intercepted request, such as
/// isolate, receiver, return value, or whether running in `'use strict`' mode.
/// See `PropertyCallbackInfo`.
///
/// ```ignore
///  void GetterCallback(
///    Local<Name> name,
///    const v8::PropertyCallbackInfo<v8::Value>& info) {
///      info.GetReturnValue().Set(v8_num(42));
///  }
///
///  v8::Local<v8::FunctionTemplate> templ =
///      v8::FunctionTemplate::New(isolate);
///  templ->InstanceTemplate()->SetHandler(
///      v8::NamedPropertyHandlerConfiguration(GetterCallback));
///  LocalContext env;
///  env->Global()
///      ->Set(env.local(), v8_str("obj"), templ->GetFunction(env.local())
///                                             .ToLocalChecked()
///                                             ->NewInstance(env.local())
///                                             .ToLocalChecked())
///      .FromJust();
///  v8::Local<v8::Value> result = CompileRun("obj.a = 17; obj.a");
///  CHECK(v8_num(42)->Equals(env.local(), result).FromJust());
/// ```
///
/// See also `ObjectTemplate::SetHandler`.
///
/// # Example
///
/// ```
/// # use rusty_v8 as v8;
/// #
/// fn generic_named_property_getter_callback_example<'s>(
///   scope: &mut v8::HandleScope<'s>,
///   property: v8::Local<'s, v8::Name>,
///   arguments: v8::PropertyCallbackArguments<'s>,
///   return_value: v8::ReturnValue<'s, v8::Value>,
/// ) {
///   todo!();
/// }
/// ```
pub trait GenericNamedPropertyGetterCallback:
  UnitType
  + for<'s> FnOnce(
    &mut HandleScope<'s>,
    Local<'s, Name>,
    PropertyCallbackArguments<'s>,
    ReturnValue<'s, Value>,
  )
{
}

impl<F> GenericNamedPropertyGetterCallback for F where
  F: UnitType
    + for<'s> FnOnce(
      &mut HandleScope<'s>,
      Local<'s, Name>,
      PropertyCallbackArguments<'s>,
      ReturnValue<'s, Value>,
    )
{
}

#[macro_export]
macro_rules! impl_generic_named_property_getter_callback {
  () => {
    impl $crate::GenericNamedPropertyGetterCallback
    + for<'__s> ::std::ops::FnOnce(
      &mut $crate::HandleScope<'__s>,
      $crate::Local<'__s, $crate::Name>,
      $crate::PropertyCallbackArguments<'__s>,
      $crate::ReturnValue<'__s, $crate::Value>,
    )
  };
}

#[repr(transparent)]
pub(crate) struct RawGenericNamedPropertyGetterCallback(
  for<'s> extern "C" fn(Local<'s, Name>, *const PropertyCallbackInfo),
);

impl<F: GenericNamedPropertyGetterCallback> From<F>
  for RawGenericNamedPropertyGetterCallback
{
  fn from(_: F) -> Self {
    #[inline(always)]
    extern "C" fn adapter<'s, F: GenericNamedPropertyGetterCallback>(
      property: Local<'s, Name>,
      info: *const PropertyCallbackInfo,
    ) {
      let scope = &mut unsafe { CallbackScope::new(&*info) };
      let arguments =
        PropertyCallbackArguments::from_property_callback_info(info);
      let return_value =
        ReturnValue::<Value>::from_property_callback_info(info);
      (F::get())(scope, property, arguments, return_value)
    }

    Self(adapter::<F>)
  }
}

#[cfg(test)]
fn mock_generic_named_property_getter_callback<'s>(
  _scope: &mut HandleScope<'s>,
  _property: Local<'s, Name>,
  _arguments: PropertyCallbackArguments<'s>,
  _return_value: ReturnValue<'s, Value>,
) {
  unimplemented!()
}

#[test]
fn generic_named_property_getter_callback_as_type_param() {
  fn pass_as_type_param<F: GenericNamedPropertyGetterCallback>(
    _: F,
  ) -> RawGenericNamedPropertyGetterCallback {
    F::get().into()
  }
  let _ = pass_as_type_param(mock_generic_named_property_getter_callback);
}

#[test]
fn generic_named_property_getter_callback_as_impl_trait() {
  fn pass_as_impl_trait(
    f: impl GenericNamedPropertyGetterCallback,
  ) -> RawGenericNamedPropertyGetterCallback {
    f.into()
  }
  let _ = pass_as_impl_trait(mock_generic_named_property_getter_callback);
}

#[test]
fn generic_named_property_getter_callback_as_impl_macro() {
  fn pass_as_impl_macro(
    f: impl_generic_named_property_getter_callback!(),
  ) -> RawGenericNamedPropertyGetterCallback {
    f.into()
  }
  let _ = pass_as_impl_macro(mock_generic_named_property_getter_callback);
  let _ = pass_as_impl_macro(
    |_scope, _property, _arguments, _return_value| unimplemented!(),
  );
}

/// Interceptor for set requests on an object.
///
/// Use `info.GetReturnValue()` to indicate whether the request was intercepted
/// or not. If the setter successfully intercepts the request, i.e., if the
/// request should not be further executed, call
/// `info.GetReturnValue().Set(value)`. If the setter
/// did not intercept the request, i.e., if the request should be handled as
/// if no interceptor is present, do not not call `Set()`.
///
/// \param property The name of the property for which the request was
/// intercepted.
/// \param value The value which the property will have if the request
/// is not intercepted.
/// \param info Information about the intercepted request, such as
/// isolate, receiver, return value, or whether running in `'use strict'` mode.
/// See `PropertyCallbackInfo`.
///
/// See also
/// `ObjectTemplate::SetHandler.`
///
/// # Example
///
/// ```
/// # use rusty_v8 as v8;
/// #
/// fn generic_named_property_setter_callback_example<'s>(
///   scope: &mut v8::HandleScope<'s>,
///   property: v8::Local<'s, v8::Name>,
///   value: v8::Local<'s, v8::Value>,
///   arguments: v8::PropertyCallbackArguments<'s>,
///   return_value: v8::ReturnValue<'s, v8::Value>,
/// ) {
///   todo!();
/// }
/// ```
pub trait GenericNamedPropertySetterCallback:
  UnitType
  + for<'s> FnOnce(
    &mut HandleScope<'s>,
    Local<'s, Name>,
    Local<'s, Value>,
    PropertyCallbackArguments<'s>,
    ReturnValue<'s, Value>,
  )
{
}

impl<F> GenericNamedPropertySetterCallback for F where
  F: UnitType
    + for<'s> FnOnce(
      &mut HandleScope<'s>,
      Local<'s, Name>,
      Local<'s, Value>,
      PropertyCallbackArguments<'s>,
      ReturnValue<'s, Value>,
    )
{
}

#[macro_export]
macro_rules! impl_generic_named_property_setter_callback {
  () => {
    impl $crate::GenericNamedPropertySetterCallback
    + for<'__s> ::std::ops::FnOnce(
      &mut $crate::HandleScope<'__s>,
      $crate::Local<'__s, $crate::Name>,
      $crate::Local<'__s, $crate::Value>,
      $crate::PropertyCallbackArguments<'__s>,
      $crate::ReturnValue<'__s, $crate::Value>,
    )
  };
}

#[repr(transparent)]
pub(crate) struct RawGenericNamedPropertySetterCallback(
  for<'s> extern "C" fn(
    Local<'s, Name>,
    Local<'s, Value>,
    *const PropertyCallbackInfo,
  ),
);

impl<F: GenericNamedPropertySetterCallback> From<F>
  for RawGenericNamedPropertySetterCallback
{
  fn from(_: F) -> Self {
    #[inline(always)]
    extern "C" fn adapter<'s, F: GenericNamedPropertySetterCallback>(
      property: Local<'s, Name>,
      value: Local<'s, Value>,
      info: *const PropertyCallbackInfo,
    ) {
      let scope = &mut unsafe { CallbackScope::new(&*info) };
      let arguments =
        PropertyCallbackArguments::from_property_callback_info(info);
      let return_value =
        ReturnValue::<Value>::from_property_callback_info(info);
      (F::get())(scope, property, value, arguments, return_value)
    }

    Self(adapter::<F>)
  }
}

#[cfg(test)]
fn mock_generic_named_property_setter_callback<'s>(
  _scope: &mut HandleScope<'s>,
  _property: Local<'s, Name>,
  _value: Local<'s, Value>,
  _arguments: PropertyCallbackArguments<'s>,
  _return_value: ReturnValue<'s, Value>,
) {
  unimplemented!()
}

#[test]
fn generic_named_property_setter_callback_as_type_param() {
  fn pass_as_type_param<F: GenericNamedPropertySetterCallback>(
    _: F,
  ) -> RawGenericNamedPropertySetterCallback {
    F::get().into()
  }
  let _ = pass_as_type_param(mock_generic_named_property_setter_callback);
}

#[test]
fn generic_named_property_setter_callback_as_impl_trait() {
  fn pass_as_impl_trait(
    f: impl GenericNamedPropertySetterCallback,
  ) -> RawGenericNamedPropertySetterCallback {
    f.into()
  }
  let _ = pass_as_impl_trait(mock_generic_named_property_setter_callback);
}

#[test]
fn generic_named_property_setter_callback_as_impl_macro() {
  fn pass_as_impl_macro(
    f: impl_generic_named_property_setter_callback!(),
  ) -> RawGenericNamedPropertySetterCallback {
    f.into()
  }
  let _ = pass_as_impl_macro(mock_generic_named_property_setter_callback);
  let _ = pass_as_impl_macro(
    |_scope, _property, _value, _arguments, _return_value| unimplemented!(),
  );
}

/// Intercepts all requests that query the attributes of the
/// property, e.g., getOwnPropertyDescriptor(), propertyIsEnumerable(), and
/// defineProperty().
///
/// Use `info.GetReturnValue().Set(value)` to set the property attributes. The
/// value is an integer encoding a `v8::PropertyAttribute`.
///
/// \param property The name of the property for which the request was
/// intercepted.
/// \param info Information about the intercepted request, such as
/// isolate, receiver, return value, or whether running in `'use strict'` mode.
/// See `PropertyCallbackInfo`.
///
/// \note Some functions query the property attributes internally, even though
/// they do not return the attributes. For example, `hasOwnProperty()` can
/// trigger this interceptor depending on the state of the object.
///
/// See also
/// `ObjectTemplate::SetHandler.`
///
/// # Example
///
/// ```
/// # use rusty_v8 as v8;
/// #
/// fn generic_named_property_query_callback_example<'s>(
///   scope: &mut v8::HandleScope<'s>,
///   property: v8::Local<'s, v8::Name>,
///   arguments: v8::PropertyCallbackArguments<'s>,
///   return_value: v8::ReturnValue<'s, v8::Integer>,
/// ) {
///   todo!();
/// }
/// ```
pub trait GenericNamedPropertyQueryCallback:
  UnitType
  + for<'s> FnOnce(
    &mut HandleScope<'s>,
    Local<'s, Name>,
    PropertyCallbackArguments<'s>,
    ReturnValue<'s, Integer>,
  )
{
}

impl<F> GenericNamedPropertyQueryCallback for F where
  F: UnitType
    + for<'s> FnOnce(
      &mut HandleScope<'s>,
      Local<'s, Name>,
      PropertyCallbackArguments<'s>,
      ReturnValue<'s, Integer>,
    )
{
}

#[macro_export]
macro_rules! impl_generic_named_property_query_callback {
  () => {
    impl $crate::GenericNamedPropertyQueryCallback
    + for<'__s> ::std::ops::FnOnce(
      &mut $crate::HandleScope<'__s>,
      $crate::Local<'__s, $crate::Name>,
      $crate::PropertyCallbackArguments<'__s>,
      $crate::ReturnValue<'__s, $crate::Integer>,
    )
  };
}

#[repr(transparent)]
pub(crate) struct RawGenericNamedPropertyQueryCallback(
  for<'s> extern "C" fn(Local<'s, Name>, *const PropertyCallbackInfo),
);

impl<F: GenericNamedPropertyQueryCallback> From<F>
  for RawGenericNamedPropertyQueryCallback
{
  fn from(_: F) -> Self {
    #[inline(always)]
    extern "C" fn adapter<'s, F: GenericNamedPropertyQueryCallback>(
      property: Local<'s, Name>,
      info: *const PropertyCallbackInfo,
    ) {
      let scope = &mut unsafe { CallbackScope::new(&*info) };
      let arguments =
        PropertyCallbackArguments::from_property_callback_info(info);
      let return_value =
        ReturnValue::<Integer>::from_property_callback_info(info);
      (F::get())(scope, property, arguments, return_value)
    }

    Self(adapter::<F>)
  }
}

#[cfg(test)]
fn mock_generic_named_property_query_callback<'s>(
  _scope: &mut HandleScope<'s>,
  _property: Local<'s, Name>,
  _arguments: PropertyCallbackArguments<'s>,
  _return_value: ReturnValue<'s, Integer>,
) {
  unimplemented!()
}

#[test]
fn generic_named_property_query_callback_as_type_param() {
  fn pass_as_type_param<F: GenericNamedPropertyQueryCallback>(
    _: F,
  ) -> RawGenericNamedPropertyQueryCallback {
    F::get().into()
  }
  let _ = pass_as_type_param(mock_generic_named_property_query_callback);
}

#[test]
fn generic_named_property_query_callback_as_impl_trait() {
  fn pass_as_impl_trait(
    f: impl GenericNamedPropertyQueryCallback,
  ) -> RawGenericNamedPropertyQueryCallback {
    f.into()
  }
  let _ = pass_as_impl_trait(mock_generic_named_property_query_callback);
}

#[test]
fn generic_named_property_query_callback_as_impl_macro() {
  fn pass_as_impl_macro(
    f: impl_generic_named_property_query_callback!(),
  ) -> RawGenericNamedPropertyQueryCallback {
    f.into()
  }
  let _ = pass_as_impl_macro(mock_generic_named_property_query_callback);
  let _ = pass_as_impl_macro(
    |_scope, _property, _arguments, _return_value| unimplemented!(),
  );
}

/// Interceptor for delete requests on an object.
///
/// Use `info.GetReturnValue()` to indicate whether the request was intercepted
/// or not. If the deleter successfully intercepts the request, i.e., if the
/// request should not be further executed, call
/// `info.GetReturnValue().Set(value)` with a boolean `value`. The `value` is
/// used as the return value of `delete`.
///
/// \param property The name of the property for which the request was
/// intercepted.
/// \param info Information about the intercepted request, such as
/// isolate, receiver, return value, or whether running in `'use strict'` mode.
/// See `PropertyCallbackInfo`.
///
/// \note If you need to mimic the behavior of `delete`, i.e., throw in strict
/// mode instead of returning false, use `info.ShouldThrowOnError()` to
/// determine if you are in strict mode.
///
/// See also `ObjectTemplate::SetHandler.`
///
/// # Example
///
/// ```
/// # use rusty_v8 as v8;
/// #
/// fn generic_named_property_deleter_callback_example<'s>(
///   scope: &mut v8::HandleScope<'s>,
///   property: v8::Local<'s, v8::Name>,
///   arguments: v8::PropertyCallbackArguments<'s>,
///   return_value: v8::ReturnValue<'s, v8::Boolean>,
/// ) {
///   todo!();
/// }
/// ```
pub trait GenericNamedPropertyDeleterCallback:
  UnitType
  + for<'s> FnOnce(
    &mut HandleScope<'s>,
    Local<'s, Name>,
    PropertyCallbackArguments<'s>,
    ReturnValue<'s, Boolean>,
  )
{
}

impl<F> GenericNamedPropertyDeleterCallback for F where
  F: UnitType
    + for<'s> FnOnce(
      &mut HandleScope<'s>,
      Local<'s, Name>,
      PropertyCallbackArguments<'s>,
      ReturnValue<'s, Boolean>,
    )
{
}

#[macro_export]
macro_rules! impl_generic_named_property_deleter_callback {
  () => {
    impl $crate::GenericNamedPropertyDeleterCallback
    + for<'__s> ::std::ops::FnOnce(
      &mut $crate::HandleScope<'__s>,
      $crate::Local<'__s, $crate::Name>,
      $crate::PropertyCallbackArguments<'__s>,
      $crate::ReturnValue<'__s, $crate::Boolean>,
    )
  };
}

#[repr(transparent)]
pub(crate) struct RawGenericNamedPropertyDeleterCallback(
  for<'s> extern "C" fn(Local<'s, Name>, *const PropertyCallbackInfo),
);

impl<F: GenericNamedPropertyDeleterCallback> From<F>
  for RawGenericNamedPropertyDeleterCallback
{
  fn from(_: F) -> Self {
    #[inline(always)]
    extern "C" fn adapter<'s, F: GenericNamedPropertyDeleterCallback>(
      property: Local<'s, Name>,
      info: *const PropertyCallbackInfo,
    ) {
      let scope = &mut unsafe { CallbackScope::new(&*info) };
      let arguments =
        PropertyCallbackArguments::from_property_callback_info(info);
      let return_value =
        ReturnValue::<Boolean>::from_property_callback_info(info);
      (F::get())(scope, property, arguments, return_value)
    }

    Self(adapter::<F>)
  }
}

#[cfg(test)]
fn mock_generic_named_property_deleter_callback<'s>(
  _scope: &mut HandleScope<'s>,
  _property: Local<'s, Name>,
  _arguments: PropertyCallbackArguments<'s>,
  _return_value: ReturnValue<'s, Boolean>,
) {
  unimplemented!()
}

#[test]
fn generic_named_property_deleter_callback_as_type_param() {
  fn pass_as_type_param<F: GenericNamedPropertyDeleterCallback>(
    _: F,
  ) -> RawGenericNamedPropertyDeleterCallback {
    F::get().into()
  }
  let _ = pass_as_type_param(mock_generic_named_property_deleter_callback);
}

#[test]
fn generic_named_property_deleter_callback_as_impl_trait() {
  fn pass_as_impl_trait(
    f: impl GenericNamedPropertyDeleterCallback,
  ) -> RawGenericNamedPropertyDeleterCallback {
    f.into()
  }
  let _ = pass_as_impl_trait(mock_generic_named_property_deleter_callback);
}

#[test]
fn generic_named_property_deleter_callback_as_impl_macro() {
  fn pass_as_impl_macro(
    f: impl_generic_named_property_deleter_callback!(),
  ) -> RawGenericNamedPropertyDeleterCallback {
    f.into()
  }
  let _ = pass_as_impl_macro(mock_generic_named_property_deleter_callback);
  let _ = pass_as_impl_macro(
    |_scope, _property, _arguments, _return_value| unimplemented!(),
  );
}

/// Returns an array containing the names of the properties the named
/// property getter intercepts.
///
/// Note: The values in the array must be of type v8::Name.
///
/// # Example
///
/// ```
/// # use rusty_v8 as v8;
/// #
/// fn generic_named_property_enumerator_callback_example<'s>(
///   scope: &mut v8::HandleScope<'s>,
///   arguments: v8::PropertyCallbackArguments<'s>,
///   return_value: v8::ReturnValue<'s, v8::Array>,
/// ) {
///   todo!();
/// }
/// ```
pub trait GenericNamedPropertyEnumeratorCallback:
  UnitType
  + for<'s> FnOnce(
    &mut HandleScope<'s>,
    PropertyCallbackArguments<'s>,
    ReturnValue<'s, Array>,
  )
{
}

impl<F> GenericNamedPropertyEnumeratorCallback for F where
  F: UnitType
    + for<'s> FnOnce(
      &mut HandleScope<'s>,
      PropertyCallbackArguments<'s>,
      ReturnValue<'s, Array>,
    )
{
}

#[macro_export]
macro_rules! impl_generic_named_property_enumerator_callback {
  () => {
    impl $crate::GenericNamedPropertyEnumeratorCallback
    + for<'__s> ::std::ops::FnOnce(
      &mut $crate::HandleScope<'__s>,
      $crate::PropertyCallbackArguments<'__s>,
      $crate::ReturnValue<'__s, $crate::Array>,
    )
  };
}

#[repr(transparent)]
pub(crate) struct RawGenericNamedPropertyEnumeratorCallback(
  extern "C" fn(*const PropertyCallbackInfo),
);

impl<F: GenericNamedPropertyEnumeratorCallback> From<F>
  for RawGenericNamedPropertyEnumeratorCallback
{
  fn from(_: F) -> Self {
    #[inline(always)]
    extern "C" fn adapter<F: GenericNamedPropertyEnumeratorCallback>(
      info: *const PropertyCallbackInfo,
    ) {
      let scope = &mut unsafe { CallbackScope::new(&*info) };
      let arguments =
        PropertyCallbackArguments::from_property_callback_info(info);
      let return_value =
        ReturnValue::<Array>::from_property_callback_info(info);
      (F::get())(scope, arguments, return_value)
    }

    Self(adapter::<F>)
  }
}

#[cfg(test)]
fn mock_generic_named_property_enumerator_callback<'s>(
  _scope: &mut HandleScope<'s>,
  _arguments: PropertyCallbackArguments<'s>,
  _return_value: ReturnValue<'s, Array>,
) {
  unimplemented!()
}

#[test]
fn generic_named_property_enumerator_callback_as_type_param() {
  fn pass_as_type_param<F: GenericNamedPropertyEnumeratorCallback>(
    _: F,
  ) -> RawGenericNamedPropertyEnumeratorCallback {
    F::get().into()
  }
  let _ = pass_as_type_param(mock_generic_named_property_enumerator_callback);
}

#[test]
fn generic_named_property_enumerator_callback_as_impl_trait() {
  fn pass_as_impl_trait(
    f: impl GenericNamedPropertyEnumeratorCallback,
  ) -> RawGenericNamedPropertyEnumeratorCallback {
    f.into()
  }
  let _ = pass_as_impl_trait(mock_generic_named_property_enumerator_callback);
}

#[test]
fn generic_named_property_enumerator_callback_as_impl_macro() {
  fn pass_as_impl_macro(
    f: impl_generic_named_property_enumerator_callback!(),
  ) -> RawGenericNamedPropertyEnumeratorCallback {
    f.into()
  }
  let _ = pass_as_impl_macro(mock_generic_named_property_enumerator_callback);
  let _ =
    pass_as_impl_macro(|_scope, _arguments, _return_value| unimplemented!());
}

/// Interceptor for defineProperty requests on an object.
///
/// Use `info.GetReturnValue()` to indicate whether the request was intercepted
/// or not. If the definer successfully intercepts the request, i.e., if the
/// request should not be further executed, call
/// `info.GetReturnValue().Set(value)`. If the definer
/// did not intercept the request, i.e., if the request should be handled as
/// if no interceptor is present, do not not call `Set()`.
///
/// \param property The name of the property for which the request was
/// intercepted.
/// \param desc The property descriptor which is used to define the
/// property if the request is not intercepted.
/// \param info Information about the intercepted request, such as
/// isolate, receiver, return value, or whether running in `'use strict'` mode.
/// See `PropertyCallbackInfo`.
///
/// See also `ObjectTemplate::SetHandler`.
///
/// # Example
///
/// ```
/// # use rusty_v8 as v8;
/// #
/// fn generic_named_property_definer_callback_example<'s>(
///   scope: &mut v8::HandleScope<'s>,
///   property: v8::Local<'s, v8::Name>,
///   desc: &v8::PropertyDescriptor,
///   arguments: v8::PropertyCallbackArguments<'s>,
///   return_value: v8::ReturnValue<'s, v8::Value>,
/// ) {
///   todo!();
/// }
/// ```
pub trait GenericNamedPropertyDefinerCallback:
  UnitType
  + for<'s> FnOnce(
    &mut HandleScope<'s>,
    Local<'s, Name>,
    &PropertyDescriptor,
    PropertyCallbackArguments<'s>,
    ReturnValue<'s, Value>,
  )
{
}

impl<F> GenericNamedPropertyDefinerCallback for F where
  F: UnitType
    + for<'s> FnOnce(
      &mut HandleScope<'s>,
      Local<'s, Name>,
      &PropertyDescriptor,
      PropertyCallbackArguments<'s>,
      ReturnValue<'s, Value>,
    )
{
}

#[macro_export]
macro_rules! impl_generic_named_property_definer_callback {
  () => {
    impl $crate::GenericNamedPropertyDefinerCallback
    + for<'__s> ::std::ops::FnOnce(
      &mut $crate::HandleScope<'__s>,
      $crate::Local<'__s, $crate::Name>,
      &$crate::PropertyDescriptor,
      $crate::PropertyCallbackArguments<'__s>,
      $crate::ReturnValue<'__s, $crate::Value>,
    )
  };
}

#[repr(transparent)]
pub(crate) struct RawGenericNamedPropertyDefinerCallback(
  for<'s> extern "C" fn(
    Local<'s, Name>,
    *const PropertyDescriptor,
    *const PropertyCallbackInfo,
  ),
);

impl<F: GenericNamedPropertyDefinerCallback> From<F>
  for RawGenericNamedPropertyDefinerCallback
{
  fn from(_: F) -> Self {
    #[inline(always)]
    extern "C" fn adapter<'s, F: GenericNamedPropertyDefinerCallback>(
      property: Local<'s, Name>,
      desc: *const PropertyDescriptor,
      info: *const PropertyCallbackInfo,
    ) {
      let scope = &mut unsafe { CallbackScope::new(&*info) };
      let desc = unsafe { &*desc };
      let arguments =
        PropertyCallbackArguments::from_property_callback_info(info);
      let return_value =
        ReturnValue::<Value>::from_property_callback_info(info);
      (F::get())(scope, property, desc, arguments, return_value)
    }

    Self(adapter::<F>)
  }
}

#[cfg(test)]
fn mock_generic_named_property_definer_callback<'s>(
  _scope: &mut HandleScope<'s>,
  _property: Local<'s, Name>,
  _desc: &PropertyDescriptor,
  _arguments: PropertyCallbackArguments<'s>,
  _return_value: ReturnValue<'s, Value>,
) {
  unimplemented!()
}

#[test]
fn generic_named_property_definer_callback_as_type_param() {
  fn pass_as_type_param<F: GenericNamedPropertyDefinerCallback>(
    _: F,
  ) -> RawGenericNamedPropertyDefinerCallback {
    F::get().into()
  }
  let _ = pass_as_type_param(mock_generic_named_property_definer_callback);
}

#[test]
fn generic_named_property_definer_callback_as_impl_trait() {
  fn pass_as_impl_trait(
    f: impl GenericNamedPropertyDefinerCallback,
  ) -> RawGenericNamedPropertyDefinerCallback {
    f.into()
  }
  let _ = pass_as_impl_trait(mock_generic_named_property_definer_callback);
}

#[test]
fn generic_named_property_definer_callback_as_impl_macro() {
  fn pass_as_impl_macro(
    f: impl_generic_named_property_definer_callback!(),
  ) -> RawGenericNamedPropertyDefinerCallback {
    f.into()
  }
  let _ = pass_as_impl_macro(mock_generic_named_property_definer_callback);
  let _ = pass_as_impl_macro(
    |_scope, _property, _desc, _arguments, _return_value| unimplemented!(),
  );
}

/// Interceptor for getOwnPropertyDescriptor requests on an object.
///
/// Use `info.GetReturnValue().Set()` to set the return value of the
/// intercepted request. The return value must be an object that
/// can be converted to a PropertyDescriptor, e.g., a `v8::value` returned from
/// `v8::Object::getOwnPropertyDescriptor`.
///
/// \param property The name of the property for which the request was
/// intercepted.
/// \info Information about the intercepted request, such as
/// isolate, receiver, return value, or whether running in `'use strict'` mode.
/// See `PropertyCallbackInfo`.
///
/// \note If GetOwnPropertyDescriptor is intercepted, it will
/// always return true, i.e., indicate that the property was found.
///
/// See also `ObjectTemplate::SetHandler`.
///
/// # Example
///
/// ```
/// # use rusty_v8 as v8;
/// #
/// fn generic_named_property_descriptor_callback_example<'s>(
///   scope: &mut v8::HandleScope<'s>,
///   property: v8::Local<'s, v8::Name>,
///   arguments: v8::PropertyCallbackArguments<'s>,
///   return_value: v8::ReturnValue<'s, v8::Value>,
/// ) {
///   todo!();
/// }
/// ```
pub trait GenericNamedPropertyDescriptorCallback:
  UnitType
  + for<'s> FnOnce(
    &mut HandleScope<'s>,
    Local<'s, Name>,
    PropertyCallbackArguments<'s>,
    ReturnValue<'s, Value>,
  )
{
}

impl<F> GenericNamedPropertyDescriptorCallback for F where
  F: UnitType
    + for<'s> FnOnce(
      &mut HandleScope<'s>,
      Local<'s, Name>,
      PropertyCallbackArguments<'s>,
      ReturnValue<'s, Value>,
    )
{
}

#[macro_export]
macro_rules! impl_generic_named_property_descriptor_callback {
  () => {
    impl $crate::GenericNamedPropertyDescriptorCallback
    + for<'__s> ::std::ops::FnOnce(
      &mut $crate::HandleScope<'__s>,
      $crate::Local<'__s, $crate::Name>,
      $crate::PropertyCallbackArguments<'__s>,
      $crate::ReturnValue<'__s, $crate::Value>,
    )
  };
}

#[repr(transparent)]
pub(crate) struct RawGenericNamedPropertyDescriptorCallback(
  for<'s> extern "C" fn(Local<'s, Name>, *const PropertyCallbackInfo),
);

impl<F: GenericNamedPropertyDescriptorCallback> From<F>
  for RawGenericNamedPropertyDescriptorCallback
{
  fn from(_: F) -> Self {
    #[inline(always)]
    extern "C" fn adapter<'s, F: GenericNamedPropertyDescriptorCallback>(
      property: Local<'s, Name>,
      info: *const PropertyCallbackInfo,
    ) {
      let scope = &mut unsafe { CallbackScope::new(&*info) };
      let arguments =
        PropertyCallbackArguments::from_property_callback_info(info);
      let return_value =
        ReturnValue::<Value>::from_property_callback_info(info);
      (F::get())(scope, property, arguments, return_value)
    }

    Self(adapter::<F>)
  }
}

#[cfg(test)]
fn mock_generic_named_property_descriptor_callback<'s>(
  _scope: &mut HandleScope<'s>,
  _property: Local<'s, Name>,
  _arguments: PropertyCallbackArguments<'s>,
  _return_value: ReturnValue<'s, Value>,
) {
  unimplemented!()
}

#[test]
fn generic_named_property_descriptor_callback_as_type_param() {
  fn pass_as_type_param<F: GenericNamedPropertyDescriptorCallback>(
    _: F,
  ) -> RawGenericNamedPropertyDescriptorCallback {
    F::get().into()
  }
  let _ = pass_as_type_param(mock_generic_named_property_descriptor_callback);
}

#[test]
fn generic_named_property_descriptor_callback_as_impl_trait() {
  fn pass_as_impl_trait(
    f: impl GenericNamedPropertyDescriptorCallback,
  ) -> RawGenericNamedPropertyDescriptorCallback {
    f.into()
  }
  let _ = pass_as_impl_trait(mock_generic_named_property_descriptor_callback);
}

#[test]
fn generic_named_property_descriptor_callback_as_impl_macro() {
  fn pass_as_impl_macro(
    f: impl_generic_named_property_descriptor_callback!(),
  ) -> RawGenericNamedPropertyDescriptorCallback {
    f.into()
  }
  let _ = pass_as_impl_macro(mock_generic_named_property_descriptor_callback);
  let _ = pass_as_impl_macro(
    |_scope, _property, _arguments, _return_value| unimplemented!(),
  );
}

/// See `v8::GenericNamedPropertyGetterCallback`.
///
/// # Example
///
/// ```
/// # use rusty_v8 as v8;
/// #
/// fn indexed_property_getter_callback_example<'s>(
///   scope: &mut v8::HandleScope<'s>,
///   index: u32,
///   arguments: v8::PropertyCallbackArguments<'s>,
///   return_value: v8::ReturnValue<'s, v8::Value>,
/// ) {
///   todo!();
/// }
/// ```
pub trait IndexedPropertyGetterCallback:
  UnitType
  + for<'s> FnOnce(
    &mut HandleScope<'s>,
    u32,
    PropertyCallbackArguments<'s>,
    ReturnValue<'s, Value>,
  )
{
}

impl<F> IndexedPropertyGetterCallback for F where
  F: UnitType
    + for<'s> FnOnce(
      &mut HandleScope<'s>,
      u32,
      PropertyCallbackArguments<'s>,
      ReturnValue<'s, Value>,
    )
{
}

#[macro_export]
macro_rules! impl_indexed_property_getter_callback {
  () => {
    impl $crate::IndexedPropertyGetterCallback
    + for<'__s> ::std::ops::FnOnce(
      &mut $crate::HandleScope<'__s>,
      u32,
      $crate::PropertyCallbackArguments<'__s>,
      $crate::ReturnValue<'__s, $crate::Value>,
    )
  };
}

#[repr(transparent)]
pub(crate) struct RawIndexedPropertyGetterCallback(
  extern "C" fn(u32, *const PropertyCallbackInfo),
);

impl<F: IndexedPropertyGetterCallback> From<F>
  for RawIndexedPropertyGetterCallback
{
  fn from(_: F) -> Self {
    #[inline(always)]
    extern "C" fn adapter<F: IndexedPropertyGetterCallback>(
      index: u32,
      info: *const PropertyCallbackInfo,
    ) {
      let scope = &mut unsafe { CallbackScope::new(&*info) };
      let arguments =
        PropertyCallbackArguments::from_property_callback_info(info);
      let return_value =
        ReturnValue::<Value>::from_property_callback_info(info);
      (F::get())(scope, index, arguments, return_value)
    }

    Self(adapter::<F>)
  }
}

#[cfg(test)]
fn mock_indexed_property_getter_callback<'s>(
  _scope: &mut HandleScope<'s>,
  _index: u32,
  _arguments: PropertyCallbackArguments<'s>,
  _return_value: ReturnValue<'s, Value>,
) {
  unimplemented!()
}

#[test]
fn indexed_property_getter_callback_as_type_param() {
  fn pass_as_type_param<F: IndexedPropertyGetterCallback>(
    _: F,
  ) -> RawIndexedPropertyGetterCallback {
    F::get().into()
  }
  let _ = pass_as_type_param(mock_indexed_property_getter_callback);
}

#[test]
fn indexed_property_getter_callback_as_impl_trait() {
  fn pass_as_impl_trait(
    f: impl IndexedPropertyGetterCallback,
  ) -> RawIndexedPropertyGetterCallback {
    f.into()
  }
  let _ = pass_as_impl_trait(mock_indexed_property_getter_callback);
}

#[test]
fn indexed_property_getter_callback_as_impl_macro() {
  fn pass_as_impl_macro(
    f: impl_indexed_property_getter_callback!(),
  ) -> RawIndexedPropertyGetterCallback {
    f.into()
  }
  let _ = pass_as_impl_macro(mock_indexed_property_getter_callback);
  let _ = pass_as_impl_macro(
    |_scope, _index, _arguments, _return_value| unimplemented!(),
  );
}

/// See `v8::GenericNamedPropertySetterCallback`.
///
/// # Example
///
/// ```
/// # use rusty_v8 as v8;
/// #
/// fn indexed_property_setter_callback_example<'s>(
///   scope: &mut v8::HandleScope<'s>,
///   index: u32,
///   value: v8::Local<'s, v8::Value>,
///   arguments: v8::PropertyCallbackArguments<'s>,
///   return_value: v8::ReturnValue<'s, v8::Value>,
/// ) {
///   todo!();
/// }
/// ```
pub trait IndexedPropertySetterCallback:
  UnitType
  + for<'s> FnOnce(
    &mut HandleScope<'s>,
    u32,
    Local<'s, Value>,
    PropertyCallbackArguments<'s>,
    ReturnValue<'s, Value>,
  )
{
}

impl<F> IndexedPropertySetterCallback for F where
  F: UnitType
    + for<'s> FnOnce(
      &mut HandleScope<'s>,
      u32,
      Local<'s, Value>,
      PropertyCallbackArguments<'s>,
      ReturnValue<'s, Value>,
    )
{
}

#[macro_export]
macro_rules! impl_indexed_property_setter_callback {
  () => {
    impl $crate::IndexedPropertySetterCallback
    + for<'__s> ::std::ops::FnOnce(
      &mut $crate::HandleScope<'__s>,
      u32,
      $crate::Local<'__s, $crate::Value>,
      $crate::PropertyCallbackArguments<'__s>,
      $crate::ReturnValue<'__s, $crate::Value>,
    )
  };
}

#[repr(transparent)]
pub(crate) struct RawIndexedPropertySetterCallback(
  for<'s> extern "C" fn(u32, Local<'s, Value>, *const PropertyCallbackInfo),
);

impl<F: IndexedPropertySetterCallback> From<F>
  for RawIndexedPropertySetterCallback
{
  fn from(_: F) -> Self {
    #[inline(always)]
    extern "C" fn adapter<'s, F: IndexedPropertySetterCallback>(
      index: u32,
      value: Local<'s, Value>,
      info: *const PropertyCallbackInfo,
    ) {
      let scope = &mut unsafe { CallbackScope::new(&*info) };
      let arguments =
        PropertyCallbackArguments::from_property_callback_info(info);
      let return_value =
        ReturnValue::<Value>::from_property_callback_info(info);
      (F::get())(scope, index, value, arguments, return_value)
    }

    Self(adapter::<F>)
  }
}

#[cfg(test)]
fn mock_indexed_property_setter_callback<'s>(
  _scope: &mut HandleScope<'s>,
  _index: u32,
  _value: Local<'s, Value>,
  _arguments: PropertyCallbackArguments<'s>,
  _return_value: ReturnValue<'s, Value>,
) {
  unimplemented!()
}

#[test]
fn indexed_property_setter_callback_as_type_param() {
  fn pass_as_type_param<F: IndexedPropertySetterCallback>(
    _: F,
  ) -> RawIndexedPropertySetterCallback {
    F::get().into()
  }
  let _ = pass_as_type_param(mock_indexed_property_setter_callback);
}

#[test]
fn indexed_property_setter_callback_as_impl_trait() {
  fn pass_as_impl_trait(
    f: impl IndexedPropertySetterCallback,
  ) -> RawIndexedPropertySetterCallback {
    f.into()
  }
  let _ = pass_as_impl_trait(mock_indexed_property_setter_callback);
}

#[test]
fn indexed_property_setter_callback_as_impl_macro() {
  fn pass_as_impl_macro(
    f: impl_indexed_property_setter_callback!(),
  ) -> RawIndexedPropertySetterCallback {
    f.into()
  }
  let _ = pass_as_impl_macro(mock_indexed_property_setter_callback);
  let _ = pass_as_impl_macro(
    |_scope, _index, _value, _arguments, _return_value| unimplemented!(),
  );
}

/// See `v8::GenericNamedPropertyQueryCallback`.
///
/// # Example
///
/// ```
/// # use rusty_v8 as v8;
/// #
/// fn indexed_property_query_callback_example<'s>(
///   scope: &mut v8::HandleScope<'s>,
///   index: u32,
///   arguments: v8::PropertyCallbackArguments<'s>,
///   return_value: v8::ReturnValue<'s, v8::Integer>,
/// ) {
///   todo!();
/// }
/// ```
pub trait IndexedPropertyQueryCallback:
  UnitType
  + for<'s> FnOnce(
    &mut HandleScope<'s>,
    u32,
    PropertyCallbackArguments<'s>,
    ReturnValue<'s, Integer>,
  )
{
}

impl<F> IndexedPropertyQueryCallback for F where
  F: UnitType
    + for<'s> FnOnce(
      &mut HandleScope<'s>,
      u32,
      PropertyCallbackArguments<'s>,
      ReturnValue<'s, Integer>,
    )
{
}

#[macro_export]
macro_rules! impl_indexed_property_query_callback {
  () => {
    impl $crate::IndexedPropertyQueryCallback
    + for<'__s> ::std::ops::FnOnce(
      &mut $crate::HandleScope<'__s>,
      u32,
      $crate::PropertyCallbackArguments<'__s>,
      $crate::ReturnValue<'__s, $crate::Integer>,
    )
  };
}

#[repr(transparent)]
pub(crate) struct RawIndexedPropertyQueryCallback(
  extern "C" fn(u32, *const PropertyCallbackInfo),
);

impl<F: IndexedPropertyQueryCallback> From<F>
  for RawIndexedPropertyQueryCallback
{
  fn from(_: F) -> Self {
    #[inline(always)]
    extern "C" fn adapter<F: IndexedPropertyQueryCallback>(
      index: u32,
      info: *const PropertyCallbackInfo,
    ) {
      let scope = &mut unsafe { CallbackScope::new(&*info) };
      let arguments =
        PropertyCallbackArguments::from_property_callback_info(info);
      let return_value =
        ReturnValue::<Integer>::from_property_callback_info(info);
      (F::get())(scope, index, arguments, return_value)
    }

    Self(adapter::<F>)
  }
}

#[cfg(test)]
fn mock_indexed_property_query_callback<'s>(
  _scope: &mut HandleScope<'s>,
  _index: u32,
  _arguments: PropertyCallbackArguments<'s>,
  _return_value: ReturnValue<'s, Integer>,
) {
  unimplemented!()
}

#[test]
fn indexed_property_query_callback_as_type_param() {
  fn pass_as_type_param<F: IndexedPropertyQueryCallback>(
    _: F,
  ) -> RawIndexedPropertyQueryCallback {
    F::get().into()
  }
  let _ = pass_as_type_param(mock_indexed_property_query_callback);
}

#[test]
fn indexed_property_query_callback_as_impl_trait() {
  fn pass_as_impl_trait(
    f: impl IndexedPropertyQueryCallback,
  ) -> RawIndexedPropertyQueryCallback {
    f.into()
  }
  let _ = pass_as_impl_trait(mock_indexed_property_query_callback);
}

#[test]
fn indexed_property_query_callback_as_impl_macro() {
  fn pass_as_impl_macro(
    f: impl_indexed_property_query_callback!(),
  ) -> RawIndexedPropertyQueryCallback {
    f.into()
  }
  let _ = pass_as_impl_macro(mock_indexed_property_query_callback);
  let _ = pass_as_impl_macro(
    |_scope, _index, _arguments, _return_value| unimplemented!(),
  );
}

/// See `v8::GenericNamedPropertyDeleterCallback`.
///
/// # Example
///
/// ```
/// # use rusty_v8 as v8;
/// #
/// fn indexed_property_deleter_callback_example<'s>(
///   scope: &mut v8::HandleScope<'s>,
///   index: u32,
///   arguments: v8::PropertyCallbackArguments<'s>,
///   return_value: v8::ReturnValue<'s, v8::Boolean>,
/// ) {
///   todo!();
/// }
/// ```
pub trait IndexedPropertyDeleterCallback:
  UnitType
  + for<'s> FnOnce(
    &mut HandleScope<'s>,
    u32,
    PropertyCallbackArguments<'s>,
    ReturnValue<'s, Boolean>,
  )
{
}

impl<F> IndexedPropertyDeleterCallback for F where
  F: UnitType
    + for<'s> FnOnce(
      &mut HandleScope<'s>,
      u32,
      PropertyCallbackArguments<'s>,
      ReturnValue<'s, Boolean>,
    )
{
}

#[macro_export]
macro_rules! impl_indexed_property_deleter_callback {
  () => {
    impl $crate::IndexedPropertyDeleterCallback
    + for<'__s> ::std::ops::FnOnce(
      &mut $crate::HandleScope<'__s>,
      u32,
      $crate::PropertyCallbackArguments<'__s>,
      $crate::ReturnValue<'__s, $crate::Boolean>,
    )
  };
}

#[repr(transparent)]
pub(crate) struct RawIndexedPropertyDeleterCallback(
  extern "C" fn(u32, *const PropertyCallbackInfo),
);

impl<F: IndexedPropertyDeleterCallback> From<F>
  for RawIndexedPropertyDeleterCallback
{
  fn from(_: F) -> Self {
    #[inline(always)]
    extern "C" fn adapter<F: IndexedPropertyDeleterCallback>(
      index: u32,
      info: *const PropertyCallbackInfo,
    ) {
      let scope = &mut unsafe { CallbackScope::new(&*info) };
      let arguments =
        PropertyCallbackArguments::from_property_callback_info(info);
      let return_value =
        ReturnValue::<Boolean>::from_property_callback_info(info);
      (F::get())(scope, index, arguments, return_value)
    }

    Self(adapter::<F>)
  }
}

#[cfg(test)]
fn mock_indexed_property_deleter_callback<'s>(
  _scope: &mut HandleScope<'s>,
  _index: u32,
  _arguments: PropertyCallbackArguments<'s>,
  _return_value: ReturnValue<'s, Boolean>,
) {
  unimplemented!()
}

#[test]
fn indexed_property_deleter_callback_as_type_param() {
  fn pass_as_type_param<F: IndexedPropertyDeleterCallback>(
    _: F,
  ) -> RawIndexedPropertyDeleterCallback {
    F::get().into()
  }
  let _ = pass_as_type_param(mock_indexed_property_deleter_callback);
}

#[test]
fn indexed_property_deleter_callback_as_impl_trait() {
  fn pass_as_impl_trait(
    f: impl IndexedPropertyDeleterCallback,
  ) -> RawIndexedPropertyDeleterCallback {
    f.into()
  }
  let _ = pass_as_impl_trait(mock_indexed_property_deleter_callback);
}

#[test]
fn indexed_property_deleter_callback_as_impl_macro() {
  fn pass_as_impl_macro(
    f: impl_indexed_property_deleter_callback!(),
  ) -> RawIndexedPropertyDeleterCallback {
    f.into()
  }
  let _ = pass_as_impl_macro(mock_indexed_property_deleter_callback);
  let _ = pass_as_impl_macro(
    |_scope, _index, _arguments, _return_value| unimplemented!(),
  );
}

/// Returns an array containing the indices of the properties the indexed
/// property getter intercepts.
///
/// Note: The values in the array must be uint32_t.
///
/// # Example
///
/// ```
/// # use rusty_v8 as v8;
/// #
/// fn indexed_property_enumerator_callback_example<'s>(
///   scope: &mut v8::HandleScope<'s>,
///   arguments: v8::PropertyCallbackArguments<'s>,
///   return_value: v8::ReturnValue<'s, v8::Array>,
/// ) {
///   todo!();
/// }
/// ```
pub trait IndexedPropertyEnumeratorCallback:
  UnitType
  + for<'s> FnOnce(
    &mut HandleScope<'s>,
    PropertyCallbackArguments<'s>,
    ReturnValue<'s, Array>,
  )
{
}

impl<F> IndexedPropertyEnumeratorCallback for F where
  F: UnitType
    + for<'s> FnOnce(
      &mut HandleScope<'s>,
      PropertyCallbackArguments<'s>,
      ReturnValue<'s, Array>,
    )
{
}

#[macro_export]
macro_rules! impl_indexed_property_enumerator_callback {
  () => {
    impl $crate::IndexedPropertyEnumeratorCallback
    + for<'__s> ::std::ops::FnOnce(
      &mut $crate::HandleScope<'__s>,
      $crate::PropertyCallbackArguments<'__s>,
      $crate::ReturnValue<'__s, $crate::Array>,
    )
  };
}

#[repr(transparent)]
pub(crate) struct RawIndexedPropertyEnumeratorCallback(
  extern "C" fn(*const PropertyCallbackInfo),
);

impl<F: IndexedPropertyEnumeratorCallback> From<F>
  for RawIndexedPropertyEnumeratorCallback
{
  fn from(_: F) -> Self {
    #[inline(always)]
    extern "C" fn adapter<F: IndexedPropertyEnumeratorCallback>(
      info: *const PropertyCallbackInfo,
    ) {
      let scope = &mut unsafe { CallbackScope::new(&*info) };
      let arguments =
        PropertyCallbackArguments::from_property_callback_info(info);
      let return_value =
        ReturnValue::<Array>::from_property_callback_info(info);
      (F::get())(scope, arguments, return_value)
    }

    Self(adapter::<F>)
  }
}

#[cfg(test)]
fn mock_indexed_property_enumerator_callback<'s>(
  _scope: &mut HandleScope<'s>,
  _arguments: PropertyCallbackArguments<'s>,
  _return_value: ReturnValue<'s, Array>,
) {
  unimplemented!()
}

#[test]
fn indexed_property_enumerator_callback_as_type_param() {
  fn pass_as_type_param<F: IndexedPropertyEnumeratorCallback>(
    _: F,
  ) -> RawIndexedPropertyEnumeratorCallback {
    F::get().into()
  }
  let _ = pass_as_type_param(mock_indexed_property_enumerator_callback);
}

#[test]
fn indexed_property_enumerator_callback_as_impl_trait() {
  fn pass_as_impl_trait(
    f: impl IndexedPropertyEnumeratorCallback,
  ) -> RawIndexedPropertyEnumeratorCallback {
    f.into()
  }
  let _ = pass_as_impl_trait(mock_indexed_property_enumerator_callback);
}

#[test]
fn indexed_property_enumerator_callback_as_impl_macro() {
  fn pass_as_impl_macro(
    f: impl_indexed_property_enumerator_callback!(),
  ) -> RawIndexedPropertyEnumeratorCallback {
    f.into()
  }
  let _ = pass_as_impl_macro(mock_indexed_property_enumerator_callback);
  let _ =
    pass_as_impl_macro(|_scope, _arguments, _return_value| unimplemented!());
}

/// See `v8::GenericNamedPropertyDefinerCallback`.
///
/// # Example
///
/// ```
/// # use rusty_v8 as v8;
/// #
/// fn indexed_property_definer_callback_example<'s>(
///   scope: &mut v8::HandleScope<'s>,
///   index: u32,
///   desc: &v8::PropertyDescriptor,
///   arguments: v8::PropertyCallbackArguments<'s>,
///   return_value: v8::ReturnValue<'s, v8::Value>,
/// ) {
///   todo!();
/// }
/// ```
pub trait IndexedPropertyDefinerCallback:
  UnitType
  + for<'s> FnOnce(
    &mut HandleScope<'s>,
    u32,
    &PropertyDescriptor,
    PropertyCallbackArguments<'s>,
    ReturnValue<'s, Value>,
  )
{
}

impl<F> IndexedPropertyDefinerCallback for F where
  F: UnitType
    + for<'s> FnOnce(
      &mut HandleScope<'s>,
      u32,
      &PropertyDescriptor,
      PropertyCallbackArguments<'s>,
      ReturnValue<'s, Value>,
    )
{
}

#[macro_export]
macro_rules! impl_indexed_property_definer_callback {
  () => {
    impl $crate::IndexedPropertyDefinerCallback
    + for<'__s> ::std::ops::FnOnce(
      &mut $crate::HandleScope<'__s>,
      u32,
      &$crate::PropertyDescriptor,
      $crate::PropertyCallbackArguments<'__s>,
      $crate::ReturnValue<'__s, $crate::Value>,
    )
  };
}

#[repr(transparent)]
pub(crate) struct RawIndexedPropertyDefinerCallback(
  extern "C" fn(u32, *const PropertyDescriptor, *const PropertyCallbackInfo),
);

impl<F: IndexedPropertyDefinerCallback> From<F>
  for RawIndexedPropertyDefinerCallback
{
  fn from(_: F) -> Self {
    #[inline(always)]
    extern "C" fn adapter<F: IndexedPropertyDefinerCallback>(
      index: u32,
      desc: *const PropertyDescriptor,
      info: *const PropertyCallbackInfo,
    ) {
      let scope = &mut unsafe { CallbackScope::new(&*info) };
      let desc = unsafe { &*desc };
      let arguments =
        PropertyCallbackArguments::from_property_callback_info(info);
      let return_value =
        ReturnValue::<Value>::from_property_callback_info(info);
      (F::get())(scope, index, desc, arguments, return_value)
    }

    Self(adapter::<F>)
  }
}

#[cfg(test)]
fn mock_indexed_property_definer_callback<'s>(
  _scope: &mut HandleScope<'s>,
  _index: u32,
  _desc: &PropertyDescriptor,
  _arguments: PropertyCallbackArguments<'s>,
  _return_value: ReturnValue<'s, Value>,
) {
  unimplemented!()
}

#[test]
fn indexed_property_definer_callback_as_type_param() {
  fn pass_as_type_param<F: IndexedPropertyDefinerCallback>(
    _: F,
  ) -> RawIndexedPropertyDefinerCallback {
    F::get().into()
  }
  let _ = pass_as_type_param(mock_indexed_property_definer_callback);
}

#[test]
fn indexed_property_definer_callback_as_impl_trait() {
  fn pass_as_impl_trait(
    f: impl IndexedPropertyDefinerCallback,
  ) -> RawIndexedPropertyDefinerCallback {
    f.into()
  }
  let _ = pass_as_impl_trait(mock_indexed_property_definer_callback);
}

#[test]
fn indexed_property_definer_callback_as_impl_macro() {
  fn pass_as_impl_macro(
    f: impl_indexed_property_definer_callback!(),
  ) -> RawIndexedPropertyDefinerCallback {
    f.into()
  }
  let _ = pass_as_impl_macro(mock_indexed_property_definer_callback);
  let _ = pass_as_impl_macro(
    |_scope, _index, _desc, _arguments, _return_value| unimplemented!(),
  );
}

/// See `v8::GenericNamedPropertyDescriptorCallback`.
///
/// # Example
///
/// ```
/// # use rusty_v8 as v8;
/// #
/// fn indexed_property_descriptor_callback_example<'s>(
///   scope: &mut v8::HandleScope<'s>,
///   index: u32,
///   arguments: v8::PropertyCallbackArguments<'s>,
///   return_value: v8::ReturnValue<'s, v8::Value>,
/// ) {
///   todo!();
/// }
/// ```
pub trait IndexedPropertyDescriptorCallback:
  UnitType
  + for<'s> FnOnce(
    &mut HandleScope<'s>,
    u32,
    PropertyCallbackArguments<'s>,
    ReturnValue<'s, Value>,
  )
{
}

impl<F> IndexedPropertyDescriptorCallback for F where
  F: UnitType
    + for<'s> FnOnce(
      &mut HandleScope<'s>,
      u32,
      PropertyCallbackArguments<'s>,
      ReturnValue<'s, Value>,
    )
{
}

#[macro_export]
macro_rules! impl_indexed_property_descriptor_callback {
  () => {
    impl $crate::IndexedPropertyDescriptorCallback
    + for<'__s> ::std::ops::FnOnce(
      &mut $crate::HandleScope<'__s>,
      u32,
      $crate::PropertyCallbackArguments<'__s>,
      $crate::ReturnValue<'__s, $crate::Value>,
    )
  };
}

#[repr(transparent)]
pub(crate) struct RawIndexedPropertyDescriptorCallback(
  extern "C" fn(u32, *const PropertyCallbackInfo),
);

impl<F: IndexedPropertyDescriptorCallback> From<F>
  for RawIndexedPropertyDescriptorCallback
{
  fn from(_: F) -> Self {
    #[inline(always)]
    extern "C" fn adapter<F: IndexedPropertyDescriptorCallback>(
      index: u32,
      info: *const PropertyCallbackInfo,
    ) {
      let scope = &mut unsafe { CallbackScope::new(&*info) };
      let arguments =
        PropertyCallbackArguments::from_property_callback_info(info);
      let return_value =
        ReturnValue::<Value>::from_property_callback_info(info);
      (F::get())(scope, index, arguments, return_value)
    }

    Self(adapter::<F>)
  }
}

#[cfg(test)]
fn mock_indexed_property_descriptor_callback<'s>(
  _scope: &mut HandleScope<'s>,
  _index: u32,
  _arguments: PropertyCallbackArguments<'s>,
  _return_value: ReturnValue<'s, Value>,
) {
  unimplemented!()
}

#[test]
fn indexed_property_descriptor_callback_as_type_param() {
  fn pass_as_type_param<F: IndexedPropertyDescriptorCallback>(
    _: F,
  ) -> RawIndexedPropertyDescriptorCallback {
    F::get().into()
  }
  let _ = pass_as_type_param(mock_indexed_property_descriptor_callback);
}

#[test]
fn indexed_property_descriptor_callback_as_impl_trait() {
  fn pass_as_impl_trait(
    f: impl IndexedPropertyDescriptorCallback,
  ) -> RawIndexedPropertyDescriptorCallback {
    f.into()
  }
  let _ = pass_as_impl_trait(mock_indexed_property_descriptor_callback);
}

#[test]
fn indexed_property_descriptor_callback_as_impl_macro() {
  fn pass_as_impl_macro(
    f: impl_indexed_property_descriptor_callback!(),
  ) -> RawIndexedPropertyDescriptorCallback {
    f.into()
  }
  let _ = pass_as_impl_macro(mock_indexed_property_descriptor_callback);
  let _ = pass_as_impl_macro(
    |_scope, _index, _arguments, _return_value| unimplemented!(),
  );
}

/// Returns true if the given context should be allowed to access the given
/// object.
///
/// # Example
///
/// ```
/// # use rusty_v8 as v8;
/// #
/// fn access_check_callback_example<'s>(
///   scope: &mut v8::HandleScope<'s>,
///   accessed_object: v8::Local<'s, v8::Object>,
///   data: v8::Local<'s, v8::Value>,
/// ) -> bool {
///   todo!()
/// }
/// ```
pub trait AccessCheckCallback:
  UnitType
  + for<'s> FnOnce(
    &mut HandleScope<'s>,
    Local<'s, Object>,
    Local<'s, Value>,
  ) -> bool
{
}

impl<F> AccessCheckCallback for F where
  F: UnitType
    + for<'s> FnOnce(
      &mut HandleScope<'s>,
      Local<'s, Object>,
      Local<'s, Value>,
    ) -> bool
{
}

#[macro_export]
macro_rules! impl_access_check_callback {
  () => {
    impl $crate::AccessCheckCallback
    + for<'__s> ::std::ops::FnOnce(
      &mut $crate::HandleScope<'__s>,
      $crate::Local<'__s, $crate::Object>,
      $crate::Local<'__s, $crate::Value>,
    ) -> bool
  };
}

#[repr(transparent)]
pub(crate) struct RawAccessCheckCallback(
  for<'s> extern "C" fn(
    Local<'s, Context>,
    Local<'s, Object>,
    Local<'s, Value>,
  ) -> bool,
);

impl<F: AccessCheckCallback> From<F> for RawAccessCheckCallback {
  fn from(_: F) -> Self {
    #[inline(always)]
    extern "C" fn adapter<'s, F: AccessCheckCallback>(
      accessing_context: Local<'s, Context>,
      accessed_object: Local<'s, Object>,
      data: Local<'s, Value>,
    ) -> bool {
      let scope = &mut unsafe { CallbackScope::new(accessing_context) };
      (F::get())(scope, accessed_object, data)
    }

    Self(adapter::<F>)
  }
}

#[cfg(test)]
fn mock_access_check_callback<'s>(
  _scope: &mut HandleScope<'s>,
  _accessed_object: Local<'s, Object>,
  _data: Local<'s, Value>,
) -> bool {
  unimplemented!()
}

#[test]
fn access_check_callback_as_type_param() {
  fn pass_as_type_param<F: AccessCheckCallback>(
    _: F,
  ) -> RawAccessCheckCallback {
    F::get().into()
  }
  let _ = pass_as_type_param(mock_access_check_callback);
}

#[test]
fn access_check_callback_as_impl_trait() {
  fn pass_as_impl_trait(f: impl AccessCheckCallback) -> RawAccessCheckCallback {
    f.into()
  }
  let _ = pass_as_impl_trait(mock_access_check_callback);
}

#[test]
fn access_check_callback_as_impl_macro() {
  fn pass_as_impl_macro(
    f: impl_access_check_callback!(),
  ) -> RawAccessCheckCallback {
    f.into()
  }
  let _ = pass_as_impl_macro(mock_access_check_callback);
  let _ =
    pass_as_impl_macro(|_scope, _accessed_object, _data| unimplemented!());
}

// === FatalErrorCallback ===

/// # Example
///
/// ```
/// # use std::ffi::CStr;
/// #
/// fn fatal_error_callback_example(location: &CStr, message: &CStr) {
///   todo!();
/// }
/// ```
pub trait FatalErrorCallback: UnitType + FnOnce(&CStr, &CStr) {}

impl<F> FatalErrorCallback for F where F: UnitType + FnOnce(&CStr, &CStr) {}

#[macro_export]
macro_rules! impl_fatal_error_callback {
  () => {
    impl $crate::FatalErrorCallback
    + ::std::ops::FnOnce(
      &::std::ffi::CStr,
      &::std::ffi::CStr,
    )
  };
}

#[repr(transparent)]
pub(crate) struct RawFatalErrorCallback(
  extern "C" fn(*const c_char, *const c_char),
);

impl<F: FatalErrorCallback> From<F> for RawFatalErrorCallback {
  fn from(_: F) -> Self {
    #[inline(always)]
    extern "C" fn adapter<F: FatalErrorCallback>(
      location: *const c_char,
      message: *const c_char,
    ) {
      let location = unsafe { CStr::from_ptr(location) };
      let message = unsafe { CStr::from_ptr(message) };
      (F::get())(location, message)
    }

    Self(adapter::<F>)
  }
}

#[cfg(test)]
fn mock_fatal_error_callback(_location: &CStr, _message: &CStr) {
  unimplemented!()
}

#[test]
fn fatal_error_callback_as_type_param() {
  fn pass_as_type_param<F: FatalErrorCallback>(_: F) -> RawFatalErrorCallback {
    F::get().into()
  }
  let _ = pass_as_type_param(mock_fatal_error_callback);
}

#[test]
fn fatal_error_callback_as_impl_trait() {
  fn pass_as_impl_trait(f: impl FatalErrorCallback) -> RawFatalErrorCallback {
    f.into()
  }
  let _ = pass_as_impl_trait(mock_fatal_error_callback);
}

#[test]
fn fatal_error_callback_as_impl_macro() {
  fn pass_as_impl_macro(
    f: impl_fatal_error_callback!(),
  ) -> RawFatalErrorCallback {
    f.into()
  }
  let _ = pass_as_impl_macro(mock_fatal_error_callback);
  let _ = pass_as_impl_macro(|_location, _message| unimplemented!());
}

// === OOMErrorCallback ===

/// # Example
///
/// ```
/// # use std::ffi::CStr;
/// #
/// fn oom_error_callback_example(location: &CStr, is_heap_oom: bool) {
///   todo!();
/// }
/// ```
pub trait OOMErrorCallback: UnitType + FnOnce(&CStr, bool) {}

impl<F> OOMErrorCallback for F where F: UnitType + FnOnce(&CStr, bool) {}

#[macro_export]
macro_rules! impl_oom_error_callback {
  () => {
    impl $crate::OOMErrorCallback
    + ::std::ops::FnOnce(
      &::std::ffi::CStr,
      bool,
    )
  };
}

#[repr(transparent)]
pub(crate) struct RawOOMErrorCallback(extern "C" fn(*const c_char, bool));

impl<F: OOMErrorCallback> From<F> for RawOOMErrorCallback {
  fn from(_: F) -> Self {
    #[inline(always)]
    extern "C" fn adapter<F: OOMErrorCallback>(
      location: *const c_char,
      is_heap_oom: bool,
    ) {
      let location = unsafe { CStr::from_ptr(location) };
      (F::get())(location, is_heap_oom)
    }

    Self(adapter::<F>)
  }
}

#[cfg(test)]
fn mock_oom_error_callback(_location: &CStr, _is_heap_oom: bool) {
  unimplemented!()
}

#[test]
fn oom_error_callback_as_type_param() {
  fn pass_as_type_param<F: OOMErrorCallback>(_: F) -> RawOOMErrorCallback {
    F::get().into()
  }
  let _ = pass_as_type_param(mock_oom_error_callback);
}

#[test]
fn oom_error_callback_as_impl_trait() {
  fn pass_as_impl_trait(f: impl OOMErrorCallback) -> RawOOMErrorCallback {
    f.into()
  }
  let _ = pass_as_impl_trait(mock_oom_error_callback);
}

#[test]
fn oom_error_callback_as_impl_macro() {
  fn pass_as_impl_macro(f: impl_oom_error_callback!()) -> RawOOMErrorCallback {
    f.into()
  }
  let _ = pass_as_impl_macro(mock_oom_error_callback);
  let _ = pass_as_impl_macro(|_location, _is_heap_oom| unimplemented!());
}

// === DcheckErrorCallback ===

/// # Example
///
/// ```
/// # use std::ffi::CStr;
/// #
/// fn dcheck_error_callback_example(file: &CStr, line: i32, message: &CStr) {
///   todo!();
/// }
/// ```
pub trait DcheckErrorCallback: UnitType + FnOnce(&CStr, i32, &CStr) {}

impl<F> DcheckErrorCallback for F where F: UnitType + FnOnce(&CStr, i32, &CStr) {}

#[macro_export]
macro_rules! impl_dcheck_error_callback {
  () => {
    impl $crate::DcheckErrorCallback
    + ::std::ops::FnOnce(
      &::std::ffi::CStr,
      i32,
      &::std::ffi::CStr,
    )
  };
}

#[repr(transparent)]
pub(crate) struct RawDcheckErrorCallback(
  extern "C" fn(*const c_char, c_int, *const c_char),
);

impl<F: DcheckErrorCallback> From<F> for RawDcheckErrorCallback {
  fn from(_: F) -> Self {
    #[inline(always)]
    extern "C" fn adapter<F: DcheckErrorCallback>(
      file: *const c_char,
      line: c_int,
      message: *const c_char,
    ) {
      let file = unsafe { CStr::from_ptr(file) };
      let message = unsafe { CStr::from_ptr(message) };
      (F::get())(file, line, message)
    }

    Self(adapter::<F>)
  }
}

#[cfg(test)]
fn mock_dcheck_error_callback(_file: &CStr, _line: i32, _message: &CStr) {
  unimplemented!()
}

#[test]
fn dcheck_error_callback_as_type_param() {
  fn pass_as_type_param<F: DcheckErrorCallback>(
    _: F,
  ) -> RawDcheckErrorCallback {
    F::get().into()
  }
  let _ = pass_as_type_param(mock_dcheck_error_callback);
}

#[test]
fn dcheck_error_callback_as_impl_trait() {
  fn pass_as_impl_trait(f: impl DcheckErrorCallback) -> RawDcheckErrorCallback {
    f.into()
  }
  let _ = pass_as_impl_trait(mock_dcheck_error_callback);
}

#[test]
fn dcheck_error_callback_as_impl_macro() {
  fn pass_as_impl_macro(
    f: impl_dcheck_error_callback!(),
  ) -> RawDcheckErrorCallback {
    f.into()
  }
  let _ = pass_as_impl_macro(mock_dcheck_error_callback);
  let _ = pass_as_impl_macro(|_file, _line, _message| unimplemented!());
}

// === MessageCallback ===

/// # Example
///
/// ```
/// # use rusty_v8 as v8;
/// #
/// fn message_callback_example<'s>(
///   scope: &mut v8::HandleScope<'s>,
///   message: v8::Local<'s, v8::Message>,
///   data: v8::Local<'s, v8::Value>,
/// ) {
///   todo!();
/// }
/// ```
pub trait MessageCallback:
  UnitType
  + for<'s> FnOnce(&mut HandleScope<'s>, Local<'s, Message>, Local<'s, Value>)
{
}

impl<F> MessageCallback for F where
  F: UnitType
    + for<'s> FnOnce(&mut HandleScope<'s>, Local<'s, Message>, Local<'s, Value>)
{
}

#[macro_export]
macro_rules! impl_message_callback {
  () => {
    impl $crate::MessageCallback
    + for<'__s> ::std::ops::FnOnce(
      &mut $crate::HandleScope<'__s>,
      $crate::Local<'__s, $crate::Message>,
      $crate::Local<'__s, $crate::Value>,
    )
  };
}

#[repr(transparent)]
pub(crate) struct RawMessageCallback(
  for<'s> extern "C" fn(Local<'s, Message>, Local<'s, Value>),
);

impl<F: MessageCallback> From<F> for RawMessageCallback {
  fn from(_: F) -> Self {
    #[inline(always)]
    extern "C" fn adapter<'s, F: MessageCallback>(
      message: Local<'s, Message>,
      data: Local<'s, Value>,
    ) {
      let scope = &mut unsafe { CallbackScope::new(message) };
      (F::get())(scope, message, data)
    }

    Self(adapter::<F>)
  }
}

#[cfg(test)]
fn mock_message_callback<'s>(
  _scope: &mut HandleScope<'s>,
  _message: Local<'s, Message>,
  _data: Local<'s, Value>,
) {
  unimplemented!()
}

#[test]
fn message_callback_as_type_param() {
  fn pass_as_type_param<F: MessageCallback>(_: F) -> RawMessageCallback {
    F::get().into()
  }
  let _ = pass_as_type_param(mock_message_callback);
}

#[test]
fn message_callback_as_impl_trait() {
  fn pass_as_impl_trait(f: impl MessageCallback) -> RawMessageCallback {
    f.into()
  }
  let _ = pass_as_impl_trait(mock_message_callback);
}

#[test]
fn message_callback_as_impl_macro() {
  fn pass_as_impl_macro(f: impl_message_callback!()) -> RawMessageCallback {
    f.into()
  }
  let _ = pass_as_impl_macro(mock_message_callback);
  let _ = pass_as_impl_macro(|_scope, _message, _data| unimplemented!());
}

// === LogEventCallback ===

/// # Example
///
/// ```
/// # use std::ffi::CStr;
/// #
/// fn log_event_callback_example(name: &CStr, event: i32) {
///   todo!();
/// }
/// ```
pub trait LogEventCallback: UnitType + FnOnce(&CStr, i32) {}

impl<F> LogEventCallback for F where F: UnitType + FnOnce(&CStr, i32) {}

#[macro_export]
macro_rules! impl_log_event_callback {
  () => {
    impl $crate::LogEventCallback
    + ::std::ops::FnOnce(
      &::std::ffi::CStr,
      i32,
    )
  };
}

#[repr(transparent)]
pub(crate) struct RawLogEventCallback(extern "C" fn(*const c_char, c_int));

impl<F: LogEventCallback> From<F> for RawLogEventCallback {
  fn from(_: F) -> Self {
    #[inline(always)]
    extern "C" fn adapter<F: LogEventCallback>(
      name: *const c_char,
      event: c_int,
    ) {
      let name = unsafe { CStr::from_ptr(name) };
      (F::get())(name, event)
    }

    Self(adapter::<F>)
  }
}

#[cfg(test)]
fn mock_log_event_callback(_name: &CStr, _event: i32) {
  unimplemented!()
}

#[test]
fn log_event_callback_as_type_param() {
  fn pass_as_type_param<F: LogEventCallback>(_: F) -> RawLogEventCallback {
    F::get().into()
  }
  let _ = pass_as_type_param(mock_log_event_callback);
}

#[test]
fn log_event_callback_as_impl_trait() {
  fn pass_as_impl_trait(f: impl LogEventCallback) -> RawLogEventCallback {
    f.into()
  }
  let _ = pass_as_impl_trait(mock_log_event_callback);
}

#[test]
fn log_event_callback_as_impl_macro() {
  fn pass_as_impl_macro(f: impl_log_event_callback!()) -> RawLogEventCallback {
    f.into()
  }
  let _ = pass_as_impl_macro(mock_log_event_callback);
  let _ = pass_as_impl_macro(|_name, _event| unimplemented!());
}

// === CounterLookupCallback ===

/// # Example
///
/// ```
/// # use std::cell::Cell;
/// # use std::ffi::CStr;
/// #
/// fn counter_lookup_callback_example(name: &CStr) -> &Cell<i32> {
///   todo!()
/// }
/// ```
pub trait CounterLookupCallback:
  UnitType + FnOnce(&CStr) -> &Cell<i32>
{
}

impl<F> CounterLookupCallback for F where
  F: UnitType + FnOnce(&CStr) -> &Cell<i32>
{
}

#[macro_export]
macro_rules! impl_counter_lookup_callback {
  () => {
    impl $crate::CounterLookupCallback
    + ::std::ops::FnOnce(
      &::std::ffi::CStr,
    ) -> &::std::cell::Cell<i32>
  };
}

#[repr(transparent)]
pub(crate) struct RawCounterLookupCallback(
  extern "C" fn(*const c_char) -> *mut c_int,
);

impl<F: CounterLookupCallback> From<F> for RawCounterLookupCallback {
  fn from(_: F) -> Self {
    #[inline(always)]
    extern "C" fn adapter<F: CounterLookupCallback>(
      name: *const c_char,
    ) -> *mut c_int {
      let name = unsafe { CStr::from_ptr(name) };
      (F::get())(name).as_ptr()
    }

    Self(adapter::<F>)
  }
}

#[cfg(test)]
fn mock_counter_lookup_callback(_name: &CStr) -> &Cell<i32> {
  unimplemented!()
}

#[test]
fn counter_lookup_callback_as_type_param() {
  fn pass_as_type_param<F: CounterLookupCallback>(
    _: F,
  ) -> RawCounterLookupCallback {
    F::get().into()
  }
  let _ = pass_as_type_param(mock_counter_lookup_callback);
}

#[test]
fn counter_lookup_callback_as_impl_trait() {
  fn pass_as_impl_trait(
    f: impl CounterLookupCallback,
  ) -> RawCounterLookupCallback {
    f.into()
  }
  let _ = pass_as_impl_trait(mock_counter_lookup_callback);
}

#[test]
fn counter_lookup_callback_as_impl_macro() {
  fn pass_as_impl_macro(
    f: impl_counter_lookup_callback!(),
  ) -> RawCounterLookupCallback {
    f.into()
  }
  let _ = pass_as_impl_macro(mock_counter_lookup_callback);
  let _ = pass_as_impl_macro(|_name| unimplemented!());
}

// === CreateHistogramCallback ===

/// # Example
///
/// ```
/// # use std::ffi::CStr;
/// #
/// fn create_histogram_callback_example(
///   name: &CStr,
///   min: i32,
///   max: i32,
///   buckets: usize,
/// ) -> *mut () {
///   todo!()
/// }
/// ```
pub trait CreateHistogramCallback:
  UnitType + FnOnce(&CStr, i32, i32, usize) -> *mut ()
{
}

impl<F> CreateHistogramCallback for F where
  F: UnitType + FnOnce(&CStr, i32, i32, usize) -> *mut ()
{
}

#[macro_export]
macro_rules! impl_create_histogram_callback {
  () => {
    impl $crate::CreateHistogramCallback
    + ::std::ops::FnOnce(
      &::std::ffi::CStr,
      i32,
      i32,
      usize,
    ) -> *mut ()
  };
}

#[repr(transparent)]
pub(crate) struct RawCreateHistogramCallback(
  extern "C" fn(*const c_char, c_int, c_int, usize) -> *mut c_void,
);

impl<F: CreateHistogramCallback> From<F> for RawCreateHistogramCallback {
  fn from(_: F) -> Self {
    #[inline(always)]
    extern "C" fn adapter<F: CreateHistogramCallback>(
      name: *const c_char,
      min: c_int,
      max: c_int,
      buckets: usize,
    ) -> *mut c_void {
      let name = unsafe { CStr::from_ptr(name) };
      (F::get())(name, min, max, buckets) as *mut c_void
    }

    Self(adapter::<F>)
  }
}

#[cfg(test)]
fn mock_create_histogram_callback(
  _name: &CStr,
  _min: i32,
  _max: i32,
  _buckets: usize,
) -> *mut () {
  unimplemented!()
}

#[test]
fn create_histogram_callback_as_type_param() {
  fn pass_as_type_param<F: CreateHistogramCallback>(
    _: F,
  ) -> RawCreateHistogramCallback {
    F::get().into()
  }
  let _ = pass_as_type_param(mock_create_histogram_callback);
}

#[test]
fn create_histogram_callback_as_impl_trait() {
  fn pass_as_impl_trait(
    f: impl CreateHistogramCallback,
  ) -> RawCreateHistogramCallback {
    f.into()
  }
  let _ = pass_as_impl_trait(mock_create_histogram_callback);
}

#[test]
fn create_histogram_callback_as_impl_macro() {
  fn pass_as_impl_macro(
    f: impl_create_histogram_callback!(),
  ) -> RawCreateHistogramCallback {
    f.into()
  }
  let _ = pass_as_impl_macro(mock_create_histogram_callback);
  let _ = pass_as_impl_macro(|_name, _min, _max, _buckets| unimplemented!());
}

// === AddHistogramSampleCallback ===

/// # Example
///
/// ```
/// fn add_histogram_sample_callback_example(histogram: *mut (), sample: i32) {
///   todo!();
/// }
/// ```
pub trait AddHistogramSampleCallback: UnitType + FnOnce(*mut (), i32) {}

impl<F> AddHistogramSampleCallback for F where F: UnitType + FnOnce(*mut (), i32)
{}

#[macro_export]
macro_rules! impl_add_histogram_sample_callback {
  () => {
    impl $crate::AddHistogramSampleCallback
    + ::std::ops::FnOnce(
      *mut (),
      i32,
    )
  };
}

#[repr(transparent)]
pub(crate) struct RawAddHistogramSampleCallback(
  extern "C" fn(*mut c_void, c_int),
);

impl<F: AddHistogramSampleCallback> From<F> for RawAddHistogramSampleCallback {
  fn from(_: F) -> Self {
    #[inline(always)]
    extern "C" fn adapter<F: AddHistogramSampleCallback>(
      histogram: *mut c_void,
      sample: c_int,
    ) {
      let histogram = histogram as *mut ();
      (F::get())(histogram, sample)
    }

    Self(adapter::<F>)
  }
}

#[cfg(test)]
fn mock_add_histogram_sample_callback(_histogram: *mut (), _sample: i32) {
  unimplemented!()
}

#[test]
fn add_histogram_sample_callback_as_type_param() {
  fn pass_as_type_param<F: AddHistogramSampleCallback>(
    _: F,
  ) -> RawAddHistogramSampleCallback {
    F::get().into()
  }
  let _ = pass_as_type_param(mock_add_histogram_sample_callback);
}

#[test]
fn add_histogram_sample_callback_as_impl_trait() {
  fn pass_as_impl_trait(
    f: impl AddHistogramSampleCallback,
  ) -> RawAddHistogramSampleCallback {
    f.into()
  }
  let _ = pass_as_impl_trait(mock_add_histogram_sample_callback);
}

#[test]
fn add_histogram_sample_callback_as_impl_macro() {
  fn pass_as_impl_macro(
    f: impl_add_histogram_sample_callback!(),
  ) -> RawAddHistogramSampleCallback {
    f.into()
  }
  let _ = pass_as_impl_macro(mock_add_histogram_sample_callback);
  let _ = pass_as_impl_macro(|_histogram, _sample| unimplemented!());
}

// === AddCrashKeyCallback ===

/// # Example
///
/// ```
/// # use rusty_v8 as v8;
/// # use std::ffi::CStr;
/// #
/// fn add_crash_key_callback_example(id: v8::CrashKeyId, value: &CStr) {
///   todo!();
/// }
/// ```
pub trait AddCrashKeyCallback: UnitType + FnOnce(CrashKeyId, &CStr) {}

impl<F> AddCrashKeyCallback for F where F: UnitType + FnOnce(CrashKeyId, &CStr) {}

#[macro_export]
macro_rules! impl_add_crash_key_callback {
  () => {
    impl $crate::AddCrashKeyCallback
    + ::std::ops::FnOnce(
      $crate::CrashKeyId,
      &::std::ffi::CStr,
    )
  };
}

#[repr(transparent)]
pub(crate) struct RawAddCrashKeyCallback(
  extern "C" fn(CrashKeyId, *const CxxString),
);

impl<F: AddCrashKeyCallback> From<F> for RawAddCrashKeyCallback {
  fn from(_: F) -> Self {
    #[inline(always)]
    extern "C" fn adapter<F: AddCrashKeyCallback>(
      id: CrashKeyId,
      value: *const CxxString,
    ) {
      let value = <&CStr>::from(unsafe { &*value });
      (F::get())(id, value)
    }

    Self(adapter::<F>)
  }
}

#[cfg(test)]
fn mock_add_crash_key_callback(_id: CrashKeyId, _value: &CStr) {
  unimplemented!()
}

#[test]
fn add_crash_key_callback_as_type_param() {
  fn pass_as_type_param<F: AddCrashKeyCallback>(
    _: F,
  ) -> RawAddCrashKeyCallback {
    F::get().into()
  }
  let _ = pass_as_type_param(mock_add_crash_key_callback);
}

#[test]
fn add_crash_key_callback_as_impl_trait() {
  fn pass_as_impl_trait(f: impl AddCrashKeyCallback) -> RawAddCrashKeyCallback {
    f.into()
  }
  let _ = pass_as_impl_trait(mock_add_crash_key_callback);
}

#[test]
fn add_crash_key_callback_as_impl_macro() {
  fn pass_as_impl_macro(
    f: impl_add_crash_key_callback!(),
  ) -> RawAddCrashKeyCallback {
    f.into()
  }
  let _ = pass_as_impl_macro(mock_add_crash_key_callback);
  let _ = pass_as_impl_macro(|_id, _value| unimplemented!());
}

// === BeforeCallEnteredCallback ===

/// # Example
///
/// ```
/// # use rusty_v8 as v8;
/// #
/// fn before_call_entered_callback_example(isolate: &mut v8::Isolate) {
///   todo!();
/// }
/// ```
pub trait BeforeCallEnteredCallback: UnitType + FnOnce(&mut Isolate) {}

impl<F> BeforeCallEnteredCallback for F where F: UnitType + FnOnce(&mut Isolate) {}

#[macro_export]
macro_rules! impl_before_call_entered_callback {
  () => {
    impl $crate::BeforeCallEnteredCallback
    + ::std::ops::FnOnce(
      &mut $crate::Isolate,
    )
  };
}

#[repr(transparent)]
pub(crate) struct RawBeforeCallEnteredCallback(extern "C" fn(*mut Isolate));

impl<F: BeforeCallEnteredCallback> From<F> for RawBeforeCallEnteredCallback {
  fn from(_: F) -> Self {
    #[inline(always)]
    extern "C" fn adapter<F: BeforeCallEnteredCallback>(isolate: *mut Isolate) {
      let isolate = unsafe { &mut *isolate };
      (F::get())(isolate)
    }

    Self(adapter::<F>)
  }
}

#[cfg(test)]
fn mock_before_call_entered_callback(_isolate: &mut Isolate) {
  unimplemented!()
}

#[test]
fn before_call_entered_callback_as_type_param() {
  fn pass_as_type_param<F: BeforeCallEnteredCallback>(
    _: F,
  ) -> RawBeforeCallEnteredCallback {
    F::get().into()
  }
  let _ = pass_as_type_param(mock_before_call_entered_callback);
}

#[test]
fn before_call_entered_callback_as_impl_trait() {
  fn pass_as_impl_trait(
    f: impl BeforeCallEnteredCallback,
  ) -> RawBeforeCallEnteredCallback {
    f.into()
  }
  let _ = pass_as_impl_trait(mock_before_call_entered_callback);
}

#[test]
fn before_call_entered_callback_as_impl_macro() {
  fn pass_as_impl_macro(
    f: impl_before_call_entered_callback!(),
  ) -> RawBeforeCallEnteredCallback {
    f.into()
  }
  let _ = pass_as_impl_macro(mock_before_call_entered_callback);
  let _ = pass_as_impl_macro(|_isolate| unimplemented!());
}

// === CallCompletedCallback ===

/// # Example
///
/// ```
/// # use rusty_v8 as v8;
/// #
/// fn call_completed_callback_example(isolate: &mut v8::Isolate) {
///   todo!();
/// }
/// ```
pub trait CallCompletedCallback: UnitType + FnOnce(&mut Isolate) {}

impl<F> CallCompletedCallback for F where F: UnitType + FnOnce(&mut Isolate) {}

#[macro_export]
macro_rules! impl_call_completed_callback {
  () => {
    impl $crate::CallCompletedCallback
    + ::std::ops::FnOnce(
      &mut $crate::Isolate,
    )
  };
}

#[repr(transparent)]
pub(crate) struct RawCallCompletedCallback(extern "C" fn(*mut Isolate));

impl<F: CallCompletedCallback> From<F> for RawCallCompletedCallback {
  fn from(_: F) -> Self {
    #[inline(always)]
    extern "C" fn adapter<F: CallCompletedCallback>(isolate: *mut Isolate) {
      let isolate = unsafe { &mut *isolate };
      (F::get())(isolate)
    }

    Self(adapter::<F>)
  }
}

#[cfg(test)]
fn mock_call_completed_callback(_isolate: &mut Isolate) {
  unimplemented!()
}

#[test]
fn call_completed_callback_as_type_param() {
  fn pass_as_type_param<F: CallCompletedCallback>(
    _: F,
  ) -> RawCallCompletedCallback {
    F::get().into()
  }
  let _ = pass_as_type_param(mock_call_completed_callback);
}

#[test]
fn call_completed_callback_as_impl_trait() {
  fn pass_as_impl_trait(
    f: impl CallCompletedCallback,
  ) -> RawCallCompletedCallback {
    f.into()
  }
  let _ = pass_as_impl_trait(mock_call_completed_callback);
}

#[test]
fn call_completed_callback_as_impl_macro() {
  fn pass_as_impl_macro(
    f: impl_call_completed_callback!(),
  ) -> RawCallCompletedCallback {
    f.into()
  }
  let _ = pass_as_impl_macro(mock_call_completed_callback);
  let _ = pass_as_impl_macro(|_isolate| unimplemented!());
}

/// HostImportModuleDynamicallyCallback is called when we require the
/// embedder to load a module. This is used as part of the dynamic
/// import syntax.
///
/// The referrer contains metadata about the script/module that calls
/// import.
///
/// The specifier is the name of the module that should be imported.
///
/// The embedder must compile, instantiate, evaluate the Module, and
/// obtain it's namespace object.
///
/// The Promise returned from this function is forwarded to userland
/// JavaScript. The embedder must resolve this promise with the module
/// namespace object. In case of an exception, the embedder must reject
/// this promise with the exception. If the promise creation itself
/// fails (e.g. due to stack overflow), the embedder must propagate
/// that exception by returning an empty MaybeLocal.
///
/// # Example
///
/// ```
/// # use rusty_v8 as v8;
/// #
/// fn host_import_module_dynamically_callback_example<'s>(
///   scope: &mut v8::HandleScope<'s>,
///   referrer: v8::Local<'s, v8::ScriptOrModule>,
///   specifier: v8::Local<'s, v8::String>,
/// ) -> Option<v8::Local<'s, v8::Promise>> {
///   todo!()
/// }
/// ```
pub trait HostImportModuleDynamicallyCallback:
  UnitType
  + for<'s> FnOnce(
    &mut HandleScope<'s>,
    Local<'s, ScriptOrModule>,
    Local<'s, String>,
  ) -> Option<Local<'s, Promise>>
{
}

impl<F> HostImportModuleDynamicallyCallback for F where
  F: UnitType
    + for<'s> FnOnce(
      &mut HandleScope<'s>,
      Local<'s, ScriptOrModule>,
      Local<'s, String>,
    ) -> Option<Local<'s, Promise>>
{
}

#[macro_export]
macro_rules! impl_host_import_module_dynamically_callback {
  () => {
    impl $crate::HostImportModuleDynamicallyCallback
    + for<'__s> ::std::ops::FnOnce(
      &mut $crate::HandleScope<'__s>,
      $crate::Local<'__s, $crate::ScriptOrModule>,
      $crate::Local<'__s, $crate::String>,
    ) -> ::std::option::Option<$crate::Local<'__s, $crate::Promise>>
  };
}

#[cfg(target_family = "unix")]
#[repr(transparent)]
pub(crate) struct RawHostImportModuleDynamicallyCallback(
  for<'s> extern "C" fn(
    Local<'s, Context>,
    Local<'s, ScriptOrModule>,
    Local<'s, String>,
  ) -> Option<Local<'s, Promise>>,
);

#[cfg(all(target_family = "windows", target_arch = "x86_64"))]
#[repr(transparent)]
pub(crate) struct RawHostImportModuleDynamicallyCallback(
  for<'s> extern "C" fn(
    *mut Option<Local<'s, Promise>>,
    Local<'s, Context>,
    Local<'s, ScriptOrModule>,
    Local<'s, String>,
  ) -> *mut Option<Local<'s, Promise>>,
);

impl<F: HostImportModuleDynamicallyCallback> From<F>
  for RawHostImportModuleDynamicallyCallback
{
  fn from(_: F) -> Self {
    #[inline(always)]
    fn signature_adapter<'s, F: HostImportModuleDynamicallyCallback>(
      context: Local<'s, Context>,
      referrer: Local<'s, ScriptOrModule>,
      specifier: Local<'s, String>,
    ) -> Option<Local<'s, Promise>> {
      let scope = &mut unsafe { CallbackScope::new(context) };
      (F::get())(scope, referrer, specifier)
    }

    #[cfg(target_family = "unix")]
    #[inline(always)]
    extern "C" fn abi_adapter<'s, F: HostImportModuleDynamicallyCallback>(
      context: Local<'s, Context>,
      referrer: Local<'s, ScriptOrModule>,
      specifier: Local<'s, String>,
    ) -> Option<Local<'s, Promise>> {
      signature_adapter::<F>(context, referrer, specifier)
    }

    #[cfg(all(target_family = "windows", target_arch = "x86_64"))]
    #[inline(always)]
    extern "C" fn abi_adapter<'s, F: HostImportModuleDynamicallyCallback>(
      return_value: *mut Option<Local<'s, Promise>>,
      context: Local<'s, Context>,
      referrer: Local<'s, ScriptOrModule>,
      specifier: Local<'s, String>,
    ) -> *mut Option<Local<'s, Promise>> {
      unsafe {
        std::ptr::write(
          return_value,
          signature_adapter::<F>(context, referrer, specifier),
        );
        return_value
      }
    }

    Self(abi_adapter::<F>)
  }
}

#[cfg(test)]
fn mock_host_import_module_dynamically_callback<'s>(
  _scope: &mut HandleScope<'s>,
  _referrer: Local<'s, ScriptOrModule>,
  _specifier: Local<'s, String>,
) -> Option<Local<'s, Promise>> {
  unimplemented!()
}

#[test]
fn host_import_module_dynamically_callback_as_type_param() {
  fn pass_as_type_param<F: HostImportModuleDynamicallyCallback>(
    _: F,
  ) -> RawHostImportModuleDynamicallyCallback {
    F::get().into()
  }
  let _ = pass_as_type_param(mock_host_import_module_dynamically_callback);
}

#[test]
fn host_import_module_dynamically_callback_as_impl_trait() {
  fn pass_as_impl_trait(
    f: impl HostImportModuleDynamicallyCallback,
  ) -> RawHostImportModuleDynamicallyCallback {
    f.into()
  }
  let _ = pass_as_impl_trait(mock_host_import_module_dynamically_callback);
}

#[test]
fn host_import_module_dynamically_callback_as_impl_macro() {
  fn pass_as_impl_macro(
    f: impl_host_import_module_dynamically_callback!(),
  ) -> RawHostImportModuleDynamicallyCallback {
    f.into()
  }
  let _ = pass_as_impl_macro(mock_host_import_module_dynamically_callback);
  let _ = pass_as_impl_macro(|_scope, _referrer, _specifier| unimplemented!());
}

/// HostInitializeImportMetaObjectCallback is called the first time import.meta
/// is accessed for a module. Subsequent access will reuse the same value.
///
/// The method combines two implementation-defined abstract operations into one:
/// HostGetImportMetaProperties and HostFinalizeImportMeta.
///
/// The embedder should use v8::Object::CreateDataProperty to add properties on
/// the meta object.
///
/// # Example
///
/// ```
/// # use rusty_v8 as v8;
/// #
/// fn host_initialize_import_meta_object_callback_example<'s>(
///   scope: &mut v8::HandleScope<'s>,
///   module: v8::Local<'s, v8::Module>,
///   meta: v8::Local<'s, v8::Object>,
/// ) {
///   todo!();
/// }
/// ```
pub trait HostInitializeImportMetaObjectCallback:
  UnitType
  + for<'s> FnOnce(&mut HandleScope<'s>, Local<'s, Module>, Local<'s, Object>)
{
}

impl<F> HostInitializeImportMetaObjectCallback for F where
  F: UnitType
    + for<'s> FnOnce(&mut HandleScope<'s>, Local<'s, Module>, Local<'s, Object>)
{
}

#[macro_export]
macro_rules! impl_host_initialize_import_meta_object_callback {
  () => {
    impl $crate::HostInitializeImportMetaObjectCallback
    + for<'__s> ::std::ops::FnOnce(
      &mut $crate::HandleScope<'__s>,
      $crate::Local<'__s, $crate::Module>,
      $crate::Local<'__s, $crate::Object>,
    )
  };
}

#[repr(transparent)]
pub(crate) struct RawHostInitializeImportMetaObjectCallback(
  for<'s> extern "C" fn(
    Local<'s, Context>,
    Local<'s, Module>,
    Local<'s, Object>,
  ),
);

impl<F: HostInitializeImportMetaObjectCallback> From<F>
  for RawHostInitializeImportMetaObjectCallback
{
  fn from(_: F) -> Self {
    #[inline(always)]
    extern "C" fn adapter<'s, F: HostInitializeImportMetaObjectCallback>(
      context: Local<'s, Context>,
      module: Local<'s, Module>,
      meta: Local<'s, Object>,
    ) {
      let scope = &mut unsafe { CallbackScope::new(context) };
      (F::get())(scope, module, meta)
    }

    Self(adapter::<F>)
  }
}

#[cfg(test)]
fn mock_host_initialize_import_meta_object_callback<'s>(
  _scope: &mut HandleScope<'s>,
  _module: Local<'s, Module>,
  _meta: Local<'s, Object>,
) {
  unimplemented!()
}

#[test]
fn host_initialize_import_meta_object_callback_as_type_param() {
  fn pass_as_type_param<F: HostInitializeImportMetaObjectCallback>(
    _: F,
  ) -> RawHostInitializeImportMetaObjectCallback {
    F::get().into()
  }
  let _ = pass_as_type_param(mock_host_initialize_import_meta_object_callback);
}

#[test]
fn host_initialize_import_meta_object_callback_as_impl_trait() {
  fn pass_as_impl_trait(
    f: impl HostInitializeImportMetaObjectCallback,
  ) -> RawHostInitializeImportMetaObjectCallback {
    f.into()
  }
  let _ = pass_as_impl_trait(mock_host_initialize_import_meta_object_callback);
}

#[test]
fn host_initialize_import_meta_object_callback_as_impl_macro() {
  fn pass_as_impl_macro(
    f: impl_host_initialize_import_meta_object_callback!(),
  ) -> RawHostInitializeImportMetaObjectCallback {
    f.into()
  }
  let _ = pass_as_impl_macro(mock_host_initialize_import_meta_object_callback);
  let _ = pass_as_impl_macro(|_scope, _module, _meta| unimplemented!());
}

/// PrepareStackTraceCallback is called when the stack property of an error is
/// first accessed. The return value will be used as the stack value. If this
/// callback is registed, the `Error.prepareStackTrace` API will be disabled.
/// `sites` is an array of call sites, specified in
/// https://v8.dev/docs/stack-trace-api
///
/// # Example
///
/// ```
/// # use rusty_v8 as v8;
/// #
/// fn prepare_stack_trace_callback_example<'s>(
///   scope: &mut v8::HandleScope<'s>,
///   error: v8::Local<'s, v8::Value>,
///   sites: v8::Local<'s, v8::Array>,
/// ) -> Option<v8::Local<'s, v8::Value>> {
///   todo!()
/// }
/// ```
pub trait PrepareStackTraceCallback:
  UnitType
  + for<'s> FnOnce(
    &mut HandleScope<'s>,
    Local<'s, Value>,
    Local<'s, Array>,
  ) -> Option<Local<'s, Value>>
{
}

impl<F> PrepareStackTraceCallback for F where
  F: UnitType
    + for<'s> FnOnce(
      &mut HandleScope<'s>,
      Local<'s, Value>,
      Local<'s, Array>,
    ) -> Option<Local<'s, Value>>
{
}

#[macro_export]
macro_rules! impl_prepare_stack_trace_callback {
  () => {
    impl $crate::PrepareStackTraceCallback
    + for<'__s> ::std::ops::FnOnce(
      &mut $crate::HandleScope<'__s>,
      $crate::Local<'__s, $crate::Value>,
      $crate::Local<'__s, $crate::Array>,
    ) -> ::std::option::Option<$crate::Local<'__s, $crate::Value>>
  };
}

#[cfg(target_family = "unix")]
#[repr(transparent)]
pub(crate) struct RawPrepareStackTraceCallback(
  for<'s> extern "C" fn(
    Local<'s, Context>,
    Local<'s, Value>,
    Local<'s, Array>,
  ) -> Option<Local<'s, Value>>,
);

#[cfg(all(target_family = "windows", target_arch = "x86_64"))]
#[repr(transparent)]
pub(crate) struct RawPrepareStackTraceCallback(
  for<'s> extern "C" fn(
    *mut Option<Local<'s, Value>>,
    Local<'s, Context>,
    Local<'s, Value>,
    Local<'s, Array>,
  ) -> *mut Option<Local<'s, Value>>,
);

impl<F: PrepareStackTraceCallback> From<F> for RawPrepareStackTraceCallback {
  fn from(_: F) -> Self {
    #[inline(always)]
    fn signature_adapter<'s, F: PrepareStackTraceCallback>(
      context: Local<'s, Context>,
      error: Local<'s, Value>,
      sites: Local<'s, Array>,
    ) -> Option<Local<'s, Value>> {
      let scope = &mut unsafe { CallbackScope::new(context) };
      (F::get())(scope, error, sites)
    }

    #[cfg(target_family = "unix")]
    #[inline(always)]
    extern "C" fn abi_adapter<'s, F: PrepareStackTraceCallback>(
      context: Local<'s, Context>,
      error: Local<'s, Value>,
      sites: Local<'s, Array>,
    ) -> Option<Local<'s, Value>> {
      signature_adapter::<F>(context, error, sites)
    }

    #[cfg(all(target_family = "windows", target_arch = "x86_64"))]
    #[inline(always)]
    extern "C" fn abi_adapter<'s, F: PrepareStackTraceCallback>(
      return_value: *mut Option<Local<'s, Value>>,
      context: Local<'s, Context>,
      error: Local<'s, Value>,
      sites: Local<'s, Array>,
    ) -> *mut Option<Local<'s, Value>> {
      unsafe {
        std::ptr::write(
          return_value,
          signature_adapter::<F>(context, error, sites),
        );
        return_value
      }
    }

    Self(abi_adapter::<F>)
  }
}

#[cfg(test)]
fn mock_prepare_stack_trace_callback<'s>(
  _scope: &mut HandleScope<'s>,
  _error: Local<'s, Value>,
  _sites: Local<'s, Array>,
) -> Option<Local<'s, Value>> {
  unimplemented!()
}

#[test]
fn prepare_stack_trace_callback_as_type_param() {
  fn pass_as_type_param<F: PrepareStackTraceCallback>(
    _: F,
  ) -> RawPrepareStackTraceCallback {
    F::get().into()
  }
  let _ = pass_as_type_param(mock_prepare_stack_trace_callback);
}

#[test]
fn prepare_stack_trace_callback_as_impl_trait() {
  fn pass_as_impl_trait(
    f: impl PrepareStackTraceCallback,
  ) -> RawPrepareStackTraceCallback {
    f.into()
  }
  let _ = pass_as_impl_trait(mock_prepare_stack_trace_callback);
}

#[test]
fn prepare_stack_trace_callback_as_impl_macro() {
  fn pass_as_impl_macro(
    f: impl_prepare_stack_trace_callback!(),
  ) -> RawPrepareStackTraceCallback {
    f.into()
  }
  let _ = pass_as_impl_macro(mock_prepare_stack_trace_callback);
  let _ = pass_as_impl_macro(|_scope, _error, _sites| unimplemented!());
}

// === PromiseHook ===

/// # Example
///
/// ```
/// # use rusty_v8 as v8;
/// #
/// fn promise_hook_example<'s>(
///   scope: &mut v8::HandleScope<'s>,
///   r#type: v8::PromiseHookType,
///   promise: v8::Local<'s, v8::Promise>,
///   parent: v8::Local<'s, v8::Value>,
/// ) {
///   todo!();
/// }
/// ```
pub trait PromiseHook:
  UnitType
  + for<'s> FnOnce(
    &mut HandleScope<'s>,
    PromiseHookType,
    Local<'s, Promise>,
    Local<'s, Value>,
  )
{
}

impl<F> PromiseHook for F where
  F: UnitType
    + for<'s> FnOnce(
      &mut HandleScope<'s>,
      PromiseHookType,
      Local<'s, Promise>,
      Local<'s, Value>,
    )
{
}

#[macro_export]
macro_rules! impl_promise_hook {
  () => {
    impl $crate::PromiseHook
    + for<'__s> ::std::ops::FnOnce(
      &mut $crate::HandleScope<'__s>,
      $crate::PromiseHookType,
      $crate::Local<'__s, $crate::Promise>,
      $crate::Local<'__s, $crate::Value>,
    )
  };
}

#[repr(transparent)]
pub(crate) struct RawPromiseHook(
  for<'s> extern "C" fn(PromiseHookType, Local<'s, Promise>, Local<'s, Value>),
);

impl<F: PromiseHook> From<F> for RawPromiseHook {
  fn from(_: F) -> Self {
    #[inline(always)]
    extern "C" fn adapter<'s, F: PromiseHook>(
      r#type: PromiseHookType,
      promise: Local<'s, Promise>,
      parent: Local<'s, Value>,
    ) {
      let scope = &mut unsafe { CallbackScope::new(promise) };
      (F::get())(scope, r#type, promise, parent)
    }

    Self(adapter::<F>)
  }
}

#[cfg(test)]
fn mock_promise_hook<'s>(
  _scope: &mut HandleScope<'s>,
  _type: PromiseHookType,
  _promise: Local<'s, Promise>,
  _parent: Local<'s, Value>,
) {
  unimplemented!()
}

#[test]
fn promise_hook_as_type_param() {
  fn pass_as_type_param<F: PromiseHook>(_: F) -> RawPromiseHook {
    F::get().into()
  }
  let _ = pass_as_type_param(mock_promise_hook);
}

#[test]
fn promise_hook_as_impl_trait() {
  fn pass_as_impl_trait(f: impl PromiseHook) -> RawPromiseHook {
    f.into()
  }
  let _ = pass_as_impl_trait(mock_promise_hook);
}

#[test]
fn promise_hook_as_impl_macro() {
  fn pass_as_impl_macro(f: impl_promise_hook!()) -> RawPromiseHook {
    f.into()
  }
  let _ = pass_as_impl_macro(mock_promise_hook);
  let _ =
    pass_as_impl_macro(|_scope, _type, _promise, _parent| unimplemented!());
}

// === PromiseRejectCallback ===

/// # Example
///
/// ```
/// # use rusty_v8 as v8;
/// #
/// fn promise_reject_callback_example<'s>(
///   scope: &mut v8::HandleScope<'s>,
///   message: v8::PromiseRejectMessage<'s>,
/// ) {
///   todo!();
/// }
/// ```
pub trait PromiseRejectCallback:
  UnitType + for<'s> FnOnce(&mut HandleScope<'s>, PromiseRejectMessage<'s>)
{
}

impl<F> PromiseRejectCallback for F where
  F: UnitType + for<'s> FnOnce(&mut HandleScope<'s>, PromiseRejectMessage<'s>)
{
}

#[macro_export]
macro_rules! impl_promise_reject_callback {
  () => {
    impl $crate::PromiseRejectCallback
    + for<'__s> ::std::ops::FnOnce(
      &mut $crate::HandleScope<'__s>,
      $crate::PromiseRejectMessage<'__s>,
    )
  };
}

#[repr(transparent)]
pub(crate) struct RawPromiseRejectCallback(
  for<'s> extern "C" fn(PromiseRejectMessage<'s>),
);

impl<F: PromiseRejectCallback> From<F> for RawPromiseRejectCallback {
  fn from(_: F) -> Self {
    #[inline(always)]
    extern "C" fn adapter<'s, F: PromiseRejectCallback>(
      message: PromiseRejectMessage<'s>,
    ) {
      let scope = &mut unsafe { CallbackScope::new(&message) };
      (F::get())(scope, message)
    }

    Self(adapter::<F>)
  }
}

#[cfg(test)]
fn mock_promise_reject_callback<'s>(
  _scope: &mut HandleScope<'s>,
  _message: PromiseRejectMessage<'s>,
) {
  unimplemented!()
}

#[test]
fn promise_reject_callback_as_type_param() {
  fn pass_as_type_param<F: PromiseRejectCallback>(
    _: F,
  ) -> RawPromiseRejectCallback {
    F::get().into()
  }
  let _ = pass_as_type_param(mock_promise_reject_callback);
}

#[test]
fn promise_reject_callback_as_impl_trait() {
  fn pass_as_impl_trait(
    f: impl PromiseRejectCallback,
  ) -> RawPromiseRejectCallback {
    f.into()
  }
  let _ = pass_as_impl_trait(mock_promise_reject_callback);
}

#[test]
fn promise_reject_callback_as_impl_macro() {
  fn pass_as_impl_macro(
    f: impl_promise_reject_callback!(),
  ) -> RawPromiseRejectCallback {
    f.into()
  }
  let _ = pass_as_impl_macro(mock_promise_reject_callback);
  let _ = pass_as_impl_macro(|_scope, _message| unimplemented!());
}

// === MicrotasksCompletedCallback ===

/// # Example
///
/// ```
/// # use rusty_v8 as v8;
/// #
/// fn microtasks_completed_callback_example(
///   isolate: &mut v8::Isolate,
///   callback_data: *mut (),
/// ) {
///   todo!();
/// }
/// ```
pub trait MicrotasksCompletedCallback:
  UnitType + FnOnce(&mut Isolate, *mut ())
{
}

impl<F> MicrotasksCompletedCallback for F where
  F: UnitType + FnOnce(&mut Isolate, *mut ())
{
}

#[macro_export]
macro_rules! impl_microtasks_completed_callback {
  () => {
    impl $crate::MicrotasksCompletedCallback
    + ::std::ops::FnOnce(
      &mut $crate::Isolate,
      *mut (),
    )
  };
}

#[repr(transparent)]
pub(crate) struct RawMicrotasksCompletedCallback(
  extern "C" fn(*mut Isolate, *mut c_void),
);

impl<F: MicrotasksCompletedCallback> From<F>
  for RawMicrotasksCompletedCallback
{
  fn from(_: F) -> Self {
    #[inline(always)]
    extern "C" fn adapter<F: MicrotasksCompletedCallback>(
      isolate: *mut Isolate,
      callback_data: *mut c_void,
    ) {
      let isolate = unsafe { &mut *isolate };
      let callback_data = callback_data as *mut ();
      (F::get())(isolate, callback_data)
    }

    Self(adapter::<F>)
  }
}

#[cfg(test)]
fn mock_microtasks_completed_callback(
  _isolate: &mut Isolate,
  _callback_data: *mut (),
) {
  unimplemented!()
}

#[test]
fn microtasks_completed_callback_as_type_param() {
  fn pass_as_type_param<F: MicrotasksCompletedCallback>(
    _: F,
  ) -> RawMicrotasksCompletedCallback {
    F::get().into()
  }
  let _ = pass_as_type_param(mock_microtasks_completed_callback);
}

#[test]
fn microtasks_completed_callback_as_impl_trait() {
  fn pass_as_impl_trait(
    f: impl MicrotasksCompletedCallback,
  ) -> RawMicrotasksCompletedCallback {
    f.into()
  }
  let _ = pass_as_impl_trait(mock_microtasks_completed_callback);
}

#[test]
fn microtasks_completed_callback_as_impl_macro() {
  fn pass_as_impl_macro(
    f: impl_microtasks_completed_callback!(),
  ) -> RawMicrotasksCompletedCallback {
    f.into()
  }
  let _ = pass_as_impl_macro(mock_microtasks_completed_callback);
  let _ = pass_as_impl_macro(|_isolate, _callback_data| unimplemented!());
}

// === MicrotaskCallback ===

/// # Example
///
/// ```
/// fn microtask_callback_example(data: *mut ()) {
///   todo!();
/// }
/// ```
pub trait MicrotaskCallback: UnitType + FnOnce(*mut ()) {}

impl<F> MicrotaskCallback for F where F: UnitType + FnOnce(*mut ()) {}

#[macro_export]
macro_rules! impl_microtask_callback {
  () => {
    impl $crate::MicrotaskCallback
    + ::std::ops::FnOnce(
      *mut (),
    )
  };
}

#[repr(transparent)]
pub(crate) struct RawMicrotaskCallback(extern "C" fn(*mut c_void));

impl<F: MicrotaskCallback> From<F> for RawMicrotaskCallback {
  fn from(_: F) -> Self {
    #[inline(always)]
    extern "C" fn adapter<F: MicrotaskCallback>(data: *mut c_void) {
      let data = data as *mut ();
      (F::get())(data)
    }

    Self(adapter::<F>)
  }
}

#[cfg(test)]
fn mock_microtask_callback(_data: *mut ()) {
  unimplemented!()
}

#[test]
fn microtask_callback_as_type_param() {
  fn pass_as_type_param<F: MicrotaskCallback>(_: F) -> RawMicrotaskCallback {
    F::get().into()
  }
  let _ = pass_as_type_param(mock_microtask_callback);
}

#[test]
fn microtask_callback_as_impl_trait() {
  fn pass_as_impl_trait(f: impl MicrotaskCallback) -> RawMicrotaskCallback {
    f.into()
  }
  let _ = pass_as_impl_trait(mock_microtask_callback);
}

#[test]
fn microtask_callback_as_impl_macro() {
  fn pass_as_impl_macro(f: impl_microtask_callback!()) -> RawMicrotaskCallback {
    f.into()
  }
  let _ = pass_as_impl_macro(mock_microtask_callback);
  let _ = pass_as_impl_macro(|_data| unimplemented!());
}

// === FailedAccessCheckCallback ===

/// # Example
///
/// ```
/// # use rusty_v8 as v8;
/// #
/// fn failed_access_check_callback_example<'s>(
///   scope: &mut v8::HandleScope<'s>,
///   target: v8::Local<'s, v8::Object>,
///   r#type: v8::AccessType,
///   data: v8::Local<'s, v8::Value>,
/// ) {
///   todo!();
/// }
/// ```
pub trait FailedAccessCheckCallback:
  UnitType
  + for<'s> FnOnce(
    &mut HandleScope<'s>,
    Local<'s, Object>,
    AccessType,
    Local<'s, Value>,
  )
{
}

impl<F> FailedAccessCheckCallback for F where
  F: UnitType
    + for<'s> FnOnce(
      &mut HandleScope<'s>,
      Local<'s, Object>,
      AccessType,
      Local<'s, Value>,
    )
{
}

#[macro_export]
macro_rules! impl_failed_access_check_callback {
  () => {
    impl $crate::FailedAccessCheckCallback
    + for<'__s> ::std::ops::FnOnce(
      &mut $crate::HandleScope<'__s>,
      $crate::Local<'__s, $crate::Object>,
      $crate::AccessType,
      $crate::Local<'__s, $crate::Value>,
    )
  };
}

#[repr(transparent)]
pub(crate) struct RawFailedAccessCheckCallback(
  for<'s> extern "C" fn(Local<'s, Object>, AccessType, Local<'s, Value>),
);

impl<F: FailedAccessCheckCallback> From<F> for RawFailedAccessCheckCallback {
  fn from(_: F) -> Self {
    #[inline(always)]
    extern "C" fn adapter<'s, F: FailedAccessCheckCallback>(
      target: Local<'s, Object>,
      r#type: AccessType,
      data: Local<'s, Value>,
    ) {
      let scope = &mut unsafe { CallbackScope::new(target) };
      (F::get())(scope, target, r#type, data)
    }

    Self(adapter::<F>)
  }
}

#[cfg(test)]
fn mock_failed_access_check_callback<'s>(
  _scope: &mut HandleScope<'s>,
  _target: Local<'s, Object>,
  _type: AccessType,
  _data: Local<'s, Value>,
) {
  unimplemented!()
}

#[test]
fn failed_access_check_callback_as_type_param() {
  fn pass_as_type_param<F: FailedAccessCheckCallback>(
    _: F,
  ) -> RawFailedAccessCheckCallback {
    F::get().into()
  }
  let _ = pass_as_type_param(mock_failed_access_check_callback);
}

#[test]
fn failed_access_check_callback_as_impl_trait() {
  fn pass_as_impl_trait(
    f: impl FailedAccessCheckCallback,
  ) -> RawFailedAccessCheckCallback {
    f.into()
  }
  let _ = pass_as_impl_trait(mock_failed_access_check_callback);
}

#[test]
fn failed_access_check_callback_as_impl_macro() {
  fn pass_as_impl_macro(
    f: impl_failed_access_check_callback!(),
  ) -> RawFailedAccessCheckCallback {
    f.into()
  }
  let _ = pass_as_impl_macro(mock_failed_access_check_callback);
  let _ = pass_as_impl_macro(|_scope, _target, _type, _data| unimplemented!());
}

/// Callback to check if code generation from strings is allowed. See
/// Context::AllowCodeGenerationFromStrings.
///
/// # Example
///
/// ```
/// # use rusty_v8 as v8;
/// #
/// fn allow_code_generation_from_strings_callback_example<'s>(
///   scope: &mut v8::HandleScope<'s>,
///   source: v8::Local<'s, v8::String>,
/// ) -> bool {
///   todo!()
/// }
/// ```
pub trait AllowCodeGenerationFromStringsCallback:
  UnitType + for<'s> FnOnce(&mut HandleScope<'s>, Local<'s, String>) -> bool
{
}

impl<F> AllowCodeGenerationFromStringsCallback for F where
  F: UnitType + for<'s> FnOnce(&mut HandleScope<'s>, Local<'s, String>) -> bool
{
}

#[macro_export]
macro_rules! impl_allow_code_generation_from_strings_callback {
  () => {
    impl $crate::AllowCodeGenerationFromStringsCallback
    + for<'__s> ::std::ops::FnOnce(
      &mut $crate::HandleScope<'__s>,
      $crate::Local<'__s, $crate::String>,
    ) -> bool
  };
}

#[repr(transparent)]
pub(crate) struct RawAllowCodeGenerationFromStringsCallback(
  for<'s> extern "C" fn(Local<'s, Context>, Local<'s, String>) -> bool,
);

impl<F: AllowCodeGenerationFromStringsCallback> From<F>
  for RawAllowCodeGenerationFromStringsCallback
{
  fn from(_: F) -> Self {
    #[inline(always)]
    extern "C" fn adapter<'s, F: AllowCodeGenerationFromStringsCallback>(
      context: Local<'s, Context>,
      source: Local<'s, String>,
    ) -> bool {
      let scope = &mut unsafe { CallbackScope::new(context) };
      (F::get())(scope, source)
    }

    Self(adapter::<F>)
  }
}

#[cfg(test)]
fn mock_allow_code_generation_from_strings_callback<'s>(
  _scope: &mut HandleScope<'s>,
  _source: Local<'s, String>,
) -> bool {
  unimplemented!()
}

#[test]
fn allow_code_generation_from_strings_callback_as_type_param() {
  fn pass_as_type_param<F: AllowCodeGenerationFromStringsCallback>(
    _: F,
  ) -> RawAllowCodeGenerationFromStringsCallback {
    F::get().into()
  }
  let _ = pass_as_type_param(mock_allow_code_generation_from_strings_callback);
}

#[test]
fn allow_code_generation_from_strings_callback_as_impl_trait() {
  fn pass_as_impl_trait(
    f: impl AllowCodeGenerationFromStringsCallback,
  ) -> RawAllowCodeGenerationFromStringsCallback {
    f.into()
  }
  let _ = pass_as_impl_trait(mock_allow_code_generation_from_strings_callback);
}

#[test]
fn allow_code_generation_from_strings_callback_as_impl_macro() {
  fn pass_as_impl_macro(
    f: impl_allow_code_generation_from_strings_callback!(),
  ) -> RawAllowCodeGenerationFromStringsCallback {
    f.into()
  }
  let _ = pass_as_impl_macro(mock_allow_code_generation_from_strings_callback);
  let _ = pass_as_impl_macro(|_scope, _source| unimplemented!());
}

/// Callback to check if codegen is allowed from a source object, and convert
/// the source to string if necessary.See  ModifyCodeGenerationFromStrings.
///
/// # Example
///
/// ```
/// # use rusty_v8 as v8;
/// #
/// fn modify_code_generation_from_strings_callback_example<'s>(
///   scope: &mut v8::HandleScope<'s>,
///   source: v8::Local<'s, v8::Value>,
/// ) -> v8::ModifyCodeGenerationFromStringsResult<'s> {
///   todo!()
/// }
/// ```
pub trait ModifyCodeGenerationFromStringsCallback:
  UnitType
  + for<'s> FnOnce(
    &mut HandleScope<'s>,
    Local<'s, Value>,
  ) -> ModifyCodeGenerationFromStringsResult<'s>
{
}

impl<F> ModifyCodeGenerationFromStringsCallback for F where
  F: UnitType
    + for<'s> FnOnce(
      &mut HandleScope<'s>,
      Local<'s, Value>,
    ) -> ModifyCodeGenerationFromStringsResult<'s>
{
}

#[macro_export]
macro_rules! impl_modify_code_generation_from_strings_callback {
  () => {
    impl $crate::ModifyCodeGenerationFromStringsCallback
    + for<'__s> ::std::ops::FnOnce(
      &mut $crate::HandleScope<'__s>,
      $crate::Local<'__s, $crate::Value>,
    ) -> $crate::ModifyCodeGenerationFromStringsResult<'__s>
  };
}

#[cfg(target_family = "unix")]
#[repr(transparent)]
pub(crate) struct RawModifyCodeGenerationFromStringsCallback(
  for<'s> extern "C" fn(
    Local<'s, Context>,
    Local<'s, Value>,
  ) -> ModifyCodeGenerationFromStringsResult<'s>,
);

#[cfg(all(target_family = "windows", target_arch = "x86_64"))]
#[repr(transparent)]
pub(crate) struct RawModifyCodeGenerationFromStringsCallback(
  for<'s> extern "C" fn(
    *mut ModifyCodeGenerationFromStringsResult<'s>,
    Local<'s, Context>,
    Local<'s, Value>,
  ) -> *mut ModifyCodeGenerationFromStringsResult<'s>,
);

impl<F: ModifyCodeGenerationFromStringsCallback> From<F>
  for RawModifyCodeGenerationFromStringsCallback
{
  fn from(_: F) -> Self {
    #[inline(always)]
    fn signature_adapter<'s, F: ModifyCodeGenerationFromStringsCallback>(
      context: Local<'s, Context>,
      source: Local<'s, Value>,
    ) -> ModifyCodeGenerationFromStringsResult<'s> {
      let scope = &mut unsafe { CallbackScope::new(context) };
      (F::get())(scope, source)
    }

    #[cfg(target_family = "unix")]
    #[inline(always)]
    extern "C" fn abi_adapter<
      's,
      F: ModifyCodeGenerationFromStringsCallback,
    >(
      context: Local<'s, Context>,
      source: Local<'s, Value>,
    ) -> ModifyCodeGenerationFromStringsResult<'s> {
      signature_adapter::<F>(context, source)
    }

    #[cfg(all(target_family = "windows", target_arch = "x86_64"))]
    #[inline(always)]
    extern "C" fn abi_adapter<
      's,
      F: ModifyCodeGenerationFromStringsCallback,
    >(
      return_value: *mut ModifyCodeGenerationFromStringsResult<'s>,
      context: Local<'s, Context>,
      source: Local<'s, Value>,
    ) -> *mut ModifyCodeGenerationFromStringsResult<'s> {
      unsafe {
        std::ptr::write(return_value, signature_adapter::<F>(context, source));
        return_value
      }
    }

    Self(abi_adapter::<F>)
  }
}

#[cfg(test)]
fn mock_modify_code_generation_from_strings_callback<'s>(
  _scope: &mut HandleScope<'s>,
  _source: Local<'s, Value>,
) -> ModifyCodeGenerationFromStringsResult<'s> {
  unimplemented!()
}

#[test]
fn modify_code_generation_from_strings_callback_as_type_param() {
  fn pass_as_type_param<F: ModifyCodeGenerationFromStringsCallback>(
    _: F,
  ) -> RawModifyCodeGenerationFromStringsCallback {
    F::get().into()
  }
  let _ = pass_as_type_param(mock_modify_code_generation_from_strings_callback);
}

#[test]
fn modify_code_generation_from_strings_callback_as_impl_trait() {
  fn pass_as_impl_trait(
    f: impl ModifyCodeGenerationFromStringsCallback,
  ) -> RawModifyCodeGenerationFromStringsCallback {
    f.into()
  }
  let _ = pass_as_impl_trait(mock_modify_code_generation_from_strings_callback);
}

#[test]
fn modify_code_generation_from_strings_callback_as_impl_macro() {
  fn pass_as_impl_macro(
    f: impl_modify_code_generation_from_strings_callback!(),
  ) -> RawModifyCodeGenerationFromStringsCallback {
    f.into()
  }
  let _ = pass_as_impl_macro(mock_modify_code_generation_from_strings_callback);
  let _ = pass_as_impl_macro(|_scope, _source| unimplemented!());
}

// === ExtensionCallback ===

/// # Example
///
/// ```
/// # use rusty_v8 as v8;
/// #
/// fn extension_callback_example<'s>(
///   scope: &mut v8::HandleScope<'s>,
///   arguments: v8::FunctionCallbackArguments<'s>,
///   return_value: v8::ReturnValue<'s, v8::Value>,
/// ) -> bool {
///   todo!()
/// }
/// ```
pub trait ExtensionCallback:
  UnitType
  + for<'s> FnOnce(
    &mut HandleScope<'s>,
    FunctionCallbackArguments<'s>,
    ReturnValue<'s, Value>,
  ) -> bool
{
}

impl<F> ExtensionCallback for F where
  F: UnitType
    + for<'s> FnOnce(
      &mut HandleScope<'s>,
      FunctionCallbackArguments<'s>,
      ReturnValue<'s, Value>,
    ) -> bool
{
}

#[macro_export]
macro_rules! impl_extension_callback {
  () => {
    impl $crate::ExtensionCallback
    + for<'__s> ::std::ops::FnOnce(
      &mut $crate::HandleScope<'__s>,
      $crate::FunctionCallbackArguments<'__s>,
      $crate::ReturnValue<'__s, $crate::Value>,
    ) -> bool
  };
}

#[repr(transparent)]
pub(crate) struct RawExtensionCallback(
  extern "C" fn(*const FunctionCallbackInfo) -> bool,
);

impl<F: ExtensionCallback> From<F> for RawExtensionCallback {
  fn from(_: F) -> Self {
    #[inline(always)]
    extern "C" fn adapter<F: ExtensionCallback>(
      info: *const FunctionCallbackInfo,
    ) -> bool {
      let scope = &mut unsafe { CallbackScope::new(&*info) };
      let arguments =
        FunctionCallbackArguments::from_function_callback_info(info);
      let return_value =
        ReturnValue::<Value>::from_function_callback_info(info);
      (F::get())(scope, arguments, return_value)
    }

    Self(adapter::<F>)
  }
}

#[cfg(test)]
fn mock_extension_callback<'s>(
  _scope: &mut HandleScope<'s>,
  _arguments: FunctionCallbackArguments<'s>,
  _return_value: ReturnValue<'s, Value>,
) -> bool {
  unimplemented!()
}

#[test]
fn extension_callback_as_type_param() {
  fn pass_as_type_param<F: ExtensionCallback>(_: F) -> RawExtensionCallback {
    F::get().into()
  }
  let _ = pass_as_type_param(mock_extension_callback);
}

#[test]
fn extension_callback_as_impl_trait() {
  fn pass_as_impl_trait(f: impl ExtensionCallback) -> RawExtensionCallback {
    f.into()
  }
  let _ = pass_as_impl_trait(mock_extension_callback);
}

#[test]
fn extension_callback_as_impl_macro() {
  fn pass_as_impl_macro(f: impl_extension_callback!()) -> RawExtensionCallback {
    f.into()
  }
  let _ = pass_as_impl_macro(mock_extension_callback);
  let _ =
    pass_as_impl_macro(|_scope, _arguments, _return_value| unimplemented!());
}

// === AllowWasmCodeGenerationCallback ===

/// # Example
///
/// ```
/// # use rusty_v8 as v8;
/// #
/// fn allow_wasm_code_generation_callback_example<'s>(
///   scope: &mut v8::HandleScope<'s>,
///   source: v8::Local<'s, v8::String>,
/// ) -> bool {
///   todo!()
/// }
/// ```
pub trait AllowWasmCodeGenerationCallback:
  UnitType + for<'s> FnOnce(&mut HandleScope<'s>, Local<'s, String>) -> bool
{
}

impl<F> AllowWasmCodeGenerationCallback for F where
  F: UnitType + for<'s> FnOnce(&mut HandleScope<'s>, Local<'s, String>) -> bool
{
}

#[macro_export]
macro_rules! impl_allow_wasm_code_generation_callback {
  () => {
    impl $crate::AllowWasmCodeGenerationCallback
    + for<'__s> ::std::ops::FnOnce(
      &mut $crate::HandleScope<'__s>,
      $crate::Local<'__s, $crate::String>,
    ) -> bool
  };
}

#[repr(transparent)]
pub(crate) struct RawAllowWasmCodeGenerationCallback(
  for<'s> extern "C" fn(Local<'s, Context>, Local<'s, String>) -> bool,
);

impl<F: AllowWasmCodeGenerationCallback> From<F>
  for RawAllowWasmCodeGenerationCallback
{
  fn from(_: F) -> Self {
    #[inline(always)]
    extern "C" fn adapter<'s, F: AllowWasmCodeGenerationCallback>(
      context: Local<'s, Context>,
      source: Local<'s, String>,
    ) -> bool {
      let scope = &mut unsafe { CallbackScope::new(context) };
      (F::get())(scope, source)
    }

    Self(adapter::<F>)
  }
}

#[cfg(test)]
fn mock_allow_wasm_code_generation_callback<'s>(
  _scope: &mut HandleScope<'s>,
  _source: Local<'s, String>,
) -> bool {
  unimplemented!()
}

#[test]
fn allow_wasm_code_generation_callback_as_type_param() {
  fn pass_as_type_param<F: AllowWasmCodeGenerationCallback>(
    _: F,
  ) -> RawAllowWasmCodeGenerationCallback {
    F::get().into()
  }
  let _ = pass_as_type_param(mock_allow_wasm_code_generation_callback);
}

#[test]
fn allow_wasm_code_generation_callback_as_impl_trait() {
  fn pass_as_impl_trait(
    f: impl AllowWasmCodeGenerationCallback,
  ) -> RawAllowWasmCodeGenerationCallback {
    f.into()
  }
  let _ = pass_as_impl_trait(mock_allow_wasm_code_generation_callback);
}

#[test]
fn allow_wasm_code_generation_callback_as_impl_macro() {
  fn pass_as_impl_macro(
    f: impl_allow_wasm_code_generation_callback!(),
  ) -> RawAllowWasmCodeGenerationCallback {
    f.into()
  }
  let _ = pass_as_impl_macro(mock_allow_wasm_code_generation_callback);
  let _ = pass_as_impl_macro(|_scope, _source| unimplemented!());
}

// === ApiImplementationCallback ===

/// # Example
///
/// ```
/// # use rusty_v8 as v8;
/// #
/// fn api_implementation_callback_example<'s>(
///   scope: &mut v8::HandleScope<'s>,
///   arguments: v8::FunctionCallbackArguments<'s>,
///   return_value: v8::ReturnValue<'s, v8::Value>,
/// ) {
///   todo!();
/// }
/// ```
pub trait ApiImplementationCallback:
  UnitType
  + for<'s> FnOnce(
    &mut HandleScope<'s>,
    FunctionCallbackArguments<'s>,
    ReturnValue<'s, Value>,
  )
{
}

impl<F> ApiImplementationCallback for F where
  F: UnitType
    + for<'s> FnOnce(
      &mut HandleScope<'s>,
      FunctionCallbackArguments<'s>,
      ReturnValue<'s, Value>,
    )
{
}

#[macro_export]
macro_rules! impl_api_implementation_callback {
  () => {
    impl $crate::ApiImplementationCallback
    + for<'__s> ::std::ops::FnOnce(
      &mut $crate::HandleScope<'__s>,
      $crate::FunctionCallbackArguments<'__s>,
      $crate::ReturnValue<'__s, $crate::Value>,
    )
  };
}

#[repr(transparent)]
pub(crate) struct RawApiImplementationCallback(
  extern "C" fn(*const FunctionCallbackInfo),
);

impl<F: ApiImplementationCallback> From<F> for RawApiImplementationCallback {
  fn from(_: F) -> Self {
    #[inline(always)]
    extern "C" fn adapter<F: ApiImplementationCallback>(
      info: *const FunctionCallbackInfo,
    ) {
      let scope = &mut unsafe { CallbackScope::new(&*info) };
      let arguments =
        FunctionCallbackArguments::from_function_callback_info(info);
      let return_value =
        ReturnValue::<Value>::from_function_callback_info(info);
      (F::get())(scope, arguments, return_value)
    }

    Self(adapter::<F>)
  }
}

#[cfg(test)]
fn mock_api_implementation_callback<'s>(
  _scope: &mut HandleScope<'s>,
  _arguments: FunctionCallbackArguments<'s>,
  _return_value: ReturnValue<'s, Value>,
) {
  unimplemented!()
}

#[test]
fn api_implementation_callback_as_type_param() {
  fn pass_as_type_param<F: ApiImplementationCallback>(
    _: F,
  ) -> RawApiImplementationCallback {
    F::get().into()
  }
  let _ = pass_as_type_param(mock_api_implementation_callback);
}

#[test]
fn api_implementation_callback_as_impl_trait() {
  fn pass_as_impl_trait(
    f: impl ApiImplementationCallback,
  ) -> RawApiImplementationCallback {
    f.into()
  }
  let _ = pass_as_impl_trait(mock_api_implementation_callback);
}

#[test]
fn api_implementation_callback_as_impl_macro() {
  fn pass_as_impl_macro(
    f: impl_api_implementation_callback!(),
  ) -> RawApiImplementationCallback {
    f.into()
  }
  let _ = pass_as_impl_macro(mock_api_implementation_callback);
  let _ =
    pass_as_impl_macro(|_scope, _arguments, _return_value| unimplemented!());
}

// === WasmStreamingCallback ===

/// # Example
///
/// ```
/// # use rusty_v8 as v8;
/// #
/// fn wasm_streaming_callback_example<'s>(
///   scope: &mut v8::HandleScope<'s>,
///   arguments: v8::FunctionCallbackArguments<'s>,
///   return_value: v8::ReturnValue<'s, v8::Value>,
/// ) {
///   todo!();
/// }
/// ```
pub trait WasmStreamingCallback:
  UnitType
  + for<'s> FnOnce(
    &mut HandleScope<'s>,
    FunctionCallbackArguments<'s>,
    ReturnValue<'s, Value>,
  )
{
}

impl<F> WasmStreamingCallback for F where
  F: UnitType
    + for<'s> FnOnce(
      &mut HandleScope<'s>,
      FunctionCallbackArguments<'s>,
      ReturnValue<'s, Value>,
    )
{
}

#[macro_export]
macro_rules! impl_wasm_streaming_callback {
  () => {
    impl $crate::WasmStreamingCallback
    + for<'__s> ::std::ops::FnOnce(
      &mut $crate::HandleScope<'__s>,
      $crate::FunctionCallbackArguments<'__s>,
      $crate::ReturnValue<'__s, $crate::Value>,
    )
  };
}

#[repr(transparent)]
pub(crate) struct RawWasmStreamingCallback(
  extern "C" fn(*const FunctionCallbackInfo),
);

impl<F: WasmStreamingCallback> From<F> for RawWasmStreamingCallback {
  fn from(_: F) -> Self {
    #[inline(always)]
    extern "C" fn adapter<F: WasmStreamingCallback>(
      info: *const FunctionCallbackInfo,
    ) {
      let scope = &mut unsafe { CallbackScope::new(&*info) };
      let arguments =
        FunctionCallbackArguments::from_function_callback_info(info);
      let return_value =
        ReturnValue::<Value>::from_function_callback_info(info);
      (F::get())(scope, arguments, return_value)
    }

    Self(adapter::<F>)
  }
}

#[cfg(test)]
fn mock_wasm_streaming_callback<'s>(
  _scope: &mut HandleScope<'s>,
  _arguments: FunctionCallbackArguments<'s>,
  _return_value: ReturnValue<'s, Value>,
) {
  unimplemented!()
}

#[test]
fn wasm_streaming_callback_as_type_param() {
  fn pass_as_type_param<F: WasmStreamingCallback>(
    _: F,
  ) -> RawWasmStreamingCallback {
    F::get().into()
  }
  let _ = pass_as_type_param(mock_wasm_streaming_callback);
}

#[test]
fn wasm_streaming_callback_as_impl_trait() {
  fn pass_as_impl_trait(
    f: impl WasmStreamingCallback,
  ) -> RawWasmStreamingCallback {
    f.into()
  }
  let _ = pass_as_impl_trait(mock_wasm_streaming_callback);
}

#[test]
fn wasm_streaming_callback_as_impl_macro() {
  fn pass_as_impl_macro(
    f: impl_wasm_streaming_callback!(),
  ) -> RawWasmStreamingCallback {
    f.into()
  }
  let _ = pass_as_impl_macro(mock_wasm_streaming_callback);
  let _ =
    pass_as_impl_macro(|_scope, _arguments, _return_value| unimplemented!());
}

// === WasmThreadsEnabledCallback ===

/// # Example
///
/// ```
/// # use rusty_v8 as v8;
/// #
/// fn wasm_threads_enabled_callback_example<'s>(
///   scope: &mut v8::HandleScope<'s>,
/// ) -> bool {
///   todo!()
/// }
/// ```
pub trait WasmThreadsEnabledCallback:
  UnitType + for<'s> FnOnce(&mut HandleScope<'s>) -> bool
{
}

impl<F> WasmThreadsEnabledCallback for F where
  F: UnitType + for<'s> FnOnce(&mut HandleScope<'s>) -> bool
{
}

#[macro_export]
macro_rules! impl_wasm_threads_enabled_callback {
  () => {
    impl $crate::WasmThreadsEnabledCallback
    + for<'__s> ::std::ops::FnOnce(
      &mut $crate::HandleScope<'__s>,
    ) -> bool
  };
}

#[repr(transparent)]
pub(crate) struct RawWasmThreadsEnabledCallback(
  for<'s> extern "C" fn(Local<'s, Context>) -> bool,
);

impl<F: WasmThreadsEnabledCallback> From<F> for RawWasmThreadsEnabledCallback {
  fn from(_: F) -> Self {
    #[inline(always)]
    extern "C" fn adapter<'s, F: WasmThreadsEnabledCallback>(
      context: Local<'s, Context>,
    ) -> bool {
      let scope = &mut unsafe { CallbackScope::new(context) };
      (F::get())(scope)
    }

    Self(adapter::<F>)
  }
}

#[cfg(test)]
fn mock_wasm_threads_enabled_callback<'s>(
  _scope: &mut HandleScope<'s>,
) -> bool {
  unimplemented!()
}

#[test]
fn wasm_threads_enabled_callback_as_type_param() {
  fn pass_as_type_param<F: WasmThreadsEnabledCallback>(
    _: F,
  ) -> RawWasmThreadsEnabledCallback {
    F::get().into()
  }
  let _ = pass_as_type_param(mock_wasm_threads_enabled_callback);
}

#[test]
fn wasm_threads_enabled_callback_as_impl_trait() {
  fn pass_as_impl_trait(
    f: impl WasmThreadsEnabledCallback,
  ) -> RawWasmThreadsEnabledCallback {
    f.into()
  }
  let _ = pass_as_impl_trait(mock_wasm_threads_enabled_callback);
}

#[test]
fn wasm_threads_enabled_callback_as_impl_macro() {
  fn pass_as_impl_macro(
    f: impl_wasm_threads_enabled_callback!(),
  ) -> RawWasmThreadsEnabledCallback {
    f.into()
  }
  let _ = pass_as_impl_macro(mock_wasm_threads_enabled_callback);
  let _ = pass_as_impl_macro(|_scope| unimplemented!());
}

// === WasmLoadSourceMapCallback ===

/// # Example
///
/// ```
/// # use rusty_v8 as v8;
/// # use std::ffi::CStr;
/// #
/// fn wasm_load_source_map_callback_example<'s>(
///   scope: &mut v8::HandleScope<'s, ()>,
///   name: &CStr,
/// ) -> v8::Local<'s, v8::String> {
///   todo!()
/// }
/// ```
pub trait WasmLoadSourceMapCallback:
  UnitType + for<'s> FnOnce(&mut HandleScope<'s, ()>, &CStr) -> Local<'s, String>
{
}

impl<F> WasmLoadSourceMapCallback for F where
  F: UnitType
    + for<'s> FnOnce(&mut HandleScope<'s, ()>, &CStr) -> Local<'s, String>
{
}

#[macro_export]
macro_rules! impl_wasm_load_source_map_callback {
  () => {
    impl $crate::WasmLoadSourceMapCallback
    + for<'__s> ::std::ops::FnOnce(
      &mut $crate::HandleScope<'__s, ()>,
      &::std::ffi::CStr,
    ) -> $crate::Local<'__s, $crate::String>
  };
}

#[cfg(target_family = "unix")]
#[repr(transparent)]
pub(crate) struct RawWasmLoadSourceMapCallback(
  extern "C" fn(*mut Isolate, *const c_char) -> Local<'static, String>,
);

#[cfg(all(target_family = "windows", target_arch = "x86_64"))]
#[repr(transparent)]
pub(crate) struct RawWasmLoadSourceMapCallback(
  for<'s> extern "C" fn(
    *mut Local<'s, String>,
    *mut Isolate,
    *const c_char,
  ) -> *mut Local<'s, String>,
);

impl<F: WasmLoadSourceMapCallback> From<F> for RawWasmLoadSourceMapCallback {
  fn from(_: F) -> Self {
    #[inline(always)]
    fn signature_adapter<F: WasmLoadSourceMapCallback>(
      isolate: *mut Isolate,
      name: *const c_char,
    ) -> Local<'static, String> {
      let scope = &mut unsafe { CallbackScope::new(&mut *isolate) };
      let name = unsafe { CStr::from_ptr(name) };
      (F::get())(scope, name)
    }

    #[cfg(target_family = "unix")]
    #[inline(always)]
    extern "C" fn abi_adapter<F: WasmLoadSourceMapCallback>(
      isolate: *mut Isolate,
      name: *const c_char,
    ) -> Local<'static, String> {
      signature_adapter::<F>(isolate, name)
    }

    #[cfg(all(target_family = "windows", target_arch = "x86_64"))]
    #[inline(always)]
    extern "C" fn abi_adapter<'s, F: WasmLoadSourceMapCallback>(
      return_value: *mut Local<'s, String>,
      isolate: *mut Isolate,
      name: *const c_char,
    ) -> *mut Local<'s, String> {
      unsafe {
        std::ptr::write(return_value, signature_adapter::<F>(isolate, name));
        return_value
      }
    }

    Self(abi_adapter::<F>)
  }
}

#[cfg(test)]
fn mock_wasm_load_source_map_callback<'s>(
  _scope: &mut HandleScope<'s, ()>,
  _name: &CStr,
) -> Local<'s, String> {
  unimplemented!()
}

#[test]
fn wasm_load_source_map_callback_as_type_param() {
  fn pass_as_type_param<F: WasmLoadSourceMapCallback>(
    _: F,
  ) -> RawWasmLoadSourceMapCallback {
    F::get().into()
  }
  let _ = pass_as_type_param(mock_wasm_load_source_map_callback);
}

#[test]
fn wasm_load_source_map_callback_as_impl_trait() {
  fn pass_as_impl_trait(
    f: impl WasmLoadSourceMapCallback,
  ) -> RawWasmLoadSourceMapCallback {
    f.into()
  }
  let _ = pass_as_impl_trait(mock_wasm_load_source_map_callback);
}

#[test]
fn wasm_load_source_map_callback_as_impl_macro() {
  fn pass_as_impl_macro(
    f: impl_wasm_load_source_map_callback!(),
  ) -> RawWasmLoadSourceMapCallback {
    f.into()
  }
  let _ = pass_as_impl_macro(mock_wasm_load_source_map_callback);
  let _ = pass_as_impl_macro(|_scope, _name| unimplemented!());
}

// === WasmSimdEnabledCallback ===

/// # Example
///
/// ```
/// # use rusty_v8 as v8;
/// #
/// fn wasm_simd_enabled_callback_example<'s>(
///   scope: &mut v8::HandleScope<'s>,
/// ) -> bool {
///   todo!()
/// }
/// ```
pub trait WasmSimdEnabledCallback:
  UnitType + for<'s> FnOnce(&mut HandleScope<'s>) -> bool
{
}

impl<F> WasmSimdEnabledCallback for F where
  F: UnitType + for<'s> FnOnce(&mut HandleScope<'s>) -> bool
{
}

#[macro_export]
macro_rules! impl_wasm_simd_enabled_callback {
  () => {
    impl $crate::WasmSimdEnabledCallback
    + for<'__s> ::std::ops::FnOnce(
      &mut $crate::HandleScope<'__s>,
    ) -> bool
  };
}

#[repr(transparent)]
pub(crate) struct RawWasmSimdEnabledCallback(
  for<'s> extern "C" fn(Local<'s, Context>) -> bool,
);

impl<F: WasmSimdEnabledCallback> From<F> for RawWasmSimdEnabledCallback {
  fn from(_: F) -> Self {
    #[inline(always)]
    extern "C" fn adapter<'s, F: WasmSimdEnabledCallback>(
      context: Local<'s, Context>,
    ) -> bool {
      let scope = &mut unsafe { CallbackScope::new(context) };
      (F::get())(scope)
    }

    Self(adapter::<F>)
  }
}

#[cfg(test)]
fn mock_wasm_simd_enabled_callback<'s>(_scope: &mut HandleScope<'s>) -> bool {
  unimplemented!()
}

#[test]
fn wasm_simd_enabled_callback_as_type_param() {
  fn pass_as_type_param<F: WasmSimdEnabledCallback>(
    _: F,
  ) -> RawWasmSimdEnabledCallback {
    F::get().into()
  }
  let _ = pass_as_type_param(mock_wasm_simd_enabled_callback);
}

#[test]
fn wasm_simd_enabled_callback_as_impl_trait() {
  fn pass_as_impl_trait(
    f: impl WasmSimdEnabledCallback,
  ) -> RawWasmSimdEnabledCallback {
    f.into()
  }
  let _ = pass_as_impl_trait(mock_wasm_simd_enabled_callback);
}

#[test]
fn wasm_simd_enabled_callback_as_impl_macro() {
  fn pass_as_impl_macro(
    f: impl_wasm_simd_enabled_callback!(),
  ) -> RawWasmSimdEnabledCallback {
    f.into()
  }
  let _ = pass_as_impl_macro(mock_wasm_simd_enabled_callback);
  let _ = pass_as_impl_macro(|_scope| unimplemented!());
}

// === InterruptCallback ===

/// # Example
///
/// ```
/// # use rusty_v8 as v8;
/// #
/// fn interrupt_callback_example(isolate: &mut v8::Isolate, data: *mut ()) {
///   todo!();
/// }
/// ```
pub trait InterruptCallback: UnitType + FnOnce(&mut Isolate, *mut ()) {}

impl<F> InterruptCallback for F where F: UnitType + FnOnce(&mut Isolate, *mut ())
{}

#[macro_export]
macro_rules! impl_interrupt_callback {
  () => {
    impl $crate::InterruptCallback
    + ::std::ops::FnOnce(
      &mut $crate::Isolate,
      *mut (),
    )
  };
}

#[repr(transparent)]
pub(crate) struct RawInterruptCallback(
  extern "C" fn(*mut Isolate, *mut c_void),
);

impl<F: InterruptCallback> From<F> for RawInterruptCallback {
  fn from(_: F) -> Self {
    #[inline(always)]
    extern "C" fn adapter<F: InterruptCallback>(
      isolate: *mut Isolate,
      data: *mut c_void,
    ) {
      let isolate = unsafe { &mut *isolate };
      let data = data as *mut ();
      (F::get())(isolate, data)
    }

    Self(adapter::<F>)
  }
}

#[cfg(test)]
fn mock_interrupt_callback(_isolate: &mut Isolate, _data: *mut ()) {
  unimplemented!()
}

#[test]
fn interrupt_callback_as_type_param() {
  fn pass_as_type_param<F: InterruptCallback>(_: F) -> RawInterruptCallback {
    F::get().into()
  }
  let _ = pass_as_type_param(mock_interrupt_callback);
}

#[test]
fn interrupt_callback_as_impl_trait() {
  fn pass_as_impl_trait(f: impl InterruptCallback) -> RawInterruptCallback {
    f.into()
  }
  let _ = pass_as_impl_trait(mock_interrupt_callback);
}

#[test]
fn interrupt_callback_as_impl_macro() {
  fn pass_as_impl_macro(f: impl_interrupt_callback!()) -> RawInterruptCallback {
    f.into()
  }
  let _ = pass_as_impl_macro(mock_interrupt_callback);
  let _ = pass_as_impl_macro(|_isolate, _data| unimplemented!());
}

/// This callback is invoked when the heap size is close to the heap limit and
/// V8 is likely to abort with out-of-memory error.
/// The callback can extend the heap limit by returning a value that is greater
/// than the current_heap_limit. The initial heap limit is the limit that was
/// set after heap setup.
///
/// # Example
///
/// ```
/// fn near_heap_limit_callback_example(
///   current_heap_limit: usize,
///   initial_heap_limit: usize,
///   data: *mut (),
/// ) -> usize {
///   todo!()
/// }
/// ```
pub trait NearHeapLimitCallback:
  UnitType + FnOnce(usize, usize, *mut ()) -> usize
{
}

impl<F> NearHeapLimitCallback for F where
  F: UnitType + FnOnce(usize, usize, *mut ()) -> usize
{
}

#[macro_export]
macro_rules! impl_near_heap_limit_callback {
  () => {
    impl $crate::NearHeapLimitCallback
    + ::std::ops::FnOnce(
      usize,
      usize,
      *mut (),
    ) -> usize
  };
}

#[repr(transparent)]
pub(crate) struct RawNearHeapLimitCallback(
  extern "C" fn(*mut c_void, usize, usize) -> usize,
);

impl<F: NearHeapLimitCallback> From<F> for RawNearHeapLimitCallback {
  fn from(_: F) -> Self {
    #[inline(always)]
    extern "C" fn adapter<F: NearHeapLimitCallback>(
      data: *mut c_void,
      current_heap_limit: usize,
      initial_heap_limit: usize,
    ) -> usize {
      let data = data as *mut ();
      (F::get())(current_heap_limit, initial_heap_limit, data)
    }

    Self(adapter::<F>)
  }
}

#[cfg(test)]
fn mock_near_heap_limit_callback(
  _current_heap_limit: usize,
  _initial_heap_limit: usize,
  _data: *mut (),
) -> usize {
  unimplemented!()
}

#[test]
fn near_heap_limit_callback_as_type_param() {
  fn pass_as_type_param<F: NearHeapLimitCallback>(
    _: F,
  ) -> RawNearHeapLimitCallback {
    F::get().into()
  }
  let _ = pass_as_type_param(mock_near_heap_limit_callback);
}

#[test]
fn near_heap_limit_callback_as_impl_trait() {
  fn pass_as_impl_trait(
    f: impl NearHeapLimitCallback,
  ) -> RawNearHeapLimitCallback {
    f.into()
  }
  let _ = pass_as_impl_trait(mock_near_heap_limit_callback);
}

#[test]
fn near_heap_limit_callback_as_impl_macro() {
  fn pass_as_impl_macro(
    f: impl_near_heap_limit_callback!(),
  ) -> RawNearHeapLimitCallback {
    f.into()
  }
  let _ = pass_as_impl_macro(mock_near_heap_limit_callback);
  let _ = pass_as_impl_macro(
    |_current_heap_limit, _initial_heap_limit, _data| unimplemented!(),
  );
}

/// Callback function passed to SetJitCodeEventHandler.
///
/// \param event code add, move or removal event.
///
/// # Example
///
/// ```
/// # use rusty_v8 as v8;
/// #
/// fn jit_code_event_handler_example(event: &v8::JitCodeEvent) {
///   todo!();
/// }
/// ```
pub trait JitCodeEventHandler: UnitType + FnOnce(&JitCodeEvent) {}

impl<F> JitCodeEventHandler for F where F: UnitType + FnOnce(&JitCodeEvent) {}

#[macro_export]
macro_rules! impl_jit_code_event_handler {
  () => {
    impl $crate::JitCodeEventHandler
    + ::std::ops::FnOnce(
      &$crate::JitCodeEvent,
    )
  };
}

#[repr(transparent)]
pub(crate) struct RawJitCodeEventHandler(extern "C" fn(*const JitCodeEvent));

impl<F: JitCodeEventHandler> From<F> for RawJitCodeEventHandler {
  fn from(_: F) -> Self {
    #[inline(always)]
    extern "C" fn adapter<F: JitCodeEventHandler>(event: *const JitCodeEvent) {
      let event = unsafe { &*event };
      (F::get())(event)
    }

    Self(adapter::<F>)
  }
}

#[cfg(test)]
fn mock_jit_code_event_handler(_event: &JitCodeEvent) {
  unimplemented!()
}

#[test]
fn jit_code_event_handler_as_type_param() {
  fn pass_as_type_param<F: JitCodeEventHandler>(
    _: F,
  ) -> RawJitCodeEventHandler {
    F::get().into()
  }
  let _ = pass_as_type_param(mock_jit_code_event_handler);
}

#[test]
fn jit_code_event_handler_as_impl_trait() {
  fn pass_as_impl_trait(f: impl JitCodeEventHandler) -> RawJitCodeEventHandler {
    f.into()
  }
  let _ = pass_as_impl_trait(mock_jit_code_event_handler);
}

#[test]
fn jit_code_event_handler_as_impl_macro() {
  fn pass_as_impl_macro(
    f: impl_jit_code_event_handler!(),
  ) -> RawJitCodeEventHandler {
    f.into()
  }
  let _ = pass_as_impl_macro(mock_jit_code_event_handler);
  let _ = pass_as_impl_macro(|_event| unimplemented!());
}

// === UnhandledExceptionCallback ===

/// # Example
///
/// ```
/// # use rusty_v8 as v8;
/// #
/// fn unhandled_exception_callback_example(
///   exception_pointers: &v8::EXCEPTION_POINTERS,
/// ) -> i32 {
///   todo!()
/// }
/// ```
#[cfg(target_family = "windows")]
pub trait UnhandledExceptionCallback:
  UnitType + FnOnce(&EXCEPTION_POINTERS) -> i32
{
}

#[cfg(target_family = "windows")]
impl<F> UnhandledExceptionCallback for F where
  F: UnitType + FnOnce(&EXCEPTION_POINTERS) -> i32
{
}

#[cfg(target_family = "windows")]
#[macro_export]
macro_rules! impl_unhandled_exception_callback {
  () => {
    impl $crate::UnhandledExceptionCallback
    + ::std::ops::FnOnce(
      &$crate::EXCEPTION_POINTERS,
    ) -> i32
  };
}

#[cfg(target_family = "windows")]
#[repr(transparent)]
pub(crate) struct RawUnhandledExceptionCallback(
  extern "C" fn(*const EXCEPTION_POINTERS) -> c_int,
);

#[cfg(target_family = "windows")]
impl<F: UnhandledExceptionCallback> From<F> for RawUnhandledExceptionCallback {
  fn from(_: F) -> Self {
    #[inline(always)]
    extern "C" fn adapter<F: UnhandledExceptionCallback>(
      exception_pointers: *const EXCEPTION_POINTERS,
    ) -> c_int {
      let exception_pointers = unsafe { &*exception_pointers };
      (F::get())(exception_pointers)
    }

    Self(adapter::<F>)
  }
}

#[cfg(target_family = "windows")]
#[cfg(test)]
fn mock_unhandled_exception_callback(
  _exception_pointers: &EXCEPTION_POINTERS,
) -> i32 {
  unimplemented!()
}

#[cfg(target_family = "windows")]
#[test]
fn unhandled_exception_callback_as_type_param() {
  fn pass_as_type_param<F: UnhandledExceptionCallback>(
    _: F,
  ) -> RawUnhandledExceptionCallback {
    F::get().into()
  }
  let _ = pass_as_type_param(mock_unhandled_exception_callback);
}

#[cfg(target_family = "windows")]
#[test]
fn unhandled_exception_callback_as_impl_trait() {
  fn pass_as_impl_trait(
    f: impl UnhandledExceptionCallback,
  ) -> RawUnhandledExceptionCallback {
    f.into()
  }
  let _ = pass_as_impl_trait(mock_unhandled_exception_callback);
}

#[cfg(target_family = "windows")]
#[test]
fn unhandled_exception_callback_as_impl_macro() {
  fn pass_as_impl_macro(
    f: impl_unhandled_exception_callback!(),
  ) -> RawUnhandledExceptionCallback {
    f.into()
  }
  let _ = pass_as_impl_macro(mock_unhandled_exception_callback);
  let _ = pass_as_impl_macro(|_exception_pointers| unimplemented!());
}

// === SerializeInternalFieldsCallbackFn ===

/// # Example
///
/// ```
/// # use rusty_v8 as v8;
/// #
/// fn serialize_internal_fields_callback_fn_example<'s>(
///   scope: &mut v8::HandleScope<'s>,
///   holder: v8::Local<'s, v8::Object>,
///   index: i32,
///   data: *mut (),
/// ) -> v8::StartupData {
///   todo!()
/// }
/// ```
pub trait SerializeInternalFieldsCallbackFn:
  UnitType
  + for<'s> FnOnce(
    &mut HandleScope<'s>,
    Local<'s, Object>,
    i32,
    *mut (),
  ) -> StartupData
{
}

impl<F> SerializeInternalFieldsCallbackFn for F where
  F: UnitType
    + for<'s> FnOnce(
      &mut HandleScope<'s>,
      Local<'s, Object>,
      i32,
      *mut (),
    ) -> StartupData
{
}

#[macro_export]
macro_rules! impl_serialize_internal_fields_callback_fn {
  () => {
    impl $crate::SerializeInternalFieldsCallbackFn
    + for<'__s> ::std::ops::FnOnce(
      &mut $crate::HandleScope<'__s>,
      $crate::Local<'__s, $crate::Object>,
      i32,
      *mut (),
    ) -> $crate::StartupData
  };
}

#[repr(transparent)]
pub(crate) struct RawSerializeInternalFieldsCallbackFn(
  for<'s> extern "C" fn(Local<'s, Object>, c_int, *mut c_void) -> StartupData,
);

impl<F: SerializeInternalFieldsCallbackFn> From<F>
  for RawSerializeInternalFieldsCallbackFn
{
  fn from(_: F) -> Self {
    #[inline(always)]
    extern "C" fn adapter<'s, F: SerializeInternalFieldsCallbackFn>(
      holder: Local<'s, Object>,
      index: c_int,
      data: *mut c_void,
    ) -> StartupData {
      let scope = &mut unsafe { CallbackScope::new(holder) };
      let data = data as *mut ();
      (F::get())(scope, holder, index, data)
    }

    Self(adapter::<F>)
  }
}

#[cfg(test)]
fn mock_serialize_internal_fields_callback_fn<'s>(
  _scope: &mut HandleScope<'s>,
  _holder: Local<'s, Object>,
  _index: i32,
  _data: *mut (),
) -> StartupData {
  unimplemented!()
}

#[test]
fn serialize_internal_fields_callback_fn_as_type_param() {
  fn pass_as_type_param<F: SerializeInternalFieldsCallbackFn>(
    _: F,
  ) -> RawSerializeInternalFieldsCallbackFn {
    F::get().into()
  }
  let _ = pass_as_type_param(mock_serialize_internal_fields_callback_fn);
}

#[test]
fn serialize_internal_fields_callback_fn_as_impl_trait() {
  fn pass_as_impl_trait(
    f: impl SerializeInternalFieldsCallbackFn,
  ) -> RawSerializeInternalFieldsCallbackFn {
    f.into()
  }
  let _ = pass_as_impl_trait(mock_serialize_internal_fields_callback_fn);
}

#[test]
fn serialize_internal_fields_callback_fn_as_impl_macro() {
  fn pass_as_impl_macro(
    f: impl_serialize_internal_fields_callback_fn!(),
  ) -> RawSerializeInternalFieldsCallbackFn {
    f.into()
  }
  let _ = pass_as_impl_macro(mock_serialize_internal_fields_callback_fn);
  let _ = pass_as_impl_macro(|_scope, _holder, _index, _data| unimplemented!());
}

// === DeserializeInternalFieldsCallbackFn ===

/// # Example
///
/// ```
/// # use rusty_v8 as v8;
/// #
/// fn deserialize_internal_fields_callback_fn_example<'s>(
///   scope: &mut v8::HandleScope<'s>,
///   holder: v8::Local<'s, v8::Object>,
///   index: i32,
///   payload: v8::StartupData,
///   data: *mut (),
/// ) {
///   todo!();
/// }
/// ```
pub trait DeserializeInternalFieldsCallbackFn:
  UnitType
  + for<'s> FnOnce(
    &mut HandleScope<'s>,
    Local<'s, Object>,
    i32,
    StartupData,
    *mut (),
  )
{
}

impl<F> DeserializeInternalFieldsCallbackFn for F where
  F: UnitType
    + for<'s> FnOnce(
      &mut HandleScope<'s>,
      Local<'s, Object>,
      i32,
      StartupData,
      *mut (),
    )
{
}

#[macro_export]
macro_rules! impl_deserialize_internal_fields_callback_fn {
  () => {
    impl $crate::DeserializeInternalFieldsCallbackFn
    + for<'__s> ::std::ops::FnOnce(
      &mut $crate::HandleScope<'__s>,
      $crate::Local<'__s, $crate::Object>,
      i32,
      $crate::StartupData,
      *mut (),
    )
  };
}

#[repr(transparent)]
pub(crate) struct RawDeserializeInternalFieldsCallbackFn(
  for<'s> extern "C" fn(Local<'s, Object>, c_int, StartupData, *mut c_void),
);

impl<F: DeserializeInternalFieldsCallbackFn> From<F>
  for RawDeserializeInternalFieldsCallbackFn
{
  fn from(_: F) -> Self {
    #[inline(always)]
    extern "C" fn adapter<'s, F: DeserializeInternalFieldsCallbackFn>(
      holder: Local<'s, Object>,
      index: c_int,
      payload: StartupData,
      data: *mut c_void,
    ) {
      let scope = &mut unsafe { CallbackScope::new(holder) };
      let data = data as *mut ();
      (F::get())(scope, holder, index, payload, data)
    }

    Self(adapter::<F>)
  }
}

#[cfg(test)]
fn mock_deserialize_internal_fields_callback_fn<'s>(
  _scope: &mut HandleScope<'s>,
  _holder: Local<'s, Object>,
  _index: i32,
  _payload: StartupData,
  _data: *mut (),
) {
  unimplemented!()
}

#[test]
fn deserialize_internal_fields_callback_fn_as_type_param() {
  fn pass_as_type_param<F: DeserializeInternalFieldsCallbackFn>(
    _: F,
  ) -> RawDeserializeInternalFieldsCallbackFn {
    F::get().into()
  }
  let _ = pass_as_type_param(mock_deserialize_internal_fields_callback_fn);
}

#[test]
fn deserialize_internal_fields_callback_fn_as_impl_trait() {
  fn pass_as_impl_trait(
    f: impl DeserializeInternalFieldsCallbackFn,
  ) -> RawDeserializeInternalFieldsCallbackFn {
    f.into()
  }
  let _ = pass_as_impl_trait(mock_deserialize_internal_fields_callback_fn);
}

#[test]
fn deserialize_internal_fields_callback_fn_as_impl_macro() {
  fn pass_as_impl_macro(
    f: impl_deserialize_internal_fields_callback_fn!(),
  ) -> RawDeserializeInternalFieldsCallbackFn {
    f.into()
  }
  let _ = pass_as_impl_macro(mock_deserialize_internal_fields_callback_fn);
  let _ = pass_as_impl_macro(
    |_scope, _holder, _index, _payload, _data| unimplemented!(),
  );
}

// === UseCounterCallback ===

/// # Example
///
/// ```
/// # use rusty_v8 as v8;
/// #
/// fn use_counter_callback_example(
///   isolate: &mut v8::Isolate,
///   feature: v8::UseCounterFeature,
/// ) {
///   todo!();
/// }
/// ```
pub trait UseCounterCallback:
  UnitType + FnOnce(&mut Isolate, UseCounterFeature)
{
}

impl<F> UseCounterCallback for F where
  F: UnitType + FnOnce(&mut Isolate, UseCounterFeature)
{
}

#[macro_export]
macro_rules! impl_use_counter_callback {
  () => {
    impl $crate::UseCounterCallback
    + ::std::ops::FnOnce(
      &mut $crate::Isolate,
      $crate::UseCounterFeature,
    )
  };
}

#[repr(transparent)]
pub(crate) struct RawUseCounterCallback(
  extern "C" fn(*mut Isolate, UseCounterFeature),
);

impl<F: UseCounterCallback> From<F> for RawUseCounterCallback {
  fn from(_: F) -> Self {
    #[inline(always)]
    extern "C" fn adapter<F: UseCounterCallback>(
      isolate: *mut Isolate,
      feature: UseCounterFeature,
    ) {
      let isolate = unsafe { &mut *isolate };
      (F::get())(isolate, feature)
    }

    Self(adapter::<F>)
  }
}

#[cfg(test)]
fn mock_use_counter_callback(
  _isolate: &mut Isolate,
  _feature: UseCounterFeature,
) {
  unimplemented!()
}

#[test]
fn use_counter_callback_as_type_param() {
  fn pass_as_type_param<F: UseCounterCallback>(_: F) -> RawUseCounterCallback {
    F::get().into()
  }
  let _ = pass_as_type_param(mock_use_counter_callback);
}

#[test]
fn use_counter_callback_as_impl_trait() {
  fn pass_as_impl_trait(f: impl UseCounterCallback) -> RawUseCounterCallback {
    f.into()
  }
  let _ = pass_as_impl_trait(mock_use_counter_callback);
}

#[test]
fn use_counter_callback_as_impl_macro() {
  fn pass_as_impl_macro(
    f: impl_use_counter_callback!(),
  ) -> RawUseCounterCallback {
    f.into()
  }
  let _ = pass_as_impl_macro(mock_use_counter_callback);
  let _ = pass_as_impl_macro(|_isolate, _feature| unimplemented!());
}

/// Custom callback used by embedders to help V8 determine if it should abort
/// when it throws and no internal handler is predicted to catch the
/// exception. If --abort-on-uncaught-exception is used on the command line,
/// then V8 will abort if either:
/// - no custom callback is set.
/// - the custom callback set returns true.
/// Otherwise, the custom callback will not be called and V8 will not abort.
///
/// # Example
///
/// ```
/// # use rusty_v8 as v8;
/// #
/// fn abort_on_uncaught_exception_callback_example(
///   isolate: &mut v8::Isolate,
/// ) -> bool {
///   todo!()
/// }
/// ```
pub trait AbortOnUncaughtExceptionCallback:
  UnitType + FnOnce(&mut Isolate) -> bool
{
}

impl<F> AbortOnUncaughtExceptionCallback for F where
  F: UnitType + FnOnce(&mut Isolate) -> bool
{
}

#[macro_export]
macro_rules! impl_abort_on_uncaught_exception_callback {
  () => {
    impl $crate::AbortOnUncaughtExceptionCallback
    + ::std::ops::FnOnce(
      &mut $crate::Isolate,
    ) -> bool
  };
}

#[repr(transparent)]
pub(crate) struct RawAbortOnUncaughtExceptionCallback(
  extern "C" fn(*mut Isolate) -> bool,
);

impl<F: AbortOnUncaughtExceptionCallback> From<F>
  for RawAbortOnUncaughtExceptionCallback
{
  fn from(_: F) -> Self {
    #[inline(always)]
    extern "C" fn adapter<F: AbortOnUncaughtExceptionCallback>(
      isolate: *mut Isolate,
    ) -> bool {
      let isolate = unsafe { &mut *isolate };
      (F::get())(isolate)
    }

    Self(adapter::<F>)
  }
}

#[cfg(test)]
fn mock_abort_on_uncaught_exception_callback(_isolate: &mut Isolate) -> bool {
  unimplemented!()
}

#[test]
fn abort_on_uncaught_exception_callback_as_type_param() {
  fn pass_as_type_param<F: AbortOnUncaughtExceptionCallback>(
    _: F,
  ) -> RawAbortOnUncaughtExceptionCallback {
    F::get().into()
  }
  let _ = pass_as_type_param(mock_abort_on_uncaught_exception_callback);
}

#[test]
fn abort_on_uncaught_exception_callback_as_impl_trait() {
  fn pass_as_impl_trait(
    f: impl AbortOnUncaughtExceptionCallback,
  ) -> RawAbortOnUncaughtExceptionCallback {
    f.into()
  }
  let _ = pass_as_impl_trait(mock_abort_on_uncaught_exception_callback);
}

#[test]
fn abort_on_uncaught_exception_callback_as_impl_macro() {
  fn pass_as_impl_macro(
    f: impl_abort_on_uncaught_exception_callback!(),
  ) -> RawAbortOnUncaughtExceptionCallback {
    f.into()
  }
  let _ = pass_as_impl_macro(mock_abort_on_uncaught_exception_callback);
  let _ = pass_as_impl_macro(|_isolate| unimplemented!());
}

// === GCCallback ===

/// # Example
///
/// ```
/// # use rusty_v8 as v8;
/// #
/// fn gc_callback_example(
///   isolate: &mut v8::Isolate,
///   r#type: v8::GCType,
///   flags: v8::GCCallbackFlags,
///   data: *mut (),
/// ) {
///   todo!();
/// }
/// ```
pub trait GCCallback:
  UnitType + FnOnce(&mut Isolate, GCType, GCCallbackFlags, *mut ())
{
}

impl<F> GCCallback for F where
  F: UnitType + FnOnce(&mut Isolate, GCType, GCCallbackFlags, *mut ())
{
}

#[macro_export]
macro_rules! impl_gc_callback {
  () => {
    impl $crate::GCCallback
    + ::std::ops::FnOnce(
      &mut $crate::Isolate,
      $crate::GCType,
      $crate::GCCallbackFlags,
      *mut (),
    )
  };
}

#[repr(transparent)]
pub(crate) struct RawGCCallback(
  extern "C" fn(*mut Isolate, GCType, GCCallbackFlags, *mut c_void),
);

impl<F: GCCallback> From<F> for RawGCCallback {
  fn from(_: F) -> Self {
    #[inline(always)]
    extern "C" fn adapter<F: GCCallback>(
      isolate: *mut Isolate,
      r#type: GCType,
      flags: GCCallbackFlags,
      data: *mut c_void,
    ) {
      let isolate = unsafe { &mut *isolate };
      let data = data as *mut ();
      (F::get())(isolate, r#type, flags, data)
    }

    Self(adapter::<F>)
  }
}

#[cfg(test)]
fn mock_gc_callback(
  _isolate: &mut Isolate,
  _type: GCType,
  _flags: GCCallbackFlags,
  _data: *mut (),
) {
  unimplemented!()
}

#[test]
fn gc_callback_as_type_param() {
  fn pass_as_type_param<F: GCCallback>(_: F) -> RawGCCallback {
    F::get().into()
  }
  let _ = pass_as_type_param(mock_gc_callback);
}

#[test]
fn gc_callback_as_impl_trait() {
  fn pass_as_impl_trait(f: impl GCCallback) -> RawGCCallback {
    f.into()
  }
  let _ = pass_as_impl_trait(mock_gc_callback);
}

#[test]
fn gc_callback_as_impl_macro() {
  fn pass_as_impl_macro(f: impl_gc_callback!()) -> RawGCCallback {
    f.into()
  }
  let _ = pass_as_impl_macro(mock_gc_callback);
  let _ = pass_as_impl_macro(|_isolate, _type, _flags, _data| unimplemented!());
}

/// Embedder callback for `Atomics.wait()` that can be added through
/// `SetAtomicsWaitCallback`.
///
/// This will be called just before starting to wait with the `event` value
/// `kStartWait` and after finishing waiting with one of the other
/// values of `AtomicsWaitEvent` inside of an `Atomics.wait()` call.
///
/// `array_buffer` will refer to the underlying SharedArrayBuffer,
/// `offset_in_bytes` to the location of the waited-on memory address inside
/// the SharedArrayBuffer.
///
/// `value` and `timeout_in_ms` will be the values passed to
/// the `Atomics.wait()` call. If no timeout was used, `timeout_in_ms`
/// will be `INFINITY`.
///
/// In the `kStartWait` callback, `stop_handle` will be an object that
/// is only valid until the corresponding finishing callback and that
/// can be used to stop the wait process while it is happening.
///
/// This callback may schedule exceptions, *unless* `event` is equal to
/// `kTerminatedExecution`.
///
/// # Example
///
/// ```
/// # use rusty_v8 as v8;
/// #
/// fn atomics_wait_callback_example<'s>(
///   scope: &mut v8::HandleScope<'s>,
///   event: v8::AtomicsWaitEvent,
///   array_buffer: v8::Local<'s, v8::SharedArrayBuffer>,
///   offset_in_bytes: usize,
///   value: i64,
///   timeout_in_ms: f64,
///   stop_handle: &mut v8::AtomicsWaitWakeHandle,
///   data: *mut (),
/// ) {
///   todo!();
/// }
/// ```
pub trait AtomicsWaitCallback:
  UnitType
  + for<'s> FnOnce(
    &mut HandleScope<'s>,
    AtomicsWaitEvent,
    Local<'s, SharedArrayBuffer>,
    usize,
    i64,
    f64,
    &mut AtomicsWaitWakeHandle,
    *mut (),
  )
{
}

impl<F> AtomicsWaitCallback for F where
  F: UnitType
    + for<'s> FnOnce(
      &mut HandleScope<'s>,
      AtomicsWaitEvent,
      Local<'s, SharedArrayBuffer>,
      usize,
      i64,
      f64,
      &mut AtomicsWaitWakeHandle,
      *mut (),
    )
{
}

#[macro_export]
macro_rules! impl_atomics_wait_callback {
  () => {
    impl $crate::AtomicsWaitCallback
    + for<'__s> ::std::ops::FnOnce(
      &mut $crate::HandleScope<'__s>,
      $crate::AtomicsWaitEvent,
      $crate::Local<'__s, $crate::SharedArrayBuffer>,
      usize,
      i64,
      f64,
      &mut $crate::AtomicsWaitWakeHandle,
      *mut (),
    )
  };
}

#[repr(transparent)]
pub(crate) struct RawAtomicsWaitCallback(
  for<'s> extern "C" fn(
    AtomicsWaitEvent,
    Local<'s, SharedArrayBuffer>,
    usize,
    i64,
    c_double,
    *mut AtomicsWaitWakeHandle,
    *mut c_void,
  ),
);

impl<F: AtomicsWaitCallback> From<F> for RawAtomicsWaitCallback {
  fn from(_: F) -> Self {
    #[inline(always)]
    extern "C" fn adapter<'s, F: AtomicsWaitCallback>(
      event: AtomicsWaitEvent,
      array_buffer: Local<'s, SharedArrayBuffer>,
      offset_in_bytes: usize,
      value: i64,
      timeout_in_ms: c_double,
      stop_handle: *mut AtomicsWaitWakeHandle,
      data: *mut c_void,
    ) {
      let scope = &mut unsafe { CallbackScope::new(array_buffer) };
      let stop_handle = unsafe { &mut *stop_handle };
      let data = data as *mut ();
      (F::get())(
        scope,
        event,
        array_buffer,
        offset_in_bytes,
        value,
        timeout_in_ms,
        stop_handle,
        data,
      )
    }

    Self(adapter::<F>)
  }
}

#[cfg(test)]
fn mock_atomics_wait_callback<'s>(
  _scope: &mut HandleScope<'s>,
  _event: AtomicsWaitEvent,
  _array_buffer: Local<'s, SharedArrayBuffer>,
  _offset_in_bytes: usize,
  _value: i64,
  _timeout_in_ms: f64,
  _stop_handle: &mut AtomicsWaitWakeHandle,
  _data: *mut (),
) {
  unimplemented!()
}

#[test]
fn atomics_wait_callback_as_type_param() {
  fn pass_as_type_param<F: AtomicsWaitCallback>(
    _: F,
  ) -> RawAtomicsWaitCallback {
    F::get().into()
  }
  let _ = pass_as_type_param(mock_atomics_wait_callback);
}

#[test]
fn atomics_wait_callback_as_impl_trait() {
  fn pass_as_impl_trait(f: impl AtomicsWaitCallback) -> RawAtomicsWaitCallback {
    f.into()
  }
  let _ = pass_as_impl_trait(mock_atomics_wait_callback);
}

#[test]
fn atomics_wait_callback_as_impl_macro() {
  fn pass_as_impl_macro(
    f: impl_atomics_wait_callback!(),
  ) -> RawAtomicsWaitCallback {
    f.into()
  }
  let _ = pass_as_impl_macro(mock_atomics_wait_callback);
  let _ = pass_as_impl_macro(
    |_scope,
     _event,
     _array_buffer,
     _offset_in_bytes,
     _value,
     _timeout_in_ms,
     _stop_handle,
     _data| unimplemented!(),
  );
}

// === GetExternallyAllocatedMemoryInBytesCallback ===

/// # Example
///
/// ```
/// fn get_externally_allocated_memory_in_bytes_callback_example() -> usize {
///   todo!()
/// }
/// ```
pub trait GetExternallyAllocatedMemoryInBytesCallback:
  UnitType + FnOnce() -> usize
{
}

impl<F> GetExternallyAllocatedMemoryInBytesCallback for F where
  F: UnitType + FnOnce() -> usize
{
}

#[macro_export]
macro_rules! impl_get_externally_allocated_memory_in_bytes_callback {
  () => {
    impl $crate::GetExternallyAllocatedMemoryInBytesCallback
    + ::std::ops::FnOnce(
    ) -> usize
  };
}

#[repr(transparent)]
pub(crate) struct RawGetExternallyAllocatedMemoryInBytesCallback(
  extern "C" fn() -> usize,
);

impl<F: GetExternallyAllocatedMemoryInBytesCallback> From<F>
  for RawGetExternallyAllocatedMemoryInBytesCallback
{
  fn from(_: F) -> Self {
    #[inline(always)]
    extern "C" fn adapter<F: GetExternallyAllocatedMemoryInBytesCallback>(
    ) -> usize {
      (F::get())()
    }

    Self(adapter::<F>)
  }
}

#[cfg(test)]
fn mock_get_externally_allocated_memory_in_bytes_callback() -> usize {
  unimplemented!()
}

#[test]
fn get_externally_allocated_memory_in_bytes_callback_as_type_param() {
  fn pass_as_type_param<F: GetExternallyAllocatedMemoryInBytesCallback>(
    _: F,
  ) -> RawGetExternallyAllocatedMemoryInBytesCallback {
    F::get().into()
  }
  let _ =
    pass_as_type_param(mock_get_externally_allocated_memory_in_bytes_callback);
}

#[test]
fn get_externally_allocated_memory_in_bytes_callback_as_impl_trait() {
  fn pass_as_impl_trait(
    f: impl GetExternallyAllocatedMemoryInBytesCallback,
  ) -> RawGetExternallyAllocatedMemoryInBytesCallback {
    f.into()
  }
  let _ =
    pass_as_impl_trait(mock_get_externally_allocated_memory_in_bytes_callback);
}

#[test]
fn get_externally_allocated_memory_in_bytes_callback_as_impl_macro() {
  fn pass_as_impl_macro(
    f: impl_get_externally_allocated_memory_in_bytes_callback!(),
  ) -> RawGetExternallyAllocatedMemoryInBytesCallback {
    f.into()
  }
  let _ =
    pass_as_impl_macro(mock_get_externally_allocated_memory_in_bytes_callback);
  let _ = pass_as_impl_macro(|| unimplemented!());
}

/// EntropySource is used as a callback function when v8 needs a source
/// of entropy.
///
/// # Example
///
/// ```
/// fn entropy_source_example(buffer: &mut [u8]) -> bool {
///   todo!()
/// }
/// ```
pub trait EntropySource: UnitType + FnOnce(&mut [u8]) -> bool {}

impl<F> EntropySource for F where F: UnitType + FnOnce(&mut [u8]) -> bool {}

#[macro_export]
macro_rules! impl_entropy_source {
  () => {
    impl $crate::EntropySource
    + ::std::ops::FnOnce(
      &mut [u8],
    ) -> bool
  };
}

#[repr(transparent)]
pub(crate) struct RawEntropySource(extern "C" fn(*mut c_uchar, usize) -> bool);

impl<F: EntropySource> From<F> for RawEntropySource {
  fn from(_: F) -> Self {
    #[inline(always)]
    extern "C" fn adapter<F: EntropySource>(
      buffer: *mut c_uchar,
      length: usize,
    ) -> bool {
      let buffer = unsafe { slice::from_raw_parts_mut(buffer, length) };
      (F::get())(buffer)
    }

    Self(adapter::<F>)
  }
}

#[cfg(test)]
fn mock_entropy_source(_buffer: &mut [u8]) -> bool {
  unimplemented!()
}

#[test]
fn entropy_source_as_type_param() {
  fn pass_as_type_param<F: EntropySource>(_: F) -> RawEntropySource {
    F::get().into()
  }
  let _ = pass_as_type_param(mock_entropy_source);
}

#[test]
fn entropy_source_as_impl_trait() {
  fn pass_as_impl_trait(f: impl EntropySource) -> RawEntropySource {
    f.into()
  }
  let _ = pass_as_impl_trait(mock_entropy_source);
}

#[test]
fn entropy_source_as_impl_macro() {
  fn pass_as_impl_macro(f: impl_entropy_source!()) -> RawEntropySource {
    f.into()
  }
  let _ = pass_as_impl_macro(mock_entropy_source);
  let _ = pass_as_impl_macro(|_buffer| unimplemented!());
}

/// ReturnAddressLocationResolver is used as a callback function when v8 is
/// resolving the location of a return address on the stack. Profilers that
/// change the return address on the stack can use this to resolve the stack
/// location to wherever the profiler stashed the original return address.
///
/// \param return_addr_location A location on stack where a machine
///    return address resides.
/// \returns Either return_addr_location, or else a pointer to the profiler's
///    copy of the original return address.
///
/// \note The resolver function must not cause garbage collection.
///
/// # Example
///
/// ```
/// fn return_address_location_resolver_example(
///   return_addr_location: usize,
/// ) -> usize {
///   todo!()
/// }
/// ```
pub trait ReturnAddressLocationResolver:
  UnitType + FnOnce(usize) -> usize
{
}

impl<F> ReturnAddressLocationResolver for F where
  F: UnitType + FnOnce(usize) -> usize
{
}

#[macro_export]
macro_rules! impl_return_address_location_resolver {
  () => {
    impl $crate::ReturnAddressLocationResolver
    + ::std::ops::FnOnce(
      usize,
    ) -> usize
  };
}

#[repr(transparent)]
pub(crate) struct RawReturnAddressLocationResolver(
  extern "C" fn(usize) -> usize,
);

impl<F: ReturnAddressLocationResolver> From<F>
  for RawReturnAddressLocationResolver
{
  fn from(_: F) -> Self {
    #[inline(always)]
    extern "C" fn adapter<F: ReturnAddressLocationResolver>(
      return_addr_location: usize,
    ) -> usize {
      (F::get())(return_addr_location)
    }

    Self(adapter::<F>)
  }
}

#[cfg(test)]
fn mock_return_address_location_resolver(
  _return_addr_location: usize,
) -> usize {
  unimplemented!()
}

#[test]
fn return_address_location_resolver_as_type_param() {
  fn pass_as_type_param<F: ReturnAddressLocationResolver>(
    _: F,
  ) -> RawReturnAddressLocationResolver {
    F::get().into()
  }
  let _ = pass_as_type_param(mock_return_address_location_resolver);
}

#[test]
fn return_address_location_resolver_as_impl_trait() {
  fn pass_as_impl_trait(
    f: impl ReturnAddressLocationResolver,
  ) -> RawReturnAddressLocationResolver {
    f.into()
  }
  let _ = pass_as_impl_trait(mock_return_address_location_resolver);
}

#[test]
fn return_address_location_resolver_as_impl_macro() {
  fn pass_as_impl_macro(
    f: impl_return_address_location_resolver!(),
  ) -> RawReturnAddressLocationResolver {
    f.into()
  }
  let _ = pass_as_impl_macro(mock_return_address_location_resolver);
  let _ = pass_as_impl_macro(|_return_addr_location| unimplemented!());
}

/// If callback is set, abort any attempt to execute JavaScript in this
/// context, call the specified callback, and throw an exception.
/// To unset abort, pass nullptr as callback.
///
/// # Example
///
/// ```
/// # use rusty_v8 as v8;
/// #
/// fn abort_script_execution_callback_example<'s>(
///   scope: &mut v8::HandleScope<'s>,
/// ) {
///   todo!();
/// }
/// ```
pub trait AbortScriptExecutionCallback:
  UnitType + for<'s> FnOnce(&mut HandleScope<'s>)
{
}

impl<F> AbortScriptExecutionCallback for F where
  F: UnitType + for<'s> FnOnce(&mut HandleScope<'s>)
{
}

#[macro_export]
macro_rules! impl_abort_script_execution_callback {
  () => {
    impl $crate::AbortScriptExecutionCallback
    + for<'__s> ::std::ops::FnOnce(
      &mut $crate::HandleScope<'__s>,
    )
  };
}

#[repr(transparent)]
pub(crate) struct RawAbortScriptExecutionCallback(
  for<'s> extern "C" fn(*mut Isolate, Local<'s, Context>),
);

impl<F: AbortScriptExecutionCallback> From<F>
  for RawAbortScriptExecutionCallback
{
  fn from(_: F) -> Self {
    #[inline(always)]
    extern "C" fn adapter<'s, F: AbortScriptExecutionCallback>(
      _isolate: *mut Isolate,
      context: Local<'s, Context>,
    ) {
      let scope = &mut unsafe { CallbackScope::new(context) };
      (F::get())(scope)
    }

    Self(adapter::<F>)
  }
}

#[cfg(test)]
fn mock_abort_script_execution_callback<'s>(_scope: &mut HandleScope<'s>) {
  unimplemented!()
}

#[test]
fn abort_script_execution_callback_as_type_param() {
  fn pass_as_type_param<F: AbortScriptExecutionCallback>(
    _: F,
  ) -> RawAbortScriptExecutionCallback {
    F::get().into()
  }
  let _ = pass_as_type_param(mock_abort_script_execution_callback);
}

#[test]
fn abort_script_execution_callback_as_impl_trait() {
  fn pass_as_impl_trait(
    f: impl AbortScriptExecutionCallback,
  ) -> RawAbortScriptExecutionCallback {
    f.into()
  }
  let _ = pass_as_impl_trait(mock_abort_script_execution_callback);
}

#[test]
fn abort_script_execution_callback_as_impl_macro() {
  fn pass_as_impl_macro(
    f: impl_abort_script_execution_callback!(),
  ) -> RawAbortScriptExecutionCallback {
    f.into()
  }
  let _ = pass_as_impl_macro(mock_abort_script_execution_callback);
  let _ = pass_as_impl_macro(|_scope| unimplemented!());
}
