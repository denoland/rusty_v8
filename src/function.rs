use std::convert::TryFrom;
use std::marker::PhantomData;

use crate::support::MapFnFrom;
use crate::support::MapFnTo;
use crate::support::ToCFn;
use crate::support::UnitType;
use crate::support::{int, Opaque};
use crate::CallbackScope;
use crate::Context;
use crate::Function;
use crate::HandleScope;
use crate::Local;
use crate::Name;
use crate::Object;
use crate::Value;

extern "C" {
  fn v8__Function__New(
    context: *const Context,
    callback: FunctionCallback,
  ) -> *const Function;
  fn v8__Function__NewWithData(
    context: *const Context,
    callback: FunctionCallback,
    data: *const Value,
  ) -> *const Function;
  fn v8__Function__Call(
    this: *const Function,
    context: *const Context,
    recv: *const Value,
    argc: int,
    argv: *const *const Value,
  ) -> *const Value;

  fn v8__FunctionCallbackInfo__GetReturnValue(
    info: *const FunctionCallbackInfo,
  ) -> *mut Value;
  fn v8__FunctionCallbackInfo__This(
    this: *const FunctionCallbackInfo,
  ) -> *const Object;
  fn v8__FunctionCallbackInfo__Length(this: *const FunctionCallbackInfo)
    -> int;
  fn v8__FunctionCallbackInfo__GetArgument(
    this: *const FunctionCallbackInfo,
    i: int,
  ) -> *const Value;
  fn v8__FunctionCallbackInfo__Data(
    this: *const FunctionCallbackInfo,
  ) -> *const Value;

  fn v8__PropertyCallbackInfo__GetReturnValue(
    this: *const PropertyCallbackInfo,
  ) -> *mut Value;
  fn v8__PropertyCallbackInfo__This(
    this: *const PropertyCallbackInfo,
  ) -> *const Object;

  fn v8__ReturnValue__Set(this: *mut ReturnValue, value: *const Value);
  fn v8__ReturnValue__Get(this: *const ReturnValue) -> *const Value;
}

// Npte: the 'cb lifetime is required because the ReturnValue object must not
// outlive the FunctionCallbackInfo/PropertyCallbackInfo object from which it
// is derived.
#[repr(C)]
#[derive(Debug)]
pub struct ReturnValue<'cb>(*mut Value, PhantomData<&'cb ()>);

/// In V8 ReturnValue<> has a type parameter, but
/// it turns out that in most of the APIs it's ReturnValue<Value>
/// and for our purposes we currently don't need
/// other types. So for now it's a simplified version.
impl<'cb> ReturnValue<'cb> {
  fn from_function_callback_info(info: *const FunctionCallbackInfo) -> Self {
    let slot = unsafe { v8__FunctionCallbackInfo__GetReturnValue(info) };
    Self(slot, PhantomData)
  }

  fn from_property_callback_info(info: *const PropertyCallbackInfo) -> Self {
    let slot = unsafe { v8__PropertyCallbackInfo__GetReturnValue(info) };
    Self(slot, PhantomData)
  }

  // NOTE: simplest setter, possibly we'll need to add
  // more setters specialized per type
  pub fn set(&mut self, value: Local<Value>) {
    unsafe { v8__ReturnValue__Set(&mut *self, &*value) }
  }

  /// Getter. Creates a new Local<> so it comes with a certain performance
  /// hit. If the ReturnValue was not yet set, this will return the undefined
  /// value.
  pub fn get<'s>(&self, scope: &mut HandleScope<'s>) -> Local<'s, Value> {
    unsafe { scope.cast_local(|_| v8__ReturnValue__Get(self)) }.unwrap()
  }
}

/// The argument information given to function call callbacks.  This
/// class provides access to information about the context of the call,
/// including the receiver, the number and values of arguments, and
/// the holder of the function.
#[repr(C)]
#[derive(Debug)]
pub struct FunctionCallbackInfo {
  // The layout of this struct must match that of `class FunctionCallbackInfo`
  // as defined in v8.h.
  implicit_args: *mut Opaque,
  values: *const Value,
  length: int,
}

/// The information passed to a property callback about the context
/// of the property access.
#[repr(C)]
#[derive(Debug)]
pub struct PropertyCallbackInfo {
  // The layout of this struct must match that of `class PropertyCallbackInfo`
  // as defined in v8.h.
  args: *mut Opaque,
}

#[derive(Debug)]
pub struct FunctionCallbackArguments<'s> {
  info: *const FunctionCallbackInfo,
  phantom: PhantomData<&'s ()>,
}

impl<'s> FunctionCallbackArguments<'s> {
  fn from_function_callback_info(info: *const FunctionCallbackInfo) -> Self {
    Self {
      info,
      phantom: PhantomData,
    }
  }

  /// Returns the receiver. This corresponds to the "this" value.
  pub fn this(&self) -> Local<'s, Object> {
    unsafe {
      Local::from_raw(v8__FunctionCallbackInfo__This(self.info)).unwrap()
    }
  }

  /// Returns the data argument specified when creating the callback.
  pub fn data(&self) -> Option<Local<'s, Value>> {
    unsafe { Local::from_raw(v8__FunctionCallbackInfo__Data(self.info)) }
  }

  /// The number of available arguments.
  pub fn length(&self) -> int {
    unsafe {
      let length = (*self.info).length;
      debug_assert_eq!(length, v8__FunctionCallbackInfo__Length(self.info));
      length
    }
  }

  /// Accessor for the available arguments. Returns `undefined` if the index is
  /// out of bounds.
  pub fn get(&self, i: int) -> Local<'s, Value> {
    unsafe {
      Local::from_raw(v8__FunctionCallbackInfo__GetArgument(self.info, i))
        .unwrap()
    }
  }
}

#[derive(Debug)]
pub struct PropertyCallbackArguments<'s> {
  info: *const PropertyCallbackInfo,
  phantom: PhantomData<&'s ()>,
}

impl<'s> PropertyCallbackArguments<'s> {
  fn from_property_callback_info(info: *const PropertyCallbackInfo) -> Self {
    Self {
      info,
      phantom: PhantomData,
    }
  }

  /// Returns the receiver. In many cases, this is the object on which the
  /// property access was intercepted. When using
  /// `Reflect.get`, `Function.prototype.call`, or similar functions, it is the
  /// object passed in as receiver or thisArg.
  ///
  /// ```c++
  ///   void GetterCallback(Local<Name> name,
  ///                       const v8::PropertyCallbackInfo<v8::Value>& info) {
  ///      auto context = info.GetIsolate()->GetCurrentContext();
  ///
  ///      v8::Local<v8::Value> a_this =
  ///          info.This()
  ///              ->GetRealNamedProperty(context, v8_str("a"))
  ///              .ToLocalChecked();
  ///      v8::Local<v8::Value> a_holder =
  ///          info.Holder()
  ///              ->GetRealNamedProperty(context, v8_str("a"))
  ///              .ToLocalChecked();
  ///
  ///     CHECK(v8_str("r")->Equals(context, a_this).FromJust());
  ///     CHECK(v8_str("obj")->Equals(context, a_holder).FromJust());
  ///
  ///     info.GetReturnValue().Set(name);
  ///   }
  ///
  ///   v8::Local<v8::FunctionTemplate> templ =
  ///   v8::FunctionTemplate::New(isolate);
  ///   templ->InstanceTemplate()->SetHandler(
  ///       v8::NamedPropertyHandlerConfiguration(GetterCallback));
  ///   LocalContext env;
  ///   env->Global()
  ///       ->Set(env.local(), v8_str("obj"), templ->GetFunction(env.local())
  ///                                            .ToLocalChecked()
  ///                                            ->NewInstance(env.local())
  ///                                            .ToLocalChecked())
  ///       .FromJust();
  ///
  ///   CompileRun("obj.a = 'obj'; var r = {a: 'r'}; Reflect.get(obj, 'x', r)");
  /// ```
  pub fn this(&self) -> Local<'s, Object> {
    unsafe {
      Local::from_raw(v8__PropertyCallbackInfo__This(self.info)).unwrap()
    }
  }
}

pub type FunctionCallback = extern "C" fn(*const FunctionCallbackInfo);

impl<F> MapFnFrom<F> for FunctionCallback
where
  F: UnitType + Fn(&mut HandleScope, FunctionCallbackArguments, ReturnValue),
{
  fn mapping() -> Self {
    let f = |info: *const FunctionCallbackInfo| {
      let scope = &mut unsafe { CallbackScope::new(&*info) };
      let args = FunctionCallbackArguments::from_function_callback_info(info);
      let rv = ReturnValue::from_function_callback_info(info);
      (F::get())(scope, args, rv);
    };
    f.to_c_fn()
  }
}

/// AccessorNameGetterCallback is used as callback functions when getting a
/// particular property. See Object and ObjectTemplate's method SetAccessor.
pub type AccessorNameGetterCallback<'s> =
  extern "C" fn(Local<'s, Name>, *const PropertyCallbackInfo);

impl<F> MapFnFrom<F> for AccessorNameGetterCallback<'_>
where
  F: UnitType
    + Fn(&mut HandleScope, Local<Name>, PropertyCallbackArguments, ReturnValue),
{
  fn mapping() -> Self {
    let f = |key: Local<Name>, info: *const PropertyCallbackInfo| {
      let scope = &mut unsafe { CallbackScope::new(&*info) };
      let args = PropertyCallbackArguments::from_property_callback_info(info);
      let rv = ReturnValue::from_property_callback_info(info);
      (F::get())(scope, key, args, rv);
    };
    f.to_c_fn()
  }
}

pub type AccessorNameSetterCallback<'s> =
  extern "C" fn(Local<'s, Name>, Local<'s, Value>, *const PropertyCallbackInfo);

impl<F> MapFnFrom<F> for AccessorNameSetterCallback<'_>
where
  F: UnitType
    + Fn(&mut HandleScope, Local<Name>, Local<Value>, PropertyCallbackArguments),
{
  fn mapping() -> Self {
    let f = |key: Local<Name>,
             value: Local<Value>,
             info: *const PropertyCallbackInfo| {
      let scope = &mut unsafe { CallbackScope::new(&*info) };
      let args = PropertyCallbackArguments::from_property_callback_info(info);
      (F::get())(scope, key, value, args);
    };
    f.to_c_fn()
  }
}

impl Function {
  // TODO: add remaining arguments from C++
  /// Create a function in the current execution context
  /// for a given FunctionCallback.
  pub fn new<'s>(
    scope: &mut HandleScope<'s>,
    callback: impl MapFnTo<FunctionCallback>,
  ) -> Option<Local<'s, Function>> {
    unsafe {
      scope.cast_local(|sd| {
        v8__Function__New(sd.get_current_context(), callback.map_fn_to())
      })
    }
  }

  /// Create a function in the current execution context
  /// for a given FunctionCallback and associated data.
  pub fn new_with_data<'s>(
    scope: &mut HandleScope<'s>,
    data: Local<Value>,
    callback: impl MapFnTo<FunctionCallback>,
  ) -> Option<Local<'s, Function>> {
    unsafe {
      scope.cast_local(|sd| {
        v8__Function__NewWithData(
          sd.get_current_context(),
          callback.map_fn_to(),
          &*data,
        )
      })
    }
  }

  pub fn call<'s>(
    &self,
    scope: &mut HandleScope<'s>,
    recv: Local<Value>,
    args: &[Local<Value>],
  ) -> Option<Local<'s, Value>> {
    let args = Local::slice_into_raw(args);
    let argc = int::try_from(args.len()).unwrap();
    let argv = args.as_ptr();
    unsafe {
      scope.cast_local(|sd| {
        v8__Function__Call(self, sd.get_current_context(), &*recv, argc, argv)
      })
    }
  }
}
