// Copyright 2018-2019 the Deno authors. All rights reserved. MIT license.

use std::convert::From;
use std::mem::transmute;
use std::ops::Deref;

use crate::support::Opaque;
use crate::Local;

macro_rules! impl_from {
  ($a:ident, $b:ident) => {
    impl<'sc> From<Local<'sc, $a>> for Local<'sc, $b> {
      fn from(l: Local<'sc, $a>) -> Self {
        unsafe { transmute(l) }
      }
    }
  };
}

/// The superclass of objects that can reside on V8's heap.
#[repr(C)]
pub struct Data(Opaque);

/// An AccessorSignature specifies which receivers are valid parameters
/// to an accessor callback.
#[repr(C)]
pub struct AccessorSignature(Opaque);

impl Deref for AccessorSignature {
  type Target = Data;
  fn deref(&self) -> &Self::Target {
    unsafe { &*(self as *const _ as *const Self::Target) }
  }
}

impl_from!(AccessorSignature, Data);

/// A compiled JavaScript module.
#[repr(C)]
pub struct Module(Opaque);

impl Deref for Module {
  type Target = Data;
  fn deref(&self) -> &Self::Target {
    unsafe { &*(self as *const _ as *const Self::Target) }
  }
}

impl_from!(Module, Data);

/// A private symbol
///
/// This is an experimental feature. Use at your own risk.
#[repr(C)]
pub struct Private(Opaque);

impl Deref for Private {
  type Target = Data;
  fn deref(&self) -> &Self::Target {
    unsafe { &*(self as *const _ as *const Self::Target) }
  }
}

impl_from!(Private, Data);

/// A Signature specifies which receiver is valid for a function.
///
/// A receiver matches a given signature if the receiver (or any of its
/// hidden prototypes) was created from the signature's FunctionTemplate, or
/// from a FunctionTemplate that inherits directly or indirectly from the
/// signature's FunctionTemplate.
#[repr(C)]
pub struct Signature(Opaque);

impl Deref for Signature {
  type Target = Data;
  fn deref(&self) -> &Self::Target {
    unsafe { &*(self as *const _ as *const Self::Target) }
  }
}

impl_from!(Signature, Data);

/// The superclass of object and function templates.
#[repr(C)]
pub struct Template(Opaque);

impl Deref for Template {
  type Target = Data;
  fn deref(&self) -> &Self::Target {
    unsafe { &*(self as *const _ as *const Self::Target) }
  }
}

impl_from!(Template, Data);

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

impl Deref for FunctionTemplate {
  type Target = Template;
  fn deref(&self) -> &Self::Target {
    unsafe { &*(self as *const _ as *const Self::Target) }
  }
}

impl_from!(FunctionTemplate, Data);
impl_from!(FunctionTemplate, Template);

/// An ObjectTemplate is used to create objects at runtime.
///
/// Properties added to an ObjectTemplate are added to each object
/// created from the ObjectTemplate.
#[repr(C)]
pub struct ObjectTemplate(Opaque);

impl Deref for ObjectTemplate {
  type Target = Template;
  fn deref(&self) -> &Self::Target {
    unsafe { &*(self as *const _ as *const Self::Target) }
  }
}

impl_from!(ObjectTemplate, Data);
impl_from!(ObjectTemplate, Template);

/// A compiled JavaScript module, not yet tied to a Context.
#[repr(C)]
pub struct UnboundModuleScript(Opaque);

impl Deref for UnboundModuleScript {
  type Target = Data;
  fn deref(&self) -> &Self::Target {
    unsafe { &*(self as *const _ as *const Self::Target) }
  }
}

impl_from!(UnboundModuleScript, Data);

/// The superclass of all JavaScript values and objects.
#[repr(C)]
pub struct Value(Opaque);

impl Deref for Value {
  type Target = Data;
  fn deref(&self) -> &Self::Target {
    unsafe { &*(self as *const _ as *const Self::Target) }
  }
}

impl_from!(Value, Data);

/// A JavaScript value that wraps a C++ void*. This type of value is mainly used
/// to associate C++ data structures with JavaScript objects.
#[repr(C)]
pub struct External(Opaque);

impl Deref for External {
  type Target = Value;
  fn deref(&self) -> &Self::Target {
    unsafe { &*(self as *const _ as *const Self::Target) }
  }
}

impl_from!(External, Data);
impl_from!(External, Value);

/// A JavaScript object (ECMA-262, 4.3.3)
#[repr(C)]
pub struct Object(Opaque);

impl Deref for Object {
  type Target = Value;
  fn deref(&self) -> &Self::Target {
    unsafe { &*(self as *const _ as *const Self::Target) }
  }
}

impl_from!(Object, Data);
impl_from!(Object, Value);

/// An instance of the built-in array constructor (ECMA-262, 15.4.2).
#[repr(C)]
pub struct Array(Opaque);

impl Deref for Array {
  type Target = Object;
  fn deref(&self) -> &Self::Target {
    unsafe { &*(self as *const _ as *const Self::Target) }
  }
}

impl_from!(Array, Data);
impl_from!(Array, Value);
impl_from!(Array, Object);

/// An instance of the built-in ArrayBuffer constructor (ES6 draft 15.13.5).
#[repr(C)]
pub struct ArrayBuffer(Opaque);

impl Deref for ArrayBuffer {
  type Target = Object;
  fn deref(&self) -> &Self::Target {
    unsafe { &*(self as *const _ as *const Self::Target) }
  }
}

impl_from!(ArrayBuffer, Data);
impl_from!(ArrayBuffer, Value);
impl_from!(ArrayBuffer, Object);

/// A base class for an instance of one of "views" over ArrayBuffer,
/// including TypedArrays and DataView (ES6 draft 15.13).
#[repr(C)]
pub struct ArrayBufferView(Opaque);

impl Deref for ArrayBufferView {
  type Target = Object;
  fn deref(&self) -> &Self::Target {
    unsafe { &*(self as *const _ as *const Self::Target) }
  }
}

impl_from!(ArrayBufferView, Data);
impl_from!(ArrayBufferView, Value);
impl_from!(ArrayBufferView, Object);

/// An instance of DataView constructor (ES6 draft 15.13.7).
#[repr(C)]
pub struct DataView(Opaque);

impl Deref for DataView {
  type Target = ArrayBufferView;
  fn deref(&self) -> &Self::Target {
    unsafe { &*(self as *const _ as *const Self::Target) }
  }
}

impl_from!(DataView, Data);
impl_from!(DataView, Value);
impl_from!(DataView, Object);
impl_from!(DataView, ArrayBufferView);

/// A base class for an instance of TypedArray series of constructors
/// (ES6 draft 15.13.6).
#[repr(C)]
pub struct TypedArray(Opaque);

impl Deref for TypedArray {
  type Target = ArrayBufferView;
  fn deref(&self) -> &Self::Target {
    unsafe { &*(self as *const _ as *const Self::Target) }
  }
}

impl_from!(TypedArray, Data);
impl_from!(TypedArray, Value);
impl_from!(TypedArray, Object);
impl_from!(TypedArray, ArrayBufferView);

/// An instance of BigInt64Array constructor.
#[repr(C)]
pub struct BigInt64Array(Opaque);

impl Deref for BigInt64Array {
  type Target = TypedArray;
  fn deref(&self) -> &Self::Target {
    unsafe { &*(self as *const _ as *const Self::Target) }
  }
}

impl_from!(BigInt64Array, Data);
impl_from!(BigInt64Array, Value);
impl_from!(BigInt64Array, Object);
impl_from!(BigInt64Array, ArrayBufferView);
impl_from!(BigInt64Array, TypedArray);

/// An instance of BigUint64Array constructor.
#[repr(C)]
pub struct BigUint64Array(Opaque);

impl Deref for BigUint64Array {
  type Target = TypedArray;
  fn deref(&self) -> &Self::Target {
    unsafe { &*(self as *const _ as *const Self::Target) }
  }
}

impl_from!(BigUint64Array, Data);
impl_from!(BigUint64Array, Value);
impl_from!(BigUint64Array, Object);
impl_from!(BigUint64Array, ArrayBufferView);
impl_from!(BigUint64Array, TypedArray);

/// An instance of Float32Array constructor (ES6 draft 15.13.6).
#[repr(C)]
pub struct Float32Array(Opaque);

impl Deref for Float32Array {
  type Target = TypedArray;
  fn deref(&self) -> &Self::Target {
    unsafe { &*(self as *const _ as *const Self::Target) }
  }
}

impl_from!(Float32Array, Data);
impl_from!(Float32Array, Value);
impl_from!(Float32Array, Object);
impl_from!(Float32Array, ArrayBufferView);
impl_from!(Float32Array, TypedArray);

/// An instance of Float64Array constructor (ES6 draft 15.13.6).
#[repr(C)]
pub struct Float64Array(Opaque);

impl Deref for Float64Array {
  type Target = TypedArray;
  fn deref(&self) -> &Self::Target {
    unsafe { &*(self as *const _ as *const Self::Target) }
  }
}

impl_from!(Float64Array, Data);
impl_from!(Float64Array, Value);
impl_from!(Float64Array, Object);
impl_from!(Float64Array, ArrayBufferView);
impl_from!(Float64Array, TypedArray);

/// An instance of Int16Array constructor (ES6 draft 15.13.6).
#[repr(C)]
pub struct Int16Array(Opaque);

impl Deref for Int16Array {
  type Target = TypedArray;
  fn deref(&self) -> &Self::Target {
    unsafe { &*(self as *const _ as *const Self::Target) }
  }
}

impl_from!(Int16Array, Data);
impl_from!(Int16Array, Value);
impl_from!(Int16Array, Object);
impl_from!(Int16Array, ArrayBufferView);
impl_from!(Int16Array, TypedArray);

/// An instance of Int32Array constructor (ES6 draft 15.13.6).
#[repr(C)]
pub struct Int32Array(Opaque);

impl Deref for Int32Array {
  type Target = TypedArray;
  fn deref(&self) -> &Self::Target {
    unsafe { &*(self as *const _ as *const Self::Target) }
  }
}

impl_from!(Int32Array, Data);
impl_from!(Int32Array, Value);
impl_from!(Int32Array, Object);
impl_from!(Int32Array, ArrayBufferView);
impl_from!(Int32Array, TypedArray);

/// An instance of Int8Array constructor (ES6 draft 15.13.6).
#[repr(C)]
pub struct Int8Array(Opaque);

impl Deref for Int8Array {
  type Target = TypedArray;
  fn deref(&self) -> &Self::Target {
    unsafe { &*(self as *const _ as *const Self::Target) }
  }
}

impl_from!(Int8Array, Data);
impl_from!(Int8Array, Value);
impl_from!(Int8Array, Object);
impl_from!(Int8Array, ArrayBufferView);
impl_from!(Int8Array, TypedArray);

/// An instance of Uint16Array constructor (ES6 draft 15.13.6).
#[repr(C)]
pub struct Uint16Array(Opaque);

impl Deref for Uint16Array {
  type Target = TypedArray;
  fn deref(&self) -> &Self::Target {
    unsafe { &*(self as *const _ as *const Self::Target) }
  }
}

impl_from!(Uint16Array, Data);
impl_from!(Uint16Array, Value);
impl_from!(Uint16Array, Object);
impl_from!(Uint16Array, ArrayBufferView);
impl_from!(Uint16Array, TypedArray);

/// An instance of Uint32Array constructor (ES6 draft 15.13.6).
#[repr(C)]
pub struct Uint32Array(Opaque);

impl Deref for Uint32Array {
  type Target = TypedArray;
  fn deref(&self) -> &Self::Target {
    unsafe { &*(self as *const _ as *const Self::Target) }
  }
}

impl_from!(Uint32Array, Data);
impl_from!(Uint32Array, Value);
impl_from!(Uint32Array, Object);
impl_from!(Uint32Array, ArrayBufferView);
impl_from!(Uint32Array, TypedArray);

/// An instance of Uint8Array constructor (ES6 draft 15.13.6).
#[repr(C)]
pub struct Uint8Array(Opaque);

impl Deref for Uint8Array {
  type Target = TypedArray;
  fn deref(&self) -> &Self::Target {
    unsafe { &*(self as *const _ as *const Self::Target) }
  }
}

impl_from!(Uint8Array, Data);
impl_from!(Uint8Array, Value);
impl_from!(Uint8Array, Object);
impl_from!(Uint8Array, ArrayBufferView);
impl_from!(Uint8Array, TypedArray);

/// An instance of Uint8ClampedArray constructor (ES6 draft 15.13.6).
#[repr(C)]
pub struct Uint8ClampedArray(Opaque);

impl Deref for Uint8ClampedArray {
  type Target = TypedArray;
  fn deref(&self) -> &Self::Target {
    unsafe { &*(self as *const _ as *const Self::Target) }
  }
}

impl_from!(Uint8ClampedArray, Data);
impl_from!(Uint8ClampedArray, Value);
impl_from!(Uint8ClampedArray, Object);
impl_from!(Uint8ClampedArray, ArrayBufferView);
impl_from!(Uint8ClampedArray, TypedArray);

/// A BigInt object (https://tc39.github.io/proposal-bigint)
#[repr(C)]
pub struct BigIntObject(Opaque);

impl Deref for BigIntObject {
  type Target = Object;
  fn deref(&self) -> &Self::Target {
    unsafe { &*(self as *const _ as *const Self::Target) }
  }
}

impl_from!(BigIntObject, Data);
impl_from!(BigIntObject, Value);
impl_from!(BigIntObject, Object);

/// A Boolean object (ECMA-262, 4.3.15).
#[repr(C)]
pub struct BooleanObject(Opaque);

impl Deref for BooleanObject {
  type Target = Object;
  fn deref(&self) -> &Self::Target {
    unsafe { &*(self as *const _ as *const Self::Target) }
  }
}

impl_from!(BooleanObject, Data);
impl_from!(BooleanObject, Value);
impl_from!(BooleanObject, Object);

/// An instance of the built-in Date constructor (ECMA-262, 15.9).
#[repr(C)]
pub struct Date(Opaque);

impl Deref for Date {
  type Target = Object;
  fn deref(&self) -> &Self::Target {
    unsafe { &*(self as *const _ as *const Self::Target) }
  }
}

impl_from!(Date, Data);
impl_from!(Date, Value);
impl_from!(Date, Object);

/// An instance of the built-in FinalizationGroup constructor.
///
/// This API is experimental and may change significantly.
#[repr(C)]
pub struct FinalizationGroup(Opaque);

impl Deref for FinalizationGroup {
  type Target = Object;
  fn deref(&self) -> &Self::Target {
    unsafe { &*(self as *const _ as *const Self::Target) }
  }
}

impl_from!(FinalizationGroup, Data);
impl_from!(FinalizationGroup, Value);
impl_from!(FinalizationGroup, Object);

/// A JavaScript function object (ECMA-262, 15.3).
#[repr(C)]
pub struct Function(Opaque);

impl Deref for Function {
  type Target = Object;
  fn deref(&self) -> &Self::Target {
    unsafe { &*(self as *const _ as *const Self::Target) }
  }
}

impl_from!(Function, Data);
impl_from!(Function, Value);
impl_from!(Function, Object);

/// An instance of the built-in Map constructor (ECMA-262, 6th Edition, 23.1.1).
#[repr(C)]
pub struct Map(Opaque);

impl Deref for Map {
  type Target = Object;
  fn deref(&self) -> &Self::Target {
    unsafe { &*(self as *const _ as *const Self::Target) }
  }
}

impl_from!(Map, Data);
impl_from!(Map, Value);
impl_from!(Map, Object);

/// A Number object (ECMA-262, 4.3.21).
#[repr(C)]
pub struct NumberObject(Opaque);

impl Deref for NumberObject {
  type Target = Object;
  fn deref(&self) -> &Self::Target {
    unsafe { &*(self as *const _ as *const Self::Target) }
  }
}

impl_from!(NumberObject, Data);
impl_from!(NumberObject, Value);
impl_from!(NumberObject, Object);

/// An instance of the built-in Promise constructor (ES6 draft).
#[repr(C)]
pub struct Promise(Opaque);

impl Deref for Promise {
  type Target = Object;
  fn deref(&self) -> &Self::Target {
    unsafe { &*(self as *const _ as *const Self::Target) }
  }
}

impl_from!(Promise, Data);
impl_from!(Promise, Value);
impl_from!(Promise, Object);

#[repr(C)]
pub struct PromiseResolver(Opaque);

impl Deref for PromiseResolver {
  type Target = Object;
  fn deref(&self) -> &Self::Target {
    unsafe { &*(self as *const _ as *const Self::Target) }
  }
}

impl_from!(PromiseResolver, Data);
impl_from!(PromiseResolver, Value);
impl_from!(PromiseResolver, Object);

/// An instance of the built-in Proxy constructor (ECMA-262, 6th Edition,
/// 26.2.1).
#[repr(C)]
pub struct Proxy(Opaque);

impl Deref for Proxy {
  type Target = Object;
  fn deref(&self) -> &Self::Target {
    unsafe { &*(self as *const _ as *const Self::Target) }
  }
}

impl_from!(Proxy, Data);
impl_from!(Proxy, Value);
impl_from!(Proxy, Object);

/// An instance of the built-in RegExp constructor (ECMA-262, 15.10).
#[repr(C)]
pub struct RegExp(Opaque);

impl Deref for RegExp {
  type Target = Object;
  fn deref(&self) -> &Self::Target {
    unsafe { &*(self as *const _ as *const Self::Target) }
  }
}

impl_from!(RegExp, Data);
impl_from!(RegExp, Value);
impl_from!(RegExp, Object);

/// An instance of the built-in Set constructor (ECMA-262, 6th Edition, 23.2.1).
#[repr(C)]
pub struct Set(Opaque);

impl Deref for Set {
  type Target = Object;
  fn deref(&self) -> &Self::Target {
    unsafe { &*(self as *const _ as *const Self::Target) }
  }
}

impl_from!(Set, Data);
impl_from!(Set, Value);
impl_from!(Set, Object);

/// An instance of the built-in SharedArrayBuffer constructor.
/// This API is experimental and may change significantly.
#[repr(C)]
pub struct SharedArrayBuffer(Opaque);

impl Deref for SharedArrayBuffer {
  type Target = Object;
  fn deref(&self) -> &Self::Target {
    unsafe { &*(self as *const _ as *const Self::Target) }
  }
}

impl_from!(SharedArrayBuffer, Data);
impl_from!(SharedArrayBuffer, Value);
impl_from!(SharedArrayBuffer, Object);

/// A String object (ECMA-262, 4.3.18).
#[repr(C)]
pub struct StringObject(Opaque);

impl Deref for StringObject {
  type Target = Object;
  fn deref(&self) -> &Self::Target {
    unsafe { &*(self as *const _ as *const Self::Target) }
  }
}

impl_from!(StringObject, Data);
impl_from!(StringObject, Value);
impl_from!(StringObject, Object);

/// A Symbol object (ECMA-262 edition 6).
#[repr(C)]
pub struct SymbolObject(Opaque);

impl Deref for SymbolObject {
  type Target = Object;
  fn deref(&self) -> &Self::Target {
    unsafe { &*(self as *const _ as *const Self::Target) }
  }
}

impl_from!(SymbolObject, Data);
impl_from!(SymbolObject, Value);
impl_from!(SymbolObject, Object);

#[repr(C)]
pub struct WasmModuleObject(Opaque);

impl Deref for WasmModuleObject {
  type Target = Object;
  fn deref(&self) -> &Self::Target {
    unsafe { &*(self as *const _ as *const Self::Target) }
  }
}

impl_from!(WasmModuleObject, Data);
impl_from!(WasmModuleObject, Value);
impl_from!(WasmModuleObject, Object);

/// The superclass of primitive values. See ECMA-262 4.3.2.
#[repr(C)]
pub struct Primitive(Opaque);

impl Deref for Primitive {
  type Target = Value;
  fn deref(&self) -> &Self::Target {
    unsafe { &*(self as *const _ as *const Self::Target) }
  }
}

impl_from!(Primitive, Data);
impl_from!(Primitive, Value);

/// A JavaScript BigInt value (https://tc39.github.io/proposal-bigint)
#[repr(C)]
pub struct BigInt(Opaque);

impl Deref for BigInt {
  type Target = Primitive;
  fn deref(&self) -> &Self::Target {
    unsafe { &*(self as *const _ as *const Self::Target) }
  }
}

impl_from!(BigInt, Data);
impl_from!(BigInt, Value);
impl_from!(BigInt, Primitive);

/// A primitive boolean value (ECMA-262, 4.3.14). Either the true
/// or false value.
#[repr(C)]
pub struct Boolean(Opaque);

impl Deref for Boolean {
  type Target = Primitive;
  fn deref(&self) -> &Self::Target {
    unsafe { &*(self as *const _ as *const Self::Target) }
  }
}

impl_from!(Boolean, Data);
impl_from!(Boolean, Value);
impl_from!(Boolean, Primitive);

/// A superclass for symbols and strings.
#[repr(C)]
pub struct Name(Opaque);

impl Deref for Name {
  type Target = Primitive;
  fn deref(&self) -> &Self::Target {
    unsafe { &*(self as *const _ as *const Self::Target) }
  }
}

impl_from!(Name, Data);
impl_from!(Name, Value);
impl_from!(Name, Primitive);

/// A JavaScript string value (ECMA-262, 4.3.17).
#[repr(C)]
pub struct String(Opaque);

impl Deref for String {
  type Target = Name;
  fn deref(&self) -> &Self::Target {
    unsafe { &*(self as *const _ as *const Self::Target) }
  }
}

impl_from!(String, Data);
impl_from!(String, Value);
impl_from!(String, Primitive);
impl_from!(String, Name);

/// A JavaScript symbol (ECMA-262 edition 6)
#[repr(C)]
pub struct Symbol(Opaque);

impl Deref for Symbol {
  type Target = Name;
  fn deref(&self) -> &Self::Target {
    unsafe { &*(self as *const _ as *const Self::Target) }
  }
}

impl_from!(Symbol, Data);
impl_from!(Symbol, Value);
impl_from!(Symbol, Primitive);
impl_from!(Symbol, Name);

/// A JavaScript number value (ECMA-262, 4.3.20)
#[repr(C)]
pub struct Number(Opaque);

impl Deref for Number {
  type Target = Primitive;
  fn deref(&self) -> &Self::Target {
    unsafe { &*(self as *const _ as *const Self::Target) }
  }
}

impl_from!(Number, Data);
impl_from!(Number, Value);
impl_from!(Number, Primitive);

/// A JavaScript value representing a signed integer.
#[repr(C)]
pub struct Integer(Opaque);

impl Deref for Integer {
  type Target = Number;
  fn deref(&self) -> &Self::Target {
    unsafe { &*(self as *const _ as *const Self::Target) }
  }
}

impl_from!(Integer, Data);
impl_from!(Integer, Value);
impl_from!(Integer, Primitive);
impl_from!(Integer, Number);

/// A JavaScript value representing a 32-bit signed integer.
#[repr(C)]
pub struct Int32(Opaque);

impl Deref for Int32 {
  type Target = Integer;
  fn deref(&self) -> &Self::Target {
    unsafe { &*(self as *const _ as *const Self::Target) }
  }
}

impl_from!(Int32, Data);
impl_from!(Int32, Value);
impl_from!(Int32, Primitive);
impl_from!(Int32, Number);
impl_from!(Int32, Integer);

/// A JavaScript value representing a 32-bit unsigned integer.
#[repr(C)]
pub struct Uint32(Opaque);

impl Deref for Uint32 {
  type Target = Integer;
  fn deref(&self) -> &Self::Target {
    unsafe { &*(self as *const _ as *const Self::Target) }
  }
}

impl_from!(Uint32, Data);
impl_from!(Uint32, Value);
impl_from!(Uint32, Primitive);
impl_from!(Uint32, Number);
impl_from!(Uint32, Integer);
