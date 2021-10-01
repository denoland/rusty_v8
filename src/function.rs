use std::convert::TryFrom;
use std::marker::PhantomData;
use std::ptr::null;

use crate::scope::CallbackScope;
use crate::support::MapFnFrom;
use crate::support::MapFnTo;
use crate::support::ToCFn;
use crate::support::UnitType;
use crate::support::{int, Opaque};
use crate::Context;
use crate::Function;
use crate::HandleScope;
use crate::Local;
use crate::Name;
use crate::Object;
use crate::Signature;
use crate::String;
use crate::Value;

extern "C" {
  fn v8__Function__New(
    context: *const Context,
    callback: FunctionCallback,
    data_or_null: *const Value,
    length: i32,
    constructor_behavior: ConstructorBehavior,
    side_effect_type: SideEffectType,
  ) -> *const Function;
  fn v8__Function__Call(
    this: *const Function,
    context: *const Context,
    recv: *const Value,
    argc: int,
    argv: *const *const Value,
  ) -> *const Value;
  fn v8__Function__NewInstance(
    this: *const Function,
    context: *const Context,
    argc: int,
    argv: *const *const Value,
  ) -> *const Object;
  fn v8__Function__GetName(this: *const Function) -> *const String;
  fn v8__Function__SetName(this: *const Function, name: *const String);

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

// Ad-libbed - V8 does not document ConstructorBehavior.
/// ConstructorBehavior::Allow creates a regular API function.
///
/// ConstructorBehavior::Throw creates a "concise" API function, a function
/// without a ".prototype" property, that is somewhat faster to create and has
/// a smaller footprint. Functionally equivalent to ConstructorBehavior::Allow
/// followed by a call to FunctionTemplate::RemovePrototype().
#[repr(C)]
pub enum ConstructorBehavior {
  Throw,
  Allow,
}

/// Options for marking whether callbacks may trigger JS-observable side
/// effects. Side-effect-free callbacks are allowlisted during debug evaluation
/// with throwOnSideEffect. It applies when calling a Function,
/// FunctionTemplate, or an Accessor callback. For Interceptors, please see
/// PropertyHandlerFlags's kHasNoSideEffect.
/// Callbacks that only cause side effects to the receiver are allowlisted if
/// invoked on receiver objects that are created within the same debug-evaluate
/// call, as these objects are temporary and the side effect does not escape.
#[repr(C)]
pub enum SideEffectType {
  HasSideEffect,
  HasNoSideEffect,
  HasSideEffectToReceiver,
}

#[repr(C)]
#[derive(Default)]
pub(crate) struct CFunction([usize; 2]);

// Note: the 'cb lifetime is required because the ReturnValue object must not
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
  pub(crate) fn from_function_callback_info(
    info: *const FunctionCallbackInfo,
  ) -> Self {
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
  pub(crate) fn from_property_callback_info(
    info: *const PropertyCallbackInfo,
  ) -> Self {
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

/// A builder to construct the properties of a Function or FunctionTemplate.
pub struct FunctionBuilder<'s, T> {
  pub(crate) callback: FunctionCallback,
  pub(crate) data: Option<Local<'s, Value>>,
  pub(crate) signature: Option<Local<'s, Signature>>,
  pub(crate) length: i32,
  pub(crate) constructor_behavior: ConstructorBehavior,
  pub(crate) side_effect_type: SideEffectType,
  phantom: PhantomData<T>,
}

impl<'s, T> FunctionBuilder<'s, T> {
  /// Create a new FunctionBuilder.
  pub fn new(callback: impl MapFnTo<FunctionCallback>) -> Self {
    Self {
      callback: callback.map_fn_to(),
      data: None,
      signature: None,
      length: 0,
      constructor_behavior: ConstructorBehavior::Allow,
      side_effect_type: SideEffectType::HasSideEffect,
      phantom: PhantomData,
    }
  }

  /// Set the associated data. The default is no associated data.
  pub fn data(mut self, data: Local<'s, Value>) -> Self {
    self.data = Some(data);
    self
  }

  /// Set the function length. The default is 0.
  pub fn length(mut self, length: i32) -> Self {
    self.length = length;
    self
  }

  /// Set the constructor behavior. The default is ConstructorBehavior::Allow.
  pub fn constructor_behavior(
    mut self,
    constructor_behavior: ConstructorBehavior,
  ) -> Self {
    self.constructor_behavior = constructor_behavior;
    self
  }

  /// Set the side effect type. The default is SideEffectType::HasSideEffect.
  pub fn side_effect_type(mut self, side_effect_type: SideEffectType) -> Self {
    self.side_effect_type = side_effect_type;
    self
  }
}

impl<'s> FunctionBuilder<'s, Function> {
  /// Create the function in the current execution context.
  pub fn build(
    self,
    scope: &mut HandleScope<'s>,
  ) -> Option<Local<'s, Function>> {
    unsafe {
      scope.cast_local(|sd| {
        v8__Function__New(
          sd.get_current_context(),
          self.callback,
          self.data.map_or_else(null, |p| &*p),
          self.length,
          self.constructor_behavior,
          self.side_effect_type,
        )
      })
    }
  }
}

impl Function {
  /// Create a FunctionBuilder to configure a Function.
  /// This is the same as FunctionBuilder::<Function>::new().
  pub fn builder<'s>(
    callback: impl MapFnTo<FunctionCallback>,
  ) -> FunctionBuilder<'s, Self> {
    FunctionBuilder::new(callback)
  }

  /// Create a function in the current execution context
  /// for a given FunctionCallback.
  pub fn new<'s>(
    scope: &mut HandleScope<'s>,
    callback: impl MapFnTo<FunctionCallback>,
  ) -> Option<Local<'s, Function>> {
    Self::builder(callback).build(scope)
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

  pub fn new_instance<'s>(
    &self,
    scope: &mut HandleScope<'s>,
    args: &[Local<Value>],
  ) -> Option<Local<'s, Object>> {
    let args = Local::slice_into_raw(args);
    let argc = int::try_from(args.len()).unwrap();
    let argv = args.as_ptr();
    unsafe {
      scope.cast_local(|sd| {
        v8__Function__NewInstance(self, sd.get_current_context(), argc, argv)
      })
    }
  }

  pub fn get_name<'s>(&self, scope: &mut HandleScope<'s>) -> Local<'s, String> {
    unsafe { scope.cast_local(|_| v8__Function__GetName(self)).unwrap() }
  }

  pub fn set_name(&self, name: Local<String>) {
    unsafe { v8__Function__SetName(self, &*name) }
  }
}
