// Copyright 2019-2020 the Deno authors. All rights reserved. MIT license.

use std::convert::From;
use std::convert::TryFrom;
use std::error::Error;
use std::fmt;
use std::fmt::Display;
use std::fmt::Formatter;
use std::mem::transmute;
use std::ops::Deref;

use crate::support::Opaque;
use crate::Local;

macro_rules! impl_deref {
  { $target:ident for $type:ident } => {
    impl Deref for $type {
      type Target = $target;
      fn deref(&self) -> &Self::Target {
        unsafe { &*(self as *const _ as *const Self::Target) }
      }
    }
  };
}

macro_rules! impl_from {
  { $source:ident for $type:ident } => {
    impl<'sc> From<Local< $source>> for Local<$type> {
      fn from(l: Local< $source>) -> Self {
        unsafe { transmute(l) }
      }
    }
  };
}

macro_rules! impl_try_from {
  { $source:ident for $target:ident if $value:pat => $check:expr } => {
    impl<'sc> TryFrom<Local< $source>> for Local<$target> {
      type Error = TryFromTypeError;
      fn try_from(l: Local< $source>) -> Result<Self, Self::Error> {
        match l {
          $value if $check => Ok(unsafe { transmute(l) }),
          _ => Err(TryFromTypeError::new(stringify!($target)))
        }
      }
    }
  };
}

#[derive(Clone, Copy, Debug)]
pub struct TryFromTypeError {
  expected_type: &'static str,
}

impl TryFromTypeError {
  fn new(expected_type: &'static str) -> Self {
    Self { expected_type }
  }
}

impl Display for TryFromTypeError {
  fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
    write!(f, "{} expected", self.expected_type)
  }
}

impl Error for TryFromTypeError {}

/// The superclass of objects that can reside on V8's heap.
#[repr(C)]
pub struct Data(Opaque);

impl_from! { AccessorSignature for Data }
impl_from! { Module for Data }
impl_from! { Private for Data }
impl_from! { Signature for Data }
impl_from! { Template for Data }
impl_from! { FunctionTemplate for Data }
impl_from! { ObjectTemplate for Data }
impl_from! { UnboundModuleScript for Data }
impl_from! { Value for Data }
impl_from! { External for Data }
impl_from! { Object for Data }
impl_from! { Array for Data }
impl_from! { ArrayBuffer for Data }
impl_from! { ArrayBufferView for Data }
impl_from! { DataView for Data }
impl_from! { TypedArray for Data }
impl_from! { BigInt64Array for Data }
impl_from! { BigUint64Array for Data }
impl_from! { Float32Array for Data }
impl_from! { Float64Array for Data }
impl_from! { Int16Array for Data }
impl_from! { Int32Array for Data }
impl_from! { Int8Array for Data }
impl_from! { Uint16Array for Data }
impl_from! { Uint32Array for Data }
impl_from! { Uint8Array for Data }
impl_from! { Uint8ClampedArray for Data }
impl_from! { BigIntObject for Data }
impl_from! { BooleanObject for Data }
impl_from! { Date for Data }
impl_from! { FinalizationGroup for Data }
impl_from! { Function for Data }
impl_from! { Map for Data }
impl_from! { NumberObject for Data }
impl_from! { Promise for Data }
impl_from! { PromiseResolver for Data }
impl_from! { Proxy for Data }
impl_from! { RegExp for Data }
impl_from! { Set for Data }
impl_from! { SharedArrayBuffer for Data }
impl_from! { StringObject for Data }
impl_from! { SymbolObject for Data }
impl_from! { WasmModuleObject for Data }
impl_from! { Primitive for Data }
impl_from! { BigInt for Data }
impl_from! { Boolean for Data }
impl_from! { Name for Data }
impl_from! { String for Data }
impl_from! { Symbol for Data }
impl_from! { Number for Data }
impl_from! { Integer for Data }
impl_from! { Int32 for Data }
impl_from! { Uint32 for Data }

/// An AccessorSignature specifies which receivers are valid parameters
/// to an accessor callback.
#[repr(C)]
pub struct AccessorSignature(Opaque);

impl_deref! { Data for AccessorSignature }

/// A compiled JavaScript module.
#[repr(C)]
pub struct Module(Opaque);

impl_deref! { Data for Module }

/// A private symbol
///
/// This is an experimental feature. Use at your own risk.
#[repr(C)]
pub struct Private(Opaque);

impl_deref! { Data for Private }

/// A Signature specifies which receiver is valid for a function.
///
/// A receiver matches a given signature if the receiver (or any of its
/// hidden prototypes) was created from the signature's FunctionTemplate, or
/// from a FunctionTemplate that inherits directly or indirectly from the
/// signature's FunctionTemplate.
#[repr(C)]
pub struct Signature(Opaque);

impl_deref! { Data for Signature }

/// The superclass of object and function templates.
#[repr(C)]
pub struct Template(Opaque);

impl_deref! { Data for Template }
impl_from! { FunctionTemplate for Template }
impl_from! { ObjectTemplate for Template }

/// A FunctionTemplate is used to create functions at runtime. There
/// can only be one function created from a FunctionTemplate in a
/// context. The lifetime of the created function is equal to the
/// lifetime of the context. So in case the embedder needs to create
/// temporary functions that can be collected using Scripts is
/// preferred.
///
/// Any modification of a FunctionTemplate after first instantiation will trigger
/// a crash.
///
/// A FunctionTemplate can have properties, these properties are added to the
/// function object when it is created.
///
/// A FunctionTemplate has a corresponding instance template which is
/// used to create object instances when the function is used as a
/// constructor. Properties added to the instance template are added to
/// each object instance.
///
/// A FunctionTemplate can have a prototype template. The prototype template
/// is used to create the prototype object of the function.
///
/// The following example shows how to use a FunctionTemplate:
///
/// ```ignore
///    v8::Local<v8::FunctionTemplate> t = v8::FunctionTemplate::New(isolate);
///    t->Set(isolate, "func_property", v8::Number::New(isolate, 1));
///
///    v8::Local<v8::Template> proto_t = t->PrototypeTemplate();
///    proto_t->Set(isolate,
///                 "proto_method",
///                 v8::FunctionTemplate::New(isolate, InvokeCallback));
///    proto_t->Set(isolate, "proto_const", v8::Number::New(isolate, 2));
///
///    v8::Local<v8::ObjectTemplate> instance_t = t->InstanceTemplate();
///    instance_t->SetAccessor(String::NewFromUtf8(isolate, "instance_accessor"),
///                            InstanceAccessorCallback);
///    instance_t->SetHandler(
///        NamedPropertyHandlerConfiguration(PropertyHandlerCallback));
///    instance_t->Set(String::NewFromUtf8(isolate, "instance_property"),
///                    Number::New(isolate, 3));
///
///    v8::Local<v8::Function> function = t->GetFunction();
///    v8::Local<v8::Object> instance = function->NewInstance();
/// ```
///
/// Let's use "function" as the JS variable name of the function object
/// and "instance" for the instance object created above. The function
/// and the instance will have the following properties:
///
/// ```ignore
///   func_property in function == true;
///   function.func_property == 1;
///
///   function.prototype.proto_method() invokes 'InvokeCallback'
///   function.prototype.proto_const == 2;
///
///   instance instanceof function == true;
///   instance.instance_accessor calls 'InstanceAccessorCallback'
///   instance.instance_property == 3;
/// ```
///
/// A FunctionTemplate can inherit from another one by calling the
/// FunctionTemplate::Inherit method. The following graph illustrates
/// the semantics of inheritance:
///
/// ```ignore
///   FunctionTemplate Parent  -> Parent() . prototype -> { }
///     ^                                                  ^
///     | Inherit(Parent)                                  | .__proto__
///     |                                                  |
///   FunctionTemplate Child   -> Child()  . prototype -> { }
/// ```
///
/// A FunctionTemplate 'Child' inherits from 'Parent', the prototype
/// object of the Child() function has __proto__ pointing to the
/// Parent() function's prototype object. An instance of the Child
/// function has all properties on Parent's instance templates.
///
/// Let Parent be the FunctionTemplate initialized in the previous
/// section and create a Child FunctionTemplate by:
///
/// ```ignore
///   Local<FunctionTemplate> parent = t;
///   Local<FunctionTemplate> child = FunctionTemplate::New();
///   child->Inherit(parent);
///
///   Local<Function> child_function = child->GetFunction();
///   Local<Object> child_instance = child_function->NewInstance();
/// ```
///
/// The Child function and Child instance will have the following
/// properties:
///
/// ```ignore
///   child_func.prototype.__proto__ == function.prototype;
///   child_instance.instance_accessor calls 'InstanceAccessorCallback'
///   child_instance.instance_property == 3;
/// ```
#[repr(C)]
pub struct FunctionTemplate(Opaque);

impl_deref! { Template for FunctionTemplate }

/// An ObjectTemplate is used to create objects at runtime.
///
/// Properties added to an ObjectTemplate are added to each object
/// created from the ObjectTemplate.
#[repr(C)]
pub struct ObjectTemplate(Opaque);

impl_deref! { Template for ObjectTemplate }

/// A compiled JavaScript module, not yet tied to a Context.
#[repr(C)]
pub struct UnboundModuleScript(Opaque);

impl_deref! { Data for UnboundModuleScript }

/// The superclass of all JavaScript values and objects.
#[repr(C)]
pub struct Value(Opaque);

impl_deref! { Data for Value }
impl_from! { External for Value }
impl_from! { Object for Value }
impl_from! { Array for Value }
impl_from! { ArrayBuffer for Value }
impl_from! { ArrayBufferView for Value }
impl_from! { DataView for Value }
impl_from! { TypedArray for Value }
impl_from! { BigInt64Array for Value }
impl_from! { BigUint64Array for Value }
impl_from! { Float32Array for Value }
impl_from! { Float64Array for Value }
impl_from! { Int16Array for Value }
impl_from! { Int32Array for Value }
impl_from! { Int8Array for Value }
impl_from! { Uint16Array for Value }
impl_from! { Uint32Array for Value }
impl_from! { Uint8Array for Value }
impl_from! { Uint8ClampedArray for Value }
impl_from! { BigIntObject for Value }
impl_from! { BooleanObject for Value }
impl_from! { Date for Value }
impl_from! { FinalizationGroup for Value }
impl_from! { Function for Value }
impl_from! { Map for Value }
impl_from! { NumberObject for Value }
impl_from! { Promise for Value }
impl_from! { PromiseResolver for Value }
impl_from! { Proxy for Value }
impl_from! { RegExp for Value }
impl_from! { Set for Value }
impl_from! { SharedArrayBuffer for Value }
impl_from! { StringObject for Value }
impl_from! { SymbolObject for Value }
impl_from! { WasmModuleObject for Value }
impl_from! { Primitive for Value }
impl_from! { BigInt for Value }
impl_from! { Boolean for Value }
impl_from! { Name for Value }
impl_from! { String for Value }
impl_from! { Symbol for Value }
impl_from! { Number for Value }
impl_from! { Integer for Value }
impl_from! { Int32 for Value }
impl_from! { Uint32 for Value }

/// A JavaScript value that wraps a C++ void*. This type of value is mainly used
/// to associate C++ data structures with JavaScript objects.
#[repr(C)]
pub struct External(Opaque);

impl_deref! { Value for External }
impl_try_from! { Value for External if v => v.is_external() }

/// A JavaScript object (ECMA-262, 4.3.3)
#[repr(C)]
pub struct Object(Opaque);

impl_deref! { Value for Object }
impl_try_from! { Value for Object if v => v.is_object() }
impl_from! { Array for Object }
impl_from! { ArrayBuffer for Object }
impl_from! { ArrayBufferView for Object }
impl_from! { DataView for Object }
impl_from! { TypedArray for Object }
impl_from! { BigInt64Array for Object }
impl_from! { BigUint64Array for Object }
impl_from! { Float32Array for Object }
impl_from! { Float64Array for Object }
impl_from! { Int16Array for Object }
impl_from! { Int32Array for Object }
impl_from! { Int8Array for Object }
impl_from! { Uint16Array for Object }
impl_from! { Uint32Array for Object }
impl_from! { Uint8Array for Object }
impl_from! { Uint8ClampedArray for Object }
impl_from! { BigIntObject for Object }
impl_from! { BooleanObject for Object }
impl_from! { Date for Object }
impl_from! { FinalizationGroup for Object }
impl_from! { Function for Object }
impl_from! { Map for Object }
impl_from! { NumberObject for Object }
impl_from! { Promise for Object }
impl_from! { PromiseResolver for Object }
impl_from! { Proxy for Object }
impl_from! { RegExp for Object }
impl_from! { Set for Object }
impl_from! { SharedArrayBuffer for Object }
impl_from! { StringObject for Object }
impl_from! { SymbolObject for Object }
impl_from! { WasmModuleObject for Object }

/// An instance of the built-in array constructor (ECMA-262, 15.4.2).
#[repr(C)]
pub struct Array(Opaque);

impl_deref! { Object for Array }
impl_try_from! { Value for Array if v => v.is_array() }
impl_try_from! { Object for Array if v => v.is_array() }

/// An instance of the built-in ArrayBuffer constructor (ES6 draft 15.13.5).
#[repr(C)]
pub struct ArrayBuffer(Opaque);

impl_deref! { Object for ArrayBuffer }
impl_try_from! { Value for ArrayBuffer if v => v.is_array_buffer() }
impl_try_from! { Object for ArrayBuffer if v => v.is_array_buffer() }

/// A base class for an instance of one of "views" over ArrayBuffer,
/// including TypedArrays and DataView (ES6 draft 15.13).
#[repr(C)]
pub struct ArrayBufferView(Opaque);

impl_deref! { Object for ArrayBufferView }
impl_try_from! { Value for ArrayBufferView if v => v.is_array_buffer_view() }
impl_try_from! { Object for ArrayBufferView if v => v.is_array_buffer_view() }
impl_from! { DataView for ArrayBufferView }
impl_from! { TypedArray for ArrayBufferView }
impl_from! { BigInt64Array for ArrayBufferView }
impl_from! { BigUint64Array for ArrayBufferView }
impl_from! { Float32Array for ArrayBufferView }
impl_from! { Float64Array for ArrayBufferView }
impl_from! { Int16Array for ArrayBufferView }
impl_from! { Int32Array for ArrayBufferView }
impl_from! { Int8Array for ArrayBufferView }
impl_from! { Uint16Array for ArrayBufferView }
impl_from! { Uint32Array for ArrayBufferView }
impl_from! { Uint8Array for ArrayBufferView }
impl_from! { Uint8ClampedArray for ArrayBufferView }

/// An instance of DataView constructor (ES6 draft 15.13.7).
#[repr(C)]
pub struct DataView(Opaque);

impl_deref! { ArrayBufferView for DataView }
impl_try_from! { Value for DataView if v => v.is_data_view() }
impl_try_from! { Object for DataView if v => v.is_data_view() }
impl_try_from! { ArrayBufferView for DataView if v => v.is_data_view() }

/// A base class for an instance of TypedArray series of constructors
/// (ES6 draft 15.13.6).
#[repr(C)]
pub struct TypedArray(Opaque);

impl_deref! { ArrayBufferView for TypedArray }
impl_try_from! { Value for TypedArray if v => v.is_typed_array() }
impl_try_from! { Object for TypedArray if v => v.is_typed_array() }
impl_try_from! { ArrayBufferView for TypedArray if v => v.is_typed_array() }
impl_from! { BigInt64Array for TypedArray }
impl_from! { BigUint64Array for TypedArray }
impl_from! { Float32Array for TypedArray }
impl_from! { Float64Array for TypedArray }
impl_from! { Int16Array for TypedArray }
impl_from! { Int32Array for TypedArray }
impl_from! { Int8Array for TypedArray }
impl_from! { Uint16Array for TypedArray }
impl_from! { Uint32Array for TypedArray }
impl_from! { Uint8Array for TypedArray }
impl_from! { Uint8ClampedArray for TypedArray }

/// An instance of BigInt64Array constructor.
#[repr(C)]
pub struct BigInt64Array(Opaque);

impl_deref! { TypedArray for BigInt64Array }
impl_try_from! { Value for BigInt64Array if v => v.is_big_int64_array() }
impl_try_from! { Object for BigInt64Array if v => v.is_big_int64_array() }
impl_try_from! { ArrayBufferView for BigInt64Array if v => v.is_big_int64_array() }
impl_try_from! { TypedArray for BigInt64Array if v => v.is_big_int64_array() }

/// An instance of BigUint64Array constructor.
#[repr(C)]
pub struct BigUint64Array(Opaque);

impl_deref! { TypedArray for BigUint64Array }
impl_try_from! { Value for BigUint64Array if v => v.is_big_uint64_array() }
impl_try_from! { Object for BigUint64Array if v => v.is_big_uint64_array() }
impl_try_from! { ArrayBufferView for BigUint64Array if v => v.is_big_uint64_array() }
impl_try_from! { TypedArray for BigUint64Array if v => v.is_big_uint64_array() }

/// An instance of Float32Array constructor (ES6 draft 15.13.6).
#[repr(C)]
pub struct Float32Array(Opaque);

impl_deref! { TypedArray for Float32Array }
impl_try_from! { Value for Float32Array if v => v.is_float32_array() }
impl_try_from! { Object for Float32Array if v => v.is_float32_array() }
impl_try_from! { ArrayBufferView for Float32Array if v => v.is_float32_array() }
impl_try_from! { TypedArray for Float32Array if v => v.is_float32_array() }

/// An instance of Float64Array constructor (ES6 draft 15.13.6).
#[repr(C)]
pub struct Float64Array(Opaque);

impl_deref! { TypedArray for Float64Array }
impl_try_from! { Value for Float64Array if v => v.is_float64_array() }
impl_try_from! { Object for Float64Array if v => v.is_float64_array() }
impl_try_from! { ArrayBufferView for Float64Array if v => v.is_float64_array() }
impl_try_from! { TypedArray for Float64Array if v => v.is_float64_array() }

/// An instance of Int16Array constructor (ES6 draft 15.13.6).
#[repr(C)]
pub struct Int16Array(Opaque);

impl_deref! { TypedArray for Int16Array }
impl_try_from! { Value for Int16Array if v => v.is_int16_array() }
impl_try_from! { Object for Int16Array if v => v.is_int16_array() }
impl_try_from! { ArrayBufferView for Int16Array if v => v.is_int16_array() }
impl_try_from! { TypedArray for Int16Array if v => v.is_int16_array() }

/// An instance of Int32Array constructor (ES6 draft 15.13.6).
#[repr(C)]
pub struct Int32Array(Opaque);

impl_deref! { TypedArray for Int32Array }
impl_try_from! { Value for Int32Array if v => v.is_int32_array() }
impl_try_from! { Object for Int32Array if v => v.is_int32_array() }
impl_try_from! { ArrayBufferView for Int32Array if v => v.is_int32_array() }
impl_try_from! { TypedArray for Int32Array if v => v.is_int32_array() }

/// An instance of Int8Array constructor (ES6 draft 15.13.6).
#[repr(C)]
pub struct Int8Array(Opaque);

impl_deref! { TypedArray for Int8Array }
impl_try_from! { Value for Int8Array if v => v.is_int8_array() }
impl_try_from! { Object for Int8Array if v => v.is_int8_array() }
impl_try_from! { ArrayBufferView for Int8Array if v => v.is_int8_array() }
impl_try_from! { TypedArray for Int8Array if v => v.is_int8_array() }

/// An instance of Uint16Array constructor (ES6 draft 15.13.6).
#[repr(C)]
pub struct Uint16Array(Opaque);

impl_deref! { TypedArray for Uint16Array }
impl_try_from! { Value for Uint16Array if v => v.is_uint16_array() }
impl_try_from! { Object for Uint16Array if v => v.is_uint16_array() }
impl_try_from! { ArrayBufferView for Uint16Array if v => v.is_uint16_array() }
impl_try_from! { TypedArray for Uint16Array if v => v.is_uint16_array() }

/// An instance of Uint32Array constructor (ES6 draft 15.13.6).
#[repr(C)]
pub struct Uint32Array(Opaque);

impl_deref! { TypedArray for Uint32Array }
impl_try_from! { Value for Uint32Array if v => v.is_uint32_array() }
impl_try_from! { Object for Uint32Array if v => v.is_uint32_array() }
impl_try_from! { ArrayBufferView for Uint32Array if v => v.is_uint32_array() }
impl_try_from! { TypedArray for Uint32Array if v => v.is_uint32_array() }

/// An instance of Uint8Array constructor (ES6 draft 15.13.6).
#[repr(C)]
pub struct Uint8Array(Opaque);

impl_deref! { TypedArray for Uint8Array }
impl_try_from! { Value for Uint8Array if v => v.is_uint8_array() }
impl_try_from! { Object for Uint8Array if v => v.is_uint8_array() }
impl_try_from! { ArrayBufferView for Uint8Array if v => v.is_uint8_array() }
impl_try_from! { TypedArray for Uint8Array if v => v.is_uint8_array() }

/// An instance of Uint8ClampedArray constructor (ES6 draft 15.13.6).
#[repr(C)]
pub struct Uint8ClampedArray(Opaque);

impl_deref! { TypedArray for Uint8ClampedArray }
impl_try_from! { Value for Uint8ClampedArray if v => v.is_uint8_clamped_array() }
impl_try_from! { Object for Uint8ClampedArray if v => v.is_uint8_clamped_array() }
impl_try_from! { ArrayBufferView for Uint8ClampedArray if v => v.is_uint8_clamped_array() }
impl_try_from! { TypedArray for Uint8ClampedArray if v => v.is_uint8_clamped_array() }

/// A BigInt object (https://tc39.github.io/proposal-bigint)
#[repr(C)]
pub struct BigIntObject(Opaque);

impl_deref! { Object for BigIntObject }
impl_try_from! { Value for BigIntObject if v => v.is_big_int_object() }
impl_try_from! { Object for BigIntObject if v => v.is_big_int_object() }

/// A Boolean object (ECMA-262, 4.3.15).
#[repr(C)]
pub struct BooleanObject(Opaque);

impl_deref! { Object for BooleanObject }
impl_try_from! { Value for BooleanObject if v => v.is_boolean_object() }
impl_try_from! { Object for BooleanObject if v => v.is_boolean_object() }

/// An instance of the built-in Date constructor (ECMA-262, 15.9).
#[repr(C)]
pub struct Date(Opaque);

impl_deref! { Object for Date }
impl_try_from! { Value for Date if v => v.is_date() }
impl_try_from! { Object for Date if v => v.is_date() }

/// An instance of the built-in FinalizationGroup constructor.
///
/// This API is experimental and may change significantly.
#[repr(C)]
pub struct FinalizationGroup(Opaque);

impl_deref! { Object for FinalizationGroup }

/// A JavaScript function object (ECMA-262, 15.3).
#[repr(C)]
pub struct Function(Opaque);

impl_deref! { Object for Function }
impl_try_from! { Value for Function if v => v.is_function() }
impl_try_from! { Object for Function if v => v.is_function() }

/// An instance of the built-in Map constructor (ECMA-262, 6th Edition, 23.1.1).
#[repr(C)]
pub struct Map(Opaque);

impl_deref! { Object for Map }
impl_try_from! { Value for Map if v => v.is_map() }
impl_try_from! { Object for Map if v => v.is_map() }

/// A Number object (ECMA-262, 4.3.21).
#[repr(C)]
pub struct NumberObject(Opaque);

impl_deref! { Object for NumberObject }
impl_try_from! { Value for NumberObject if v => v.is_number_object() }
impl_try_from! { Object for NumberObject if v => v.is_number_object() }

/// An instance of the built-in Promise constructor (ES6 draft).
#[repr(C)]
pub struct Promise(Opaque);

impl_deref! { Object for Promise }
impl_try_from! { Value for Promise if v => v.is_promise() }
impl_try_from! { Object for Promise if v => v.is_promise() }

#[repr(C)]
pub struct PromiseResolver(Opaque);

impl_deref! { Object for PromiseResolver }

/// An instance of the built-in Proxy constructor (ECMA-262, 6th Edition,
/// 26.2.1).
#[repr(C)]
pub struct Proxy(Opaque);

impl_deref! { Object for Proxy }
impl_try_from! { Value for Proxy if v => v.is_proxy() }
impl_try_from! { Object for Proxy if v => v.is_proxy() }

/// An instance of the built-in RegExp constructor (ECMA-262, 15.10).
#[repr(C)]
pub struct RegExp(Opaque);

impl_deref! { Object for RegExp }
impl_try_from! { Value for RegExp if v => v.is_reg_exp() }
impl_try_from! { Object for RegExp if v => v.is_reg_exp() }

/// An instance of the built-in Set constructor (ECMA-262, 6th Edition, 23.2.1).
#[repr(C)]
pub struct Set(Opaque);

impl_deref! { Object for Set }
impl_try_from! { Value for Set if v => v.is_set() }
impl_try_from! { Object for Set if v => v.is_set() }

/// An instance of the built-in SharedArrayBuffer constructor.
/// This API is experimental and may change significantly.
#[repr(C)]
pub struct SharedArrayBuffer(Opaque);

impl_deref! { Object for SharedArrayBuffer }
impl_try_from! { Value for SharedArrayBuffer if v => v.is_shared_array_buffer() }
impl_try_from! { Object for SharedArrayBuffer if v => v.is_shared_array_buffer() }

/// A String object (ECMA-262, 4.3.18).
#[repr(C)]
pub struct StringObject(Opaque);

impl_deref! { Object for StringObject }
impl_try_from! { Value for StringObject if v => v.is_string_object() }
impl_try_from! { Object for StringObject if v => v.is_string_object() }

/// A Symbol object (ECMA-262 edition 6).
#[repr(C)]
pub struct SymbolObject(Opaque);

impl_deref! { Object for SymbolObject }
impl_try_from! { Value for SymbolObject if v => v.is_symbol_object() }
impl_try_from! { Object for SymbolObject if v => v.is_symbol_object() }

#[repr(C)]
pub struct WasmModuleObject(Opaque);

impl_deref! { Object for WasmModuleObject }

/// The superclass of primitive values. See ECMA-262 4.3.2.
#[repr(C)]
pub struct Primitive(Opaque);

impl_deref! { Value for Primitive }
impl_try_from! { Value for Primitive if v => v.is_null_or_undefined() || v.is_boolean() || v.is_name() || v.is_number() || v.is_big_int() }
impl_from! { BigInt for Primitive }
impl_from! { Boolean for Primitive }
impl_from! { Name for Primitive }
impl_from! { String for Primitive }
impl_from! { Symbol for Primitive }
impl_from! { Number for Primitive }
impl_from! { Integer for Primitive }
impl_from! { Int32 for Primitive }
impl_from! { Uint32 for Primitive }

/// A JavaScript BigInt value (https://tc39.github.io/proposal-bigint)
#[repr(C)]
pub struct BigInt(Opaque);

impl_deref! { Primitive for BigInt }
impl_try_from! { Value for BigInt if v => v.is_big_int() }
impl_try_from! { Primitive for BigInt if v => v.is_big_int() }

/// A primitive boolean value (ECMA-262, 4.3.14). Either the true
/// or false value.
#[repr(C)]
pub struct Boolean(Opaque);

impl_deref! { Primitive for Boolean }
impl_try_from! { Value for Boolean if v => v.is_boolean() }
impl_try_from! { Primitive for Boolean if v => v.is_boolean() }

/// A superclass for symbols and strings.
#[repr(C)]
pub struct Name(Opaque);

impl_deref! { Primitive for Name }
impl_try_from! { Value for Name if v => v.is_name() }
impl_try_from! { Primitive for Name if v => v.is_name() }
impl_from! { String for Name }
impl_from! { Symbol for Name }

/// A JavaScript string value (ECMA-262, 4.3.17).
#[repr(C)]
pub struct String(Opaque);

impl_deref! { Name for String }
impl_try_from! { Value for String if v => v.is_string() }
impl_try_from! { Primitive for String if v => v.is_string() }
impl_try_from! { Name for String if v => v.is_string() }

/// A JavaScript symbol (ECMA-262 edition 6)
#[repr(C)]
pub struct Symbol(Opaque);

impl_deref! { Name for Symbol }
impl_try_from! { Value for Symbol if v => v.is_symbol() }
impl_try_from! { Primitive for Symbol if v => v.is_symbol() }
impl_try_from! { Name for Symbol if v => v.is_symbol() }

/// A JavaScript number value (ECMA-262, 4.3.20)
#[repr(C)]
pub struct Number(Opaque);

impl_deref! { Primitive for Number }
impl_try_from! { Value for Number if v => v.is_number() }
impl_try_from! { Primitive for Number if v => v.is_number() }
impl_from! { Integer for Number }
impl_from! { Int32 for Number }
impl_from! { Uint32 for Number }

/// A JavaScript value representing a signed integer.
#[repr(C)]
pub struct Integer(Opaque);

impl_deref! { Number for Integer }
impl_try_from! { Value for Integer if v => v.is_int32() || v.is_uint32() }
impl_try_from! { Primitive for Integer if v => v.is_int32() || v.is_uint32() }
impl_try_from! { Number for Integer if v => v.is_int32() || v.is_uint32() }
impl_from! { Int32 for Integer }
impl_from! { Uint32 for Integer }

/// A JavaScript value representing a 32-bit signed integer.
#[repr(C)]
pub struct Int32(Opaque);

impl_deref! { Integer for Int32 }
impl_try_from! { Value for Int32 if v => v.is_int32() }
impl_try_from! { Primitive for Int32 if v => v.is_int32() }
impl_try_from! { Number for Int32 if v => v.is_int32() }
impl_try_from! { Integer for Int32 if v => v.is_int32() }

/// A JavaScript value representing a 32-bit unsigned integer.
#[repr(C)]
pub struct Uint32(Opaque);

impl_deref! { Integer for Uint32 }
impl_try_from! { Value for Uint32 if v => v.is_uint32() }
impl_try_from! { Primitive for Uint32 if v => v.is_uint32() }
impl_try_from! { Number for Uint32 if v => v.is_uint32() }
impl_try_from! { Integer for Uint32 if v => v.is_uint32() }
