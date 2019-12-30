use std::convert::From;
use std::mem::transmute;

use crate::support::Opaque;
use crate::Local;

/// The superclass of objects that can reside on V8's heap.
#[repr(C)]
pub struct Data(Opaque);

/// An AccessorSignature specifies which receivers are valid parameters
/// to an accessor callback.
#[repr(C)]
pub struct AccessorSignature(Opaque);
impl<'sc> From<Local<'sc, AccessorSignature>> for Local<'sc, Data> {
  fn from(l: Local<'sc, AccessorSignature>) -> Self {
    unsafe { transmute(l) }
  }
}

/// A compiled JavaScript module.
#[repr(C)]
pub struct Module(Opaque);
impl<'sc> From<Local<'sc, Module>> for Local<'sc, Data> {
  fn from(l: Local<'sc, Module>) -> Self {
    unsafe { transmute(l) }
  }
}

/// A private symbol
///
/// This is an experimental feature. Use at your own risk.
#[repr(C)]
pub struct Private(Opaque);
impl<'sc> From<Local<'sc, Private>> for Local<'sc, Data> {
  fn from(l: Local<'sc, Private>) -> Self {
    unsafe { transmute(l) }
  }
}

/// A Signature specifies which receiver is valid for a function.
///
/// A receiver matches a given signature if the receiver (or any of its
/// hidden prototypes) was created from the signature's FunctionTemplate, or
/// from a FunctionTemplate that inherits directly or indirectly from the
/// signature's FunctionTemplate.
#[repr(C)]
pub struct Signature(Opaque);
impl<'sc> From<Local<'sc, Signature>> for Local<'sc, Data> {
  fn from(l: Local<'sc, Signature>) -> Self {
    unsafe { transmute(l) }
  }
}

/// The superclass of object and function templates.
#[repr(C)]
pub struct Template(Opaque);
impl<'sc> From<Local<'sc, Template>> for Local<'sc, Data> {
  fn from(l: Local<'sc, Template>) -> Self {
    unsafe { transmute(l) }
  }
}

/// A FunctionTemplate is used to create functions at runtime. There
/// can only be one function created from a FunctionTemplate in a
/// context.  The lifetime of the created function is equal to the
/// lifetime of the context.  So in case the embedder needs to create
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
/// and "instance" for the instance object created above.  The function
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
/// FunctionTemplate::Inherit method.  The following graph illustrates
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
impl<'sc> From<Local<'sc, FunctionTemplate>> for Local<'sc, Data> {
  fn from(l: Local<'sc, FunctionTemplate>) -> Self {
    unsafe { transmute(l) }
  }
}
impl<'sc> From<Local<'sc, FunctionTemplate>> for Local<'sc, Template> {
  fn from(l: Local<'sc, FunctionTemplate>) -> Self {
    unsafe { transmute(l) }
  }
}

/// An ObjectTemplate is used to create objects at runtime.
///
/// Properties added to an ObjectTemplate are added to each object
/// created from the ObjectTemplate.
#[repr(C)]
pub struct ObjectTemplate(Opaque);
impl<'sc> From<Local<'sc, ObjectTemplate>> for Local<'sc, Data> {
  fn from(l: Local<'sc, ObjectTemplate>) -> Self {
    unsafe { transmute(l) }
  }
}
impl<'sc> From<Local<'sc, ObjectTemplate>> for Local<'sc, Template> {
  fn from(l: Local<'sc, ObjectTemplate>) -> Self {
    unsafe { transmute(l) }
  }
}

/// A compiled JavaScript module, not yet tied to a Context.
#[repr(C)]
pub struct UnboundModuleScript(Opaque);
impl<'sc> From<Local<'sc, UnboundModuleScript>> for Local<'sc, Data> {
  fn from(l: Local<'sc, UnboundModuleScript>) -> Self {
    unsafe { transmute(l) }
  }
}

/// The superclass of all JavaScript values and objects.
#[repr(C)]
pub struct Value(Opaque);
impl<'sc> From<Local<'sc, Value>> for Local<'sc, Data> {
  fn from(l: Local<'sc, Value>) -> Self {
    unsafe { transmute(l) }
  }
}

/// A JavaScript value that wraps a C++ void*. This type of value is mainly used
/// to associate C++ data structures with JavaScript objects.
#[repr(C)]
pub struct External(Opaque);
impl<'sc> From<Local<'sc, External>> for Local<'sc, Data> {
  fn from(l: Local<'sc, External>) -> Self {
    unsafe { transmute(l) }
  }
}
impl<'sc> From<Local<'sc, External>> for Local<'sc, Value> {
  fn from(l: Local<'sc, External>) -> Self {
    unsafe { transmute(l) }
  }
}

/// A JavaScript object (ECMA-262, 4.3.3)
#[repr(C)]
pub struct Object(Opaque);
impl<'sc> From<Local<'sc, Object>> for Local<'sc, Data> {
  fn from(l: Local<'sc, Object>) -> Self {
    unsafe { transmute(l) }
  }
}
impl<'sc> From<Local<'sc, Object>> for Local<'sc, Value> {
  fn from(l: Local<'sc, Object>) -> Self {
    unsafe { transmute(l) }
  }
}

/// An instance of the built-in array constructor (ECMA-262, 15.4.2).
#[repr(C)]
pub struct Array(Opaque);
impl<'sc> From<Local<'sc, Array>> for Local<'sc, Data> {
  fn from(l: Local<'sc, Array>) -> Self {
    unsafe { transmute(l) }
  }
}
impl<'sc> From<Local<'sc, Array>> for Local<'sc, Value> {
  fn from(l: Local<'sc, Array>) -> Self {
    unsafe { transmute(l) }
  }
}
impl<'sc> From<Local<'sc, Array>> for Local<'sc, Object> {
  fn from(l: Local<'sc, Array>) -> Self {
    unsafe { transmute(l) }
  }
}

/// An instance of the built-in ArrayBuffer constructor (ES6 draft 15.13.5).
#[repr(C)]
pub struct ArrayBuffer(Opaque);
impl<'sc> From<Local<'sc, ArrayBuffer>> for Local<'sc, Data> {
  fn from(l: Local<'sc, ArrayBuffer>) -> Self {
    unsafe { transmute(l) }
  }
}
impl<'sc> From<Local<'sc, ArrayBuffer>> for Local<'sc, Value> {
  fn from(l: Local<'sc, ArrayBuffer>) -> Self {
    unsafe { transmute(l) }
  }
}
impl<'sc> From<Local<'sc, ArrayBuffer>> for Local<'sc, Object> {
  fn from(l: Local<'sc, ArrayBuffer>) -> Self {
    unsafe { transmute(l) }
  }
}

/// A base class for an instance of one of "views" over ArrayBuffer,
/// including TypedArrays and DataView (ES6 draft 15.13).
#[repr(C)]
pub struct ArrayBufferView(Opaque);
impl<'sc> From<Local<'sc, ArrayBufferView>> for Local<'sc, Data> {
  fn from(l: Local<'sc, ArrayBufferView>) -> Self {
    unsafe { transmute(l) }
  }
}
impl<'sc> From<Local<'sc, ArrayBufferView>> for Local<'sc, Value> {
  fn from(l: Local<'sc, ArrayBufferView>) -> Self {
    unsafe { transmute(l) }
  }
}
impl<'sc> From<Local<'sc, ArrayBufferView>> for Local<'sc, Object> {
  fn from(l: Local<'sc, ArrayBufferView>) -> Self {
    unsafe { transmute(l) }
  }
}

/// An instance of DataView constructor (ES6 draft 15.13.7).
#[repr(C)]
pub struct DataView(Opaque);
impl<'sc> From<Local<'sc, DataView>> for Local<'sc, Data> {
  fn from(l: Local<'sc, DataView>) -> Self {
    unsafe { transmute(l) }
  }
}
impl<'sc> From<Local<'sc, DataView>> for Local<'sc, Value> {
  fn from(l: Local<'sc, DataView>) -> Self {
    unsafe { transmute(l) }
  }
}
impl<'sc> From<Local<'sc, DataView>> for Local<'sc, Object> {
  fn from(l: Local<'sc, DataView>) -> Self {
    unsafe { transmute(l) }
  }
}
impl<'sc> From<Local<'sc, DataView>> for Local<'sc, ArrayBufferView> {
  fn from(l: Local<'sc, DataView>) -> Self {
    unsafe { transmute(l) }
  }
}

/// A base class for an instance of TypedArray series of constructors
/// (ES6 draft 15.13.6).
#[repr(C)]
pub struct TypedArray(Opaque);
impl<'sc> From<Local<'sc, TypedArray>> for Local<'sc, Data> {
  fn from(l: Local<'sc, TypedArray>) -> Self {
    unsafe { transmute(l) }
  }
}
impl<'sc> From<Local<'sc, TypedArray>> for Local<'sc, Value> {
  fn from(l: Local<'sc, TypedArray>) -> Self {
    unsafe { transmute(l) }
  }
}
impl<'sc> From<Local<'sc, TypedArray>> for Local<'sc, Object> {
  fn from(l: Local<'sc, TypedArray>) -> Self {
    unsafe { transmute(l) }
  }
}
impl<'sc> From<Local<'sc, TypedArray>> for Local<'sc, ArrayBufferView> {
  fn from(l: Local<'sc, TypedArray>) -> Self {
    unsafe { transmute(l) }
  }
}

/// An instance of BigInt64Array constructor.
#[repr(C)]
pub struct BigInt64Array(Opaque);
impl<'sc> From<Local<'sc, BigInt64Array>> for Local<'sc, Data> {
  fn from(l: Local<'sc, BigInt64Array>) -> Self {
    unsafe { transmute(l) }
  }
}
impl<'sc> From<Local<'sc, BigInt64Array>> for Local<'sc, Value> {
  fn from(l: Local<'sc, BigInt64Array>) -> Self {
    unsafe { transmute(l) }
  }
}
impl<'sc> From<Local<'sc, BigInt64Array>> for Local<'sc, Object> {
  fn from(l: Local<'sc, BigInt64Array>) -> Self {
    unsafe { transmute(l) }
  }
}
impl<'sc> From<Local<'sc, BigInt64Array>> for Local<'sc, ArrayBufferView> {
  fn from(l: Local<'sc, BigInt64Array>) -> Self {
    unsafe { transmute(l) }
  }
}
impl<'sc> From<Local<'sc, BigInt64Array>> for Local<'sc, TypedArray> {
  fn from(l: Local<'sc, BigInt64Array>) -> Self {
    unsafe { transmute(l) }
  }
}

/// An instance of BigUint64Array constructor.
#[repr(C)]
pub struct BigUint64Array(Opaque);
impl<'sc> From<Local<'sc, BigUint64Array>> for Local<'sc, Data> {
  fn from(l: Local<'sc, BigUint64Array>) -> Self {
    unsafe { transmute(l) }
  }
}
impl<'sc> From<Local<'sc, BigUint64Array>> for Local<'sc, Value> {
  fn from(l: Local<'sc, BigUint64Array>) -> Self {
    unsafe { transmute(l) }
  }
}
impl<'sc> From<Local<'sc, BigUint64Array>> for Local<'sc, Object> {
  fn from(l: Local<'sc, BigUint64Array>) -> Self {
    unsafe { transmute(l) }
  }
}
impl<'sc> From<Local<'sc, BigUint64Array>> for Local<'sc, ArrayBufferView> {
  fn from(l: Local<'sc, BigUint64Array>) -> Self {
    unsafe { transmute(l) }
  }
}
impl<'sc> From<Local<'sc, BigUint64Array>> for Local<'sc, TypedArray> {
  fn from(l: Local<'sc, BigUint64Array>) -> Self {
    unsafe { transmute(l) }
  }
}

/// An instance of Float32Array constructor (ES6 draft 15.13.6).
#[repr(C)]
pub struct Float32Array(Opaque);
impl<'sc> From<Local<'sc, Float32Array>> for Local<'sc, Data> {
  fn from(l: Local<'sc, Float32Array>) -> Self {
    unsafe { transmute(l) }
  }
}
impl<'sc> From<Local<'sc, Float32Array>> for Local<'sc, Value> {
  fn from(l: Local<'sc, Float32Array>) -> Self {
    unsafe { transmute(l) }
  }
}
impl<'sc> From<Local<'sc, Float32Array>> for Local<'sc, Object> {
  fn from(l: Local<'sc, Float32Array>) -> Self {
    unsafe { transmute(l) }
  }
}
impl<'sc> From<Local<'sc, Float32Array>> for Local<'sc, ArrayBufferView> {
  fn from(l: Local<'sc, Float32Array>) -> Self {
    unsafe { transmute(l) }
  }
}
impl<'sc> From<Local<'sc, Float32Array>> for Local<'sc, TypedArray> {
  fn from(l: Local<'sc, Float32Array>) -> Self {
    unsafe { transmute(l) }
  }
}

/// An instance of Float64Array constructor (ES6 draft 15.13.6).
#[repr(C)]
pub struct Float64Array(Opaque);
impl<'sc> From<Local<'sc, Float64Array>> for Local<'sc, Data> {
  fn from(l: Local<'sc, Float64Array>) -> Self {
    unsafe { transmute(l) }
  }
}
impl<'sc> From<Local<'sc, Float64Array>> for Local<'sc, Value> {
  fn from(l: Local<'sc, Float64Array>) -> Self {
    unsafe { transmute(l) }
  }
}
impl<'sc> From<Local<'sc, Float64Array>> for Local<'sc, Object> {
  fn from(l: Local<'sc, Float64Array>) -> Self {
    unsafe { transmute(l) }
  }
}
impl<'sc> From<Local<'sc, Float64Array>> for Local<'sc, ArrayBufferView> {
  fn from(l: Local<'sc, Float64Array>) -> Self {
    unsafe { transmute(l) }
  }
}
impl<'sc> From<Local<'sc, Float64Array>> for Local<'sc, TypedArray> {
  fn from(l: Local<'sc, Float64Array>) -> Self {
    unsafe { transmute(l) }
  }
}

/// An instance of Int16Array constructor (ES6 draft 15.13.6).
#[repr(C)]
pub struct Int16Array(Opaque);
impl<'sc> From<Local<'sc, Int16Array>> for Local<'sc, Data> {
  fn from(l: Local<'sc, Int16Array>) -> Self {
    unsafe { transmute(l) }
  }
}
impl<'sc> From<Local<'sc, Int16Array>> for Local<'sc, Value> {
  fn from(l: Local<'sc, Int16Array>) -> Self {
    unsafe { transmute(l) }
  }
}
impl<'sc> From<Local<'sc, Int16Array>> for Local<'sc, Object> {
  fn from(l: Local<'sc, Int16Array>) -> Self {
    unsafe { transmute(l) }
  }
}
impl<'sc> From<Local<'sc, Int16Array>> for Local<'sc, ArrayBufferView> {
  fn from(l: Local<'sc, Int16Array>) -> Self {
    unsafe { transmute(l) }
  }
}
impl<'sc> From<Local<'sc, Int16Array>> for Local<'sc, TypedArray> {
  fn from(l: Local<'sc, Int16Array>) -> Self {
    unsafe { transmute(l) }
  }
}

/// An instance of Int32Array constructor (ES6 draft 15.13.6).
#[repr(C)]
pub struct Int32Array(Opaque);
impl<'sc> From<Local<'sc, Int32Array>> for Local<'sc, Data> {
  fn from(l: Local<'sc, Int32Array>) -> Self {
    unsafe { transmute(l) }
  }
}
impl<'sc> From<Local<'sc, Int32Array>> for Local<'sc, Value> {
  fn from(l: Local<'sc, Int32Array>) -> Self {
    unsafe { transmute(l) }
  }
}
impl<'sc> From<Local<'sc, Int32Array>> for Local<'sc, Object> {
  fn from(l: Local<'sc, Int32Array>) -> Self {
    unsafe { transmute(l) }
  }
}
impl<'sc> From<Local<'sc, Int32Array>> for Local<'sc, ArrayBufferView> {
  fn from(l: Local<'sc, Int32Array>) -> Self {
    unsafe { transmute(l) }
  }
}
impl<'sc> From<Local<'sc, Int32Array>> for Local<'sc, TypedArray> {
  fn from(l: Local<'sc, Int32Array>) -> Self {
    unsafe { transmute(l) }
  }
}

/// An instance of Int8Array constructor (ES6 draft 15.13.6).
#[repr(C)]
pub struct Int8Array(Opaque);
impl<'sc> From<Local<'sc, Int8Array>> for Local<'sc, Data> {
  fn from(l: Local<'sc, Int8Array>) -> Self {
    unsafe { transmute(l) }
  }
}
impl<'sc> From<Local<'sc, Int8Array>> for Local<'sc, Value> {
  fn from(l: Local<'sc, Int8Array>) -> Self {
    unsafe { transmute(l) }
  }
}
impl<'sc> From<Local<'sc, Int8Array>> for Local<'sc, Object> {
  fn from(l: Local<'sc, Int8Array>) -> Self {
    unsafe { transmute(l) }
  }
}
impl<'sc> From<Local<'sc, Int8Array>> for Local<'sc, ArrayBufferView> {
  fn from(l: Local<'sc, Int8Array>) -> Self {
    unsafe { transmute(l) }
  }
}
impl<'sc> From<Local<'sc, Int8Array>> for Local<'sc, TypedArray> {
  fn from(l: Local<'sc, Int8Array>) -> Self {
    unsafe { transmute(l) }
  }
}

/// An instance of Uint16Array constructor (ES6 draft 15.13.6).
#[repr(C)]
pub struct Uint16Array(Opaque);
impl<'sc> From<Local<'sc, Uint16Array>> for Local<'sc, Data> {
  fn from(l: Local<'sc, Uint16Array>) -> Self {
    unsafe { transmute(l) }
  }
}
impl<'sc> From<Local<'sc, Uint16Array>> for Local<'sc, Value> {
  fn from(l: Local<'sc, Uint16Array>) -> Self {
    unsafe { transmute(l) }
  }
}
impl<'sc> From<Local<'sc, Uint16Array>> for Local<'sc, Object> {
  fn from(l: Local<'sc, Uint16Array>) -> Self {
    unsafe { transmute(l) }
  }
}
impl<'sc> From<Local<'sc, Uint16Array>> for Local<'sc, ArrayBufferView> {
  fn from(l: Local<'sc, Uint16Array>) -> Self {
    unsafe { transmute(l) }
  }
}
impl<'sc> From<Local<'sc, Uint16Array>> for Local<'sc, TypedArray> {
  fn from(l: Local<'sc, Uint16Array>) -> Self {
    unsafe { transmute(l) }
  }
}

/// An instance of Uint32Array constructor (ES6 draft 15.13.6).
#[repr(C)]
pub struct Uint32Array(Opaque);
impl<'sc> From<Local<'sc, Uint32Array>> for Local<'sc, Data> {
  fn from(l: Local<'sc, Uint32Array>) -> Self {
    unsafe { transmute(l) }
  }
}
impl<'sc> From<Local<'sc, Uint32Array>> for Local<'sc, Value> {
  fn from(l: Local<'sc, Uint32Array>) -> Self {
    unsafe { transmute(l) }
  }
}
impl<'sc> From<Local<'sc, Uint32Array>> for Local<'sc, Object> {
  fn from(l: Local<'sc, Uint32Array>) -> Self {
    unsafe { transmute(l) }
  }
}
impl<'sc> From<Local<'sc, Uint32Array>> for Local<'sc, ArrayBufferView> {
  fn from(l: Local<'sc, Uint32Array>) -> Self {
    unsafe { transmute(l) }
  }
}
impl<'sc> From<Local<'sc, Uint32Array>> for Local<'sc, TypedArray> {
  fn from(l: Local<'sc, Uint32Array>) -> Self {
    unsafe { transmute(l) }
  }
}

/// An instance of Uint8Array constructor (ES6 draft 15.13.6).
#[repr(C)]
pub struct Uint8Array(Opaque);
impl<'sc> From<Local<'sc, Uint8Array>> for Local<'sc, Data> {
  fn from(l: Local<'sc, Uint8Array>) -> Self {
    unsafe { transmute(l) }
  }
}
impl<'sc> From<Local<'sc, Uint8Array>> for Local<'sc, Value> {
  fn from(l: Local<'sc, Uint8Array>) -> Self {
    unsafe { transmute(l) }
  }
}
impl<'sc> From<Local<'sc, Uint8Array>> for Local<'sc, Object> {
  fn from(l: Local<'sc, Uint8Array>) -> Self {
    unsafe { transmute(l) }
  }
}
impl<'sc> From<Local<'sc, Uint8Array>> for Local<'sc, ArrayBufferView> {
  fn from(l: Local<'sc, Uint8Array>) -> Self {
    unsafe { transmute(l) }
  }
}
impl<'sc> From<Local<'sc, Uint8Array>> for Local<'sc, TypedArray> {
  fn from(l: Local<'sc, Uint8Array>) -> Self {
    unsafe { transmute(l) }
  }
}

/// An instance of Uint8ClampedArray constructor (ES6 draft 15.13.6).
#[repr(C)]
pub struct Uint8ClampedArray(Opaque);
impl<'sc> From<Local<'sc, Uint8ClampedArray>> for Local<'sc, Data> {
  fn from(l: Local<'sc, Uint8ClampedArray>) -> Self {
    unsafe { transmute(l) }
  }
}
impl<'sc> From<Local<'sc, Uint8ClampedArray>> for Local<'sc, Value> {
  fn from(l: Local<'sc, Uint8ClampedArray>) -> Self {
    unsafe { transmute(l) }
  }
}
impl<'sc> From<Local<'sc, Uint8ClampedArray>> for Local<'sc, Object> {
  fn from(l: Local<'sc, Uint8ClampedArray>) -> Self {
    unsafe { transmute(l) }
  }
}
impl<'sc> From<Local<'sc, Uint8ClampedArray>> for Local<'sc, ArrayBufferView> {
  fn from(l: Local<'sc, Uint8ClampedArray>) -> Self {
    unsafe { transmute(l) }
  }
}
impl<'sc> From<Local<'sc, Uint8ClampedArray>> for Local<'sc, TypedArray> {
  fn from(l: Local<'sc, Uint8ClampedArray>) -> Self {
    unsafe { transmute(l) }
  }
}

/// A BigInt object (https://tc39.github.io/proposal-bigint)
#[repr(C)]
pub struct BigIntObject(Opaque);
impl<'sc> From<Local<'sc, BigIntObject>> for Local<'sc, Data> {
  fn from(l: Local<'sc, BigIntObject>) -> Self {
    unsafe { transmute(l) }
  }
}
impl<'sc> From<Local<'sc, BigIntObject>> for Local<'sc, Value> {
  fn from(l: Local<'sc, BigIntObject>) -> Self {
    unsafe { transmute(l) }
  }
}
impl<'sc> From<Local<'sc, BigIntObject>> for Local<'sc, Object> {
  fn from(l: Local<'sc, BigIntObject>) -> Self {
    unsafe { transmute(l) }
  }
}

/// A Boolean object (ECMA-262, 4.3.15).
#[repr(C)]
pub struct BooleanObject(Opaque);
impl<'sc> From<Local<'sc, BooleanObject>> for Local<'sc, Data> {
  fn from(l: Local<'sc, BooleanObject>) -> Self {
    unsafe { transmute(l) }
  }
}
impl<'sc> From<Local<'sc, BooleanObject>> for Local<'sc, Value> {
  fn from(l: Local<'sc, BooleanObject>) -> Self {
    unsafe { transmute(l) }
  }
}
impl<'sc> From<Local<'sc, BooleanObject>> for Local<'sc, Object> {
  fn from(l: Local<'sc, BooleanObject>) -> Self {
    unsafe { transmute(l) }
  }
}

/// An instance of the built-in Date constructor (ECMA-262, 15.9).
#[repr(C)]
pub struct Date(Opaque);
impl<'sc> From<Local<'sc, Date>> for Local<'sc, Data> {
  fn from(l: Local<'sc, Date>) -> Self {
    unsafe { transmute(l) }
  }
}
impl<'sc> From<Local<'sc, Date>> for Local<'sc, Value> {
  fn from(l: Local<'sc, Date>) -> Self {
    unsafe { transmute(l) }
  }
}
impl<'sc> From<Local<'sc, Date>> for Local<'sc, Object> {
  fn from(l: Local<'sc, Date>) -> Self {
    unsafe { transmute(l) }
  }
}

/// An instance of the built-in FinalizationGroup constructor.
///
/// This API is experimental and may change significantly.
#[repr(C)]
pub struct FinalizationGroup(Opaque);
impl<'sc> From<Local<'sc, FinalizationGroup>> for Local<'sc, Data> {
  fn from(l: Local<'sc, FinalizationGroup>) -> Self {
    unsafe { transmute(l) }
  }
}
impl<'sc> From<Local<'sc, FinalizationGroup>> for Local<'sc, Value> {
  fn from(l: Local<'sc, FinalizationGroup>) -> Self {
    unsafe { transmute(l) }
  }
}
impl<'sc> From<Local<'sc, FinalizationGroup>> for Local<'sc, Object> {
  fn from(l: Local<'sc, FinalizationGroup>) -> Self {
    unsafe { transmute(l) }
  }
}

/// A JavaScript function object (ECMA-262, 15.3).
#[repr(C)]
pub struct Function(Opaque);
impl<'sc> From<Local<'sc, Function>> for Local<'sc, Data> {
  fn from(l: Local<'sc, Function>) -> Self {
    unsafe { transmute(l) }
  }
}
impl<'sc> From<Local<'sc, Function>> for Local<'sc, Value> {
  fn from(l: Local<'sc, Function>) -> Self {
    unsafe { transmute(l) }
  }
}
impl<'sc> From<Local<'sc, Function>> for Local<'sc, Object> {
  fn from(l: Local<'sc, Function>) -> Self {
    unsafe { transmute(l) }
  }
}

/// An instance of the built-in Map constructor (ECMA-262, 6th Edition, 23.1.1).
#[repr(C)]
pub struct Map(Opaque);
impl<'sc> From<Local<'sc, Map>> for Local<'sc, Data> {
  fn from(l: Local<'sc, Map>) -> Self {
    unsafe { transmute(l) }
  }
}
impl<'sc> From<Local<'sc, Map>> for Local<'sc, Value> {
  fn from(l: Local<'sc, Map>) -> Self {
    unsafe { transmute(l) }
  }
}
impl<'sc> From<Local<'sc, Map>> for Local<'sc, Object> {
  fn from(l: Local<'sc, Map>) -> Self {
    unsafe { transmute(l) }
  }
}

/// A Number object (ECMA-262, 4.3.21).
#[repr(C)]
pub struct NumberObject(Opaque);
impl<'sc> From<Local<'sc, NumberObject>> for Local<'sc, Data> {
  fn from(l: Local<'sc, NumberObject>) -> Self {
    unsafe { transmute(l) }
  }
}
impl<'sc> From<Local<'sc, NumberObject>> for Local<'sc, Value> {
  fn from(l: Local<'sc, NumberObject>) -> Self {
    unsafe { transmute(l) }
  }
}
impl<'sc> From<Local<'sc, NumberObject>> for Local<'sc, Object> {
  fn from(l: Local<'sc, NumberObject>) -> Self {
    unsafe { transmute(l) }
  }
}

/// An instance of the built-in Promise constructor (ES6 draft).
#[repr(C)]
pub struct Promise(Opaque);
impl<'sc> From<Local<'sc, Promise>> for Local<'sc, Data> {
  fn from(l: Local<'sc, Promise>) -> Self {
    unsafe { transmute(l) }
  }
}
impl<'sc> From<Local<'sc, Promise>> for Local<'sc, Value> {
  fn from(l: Local<'sc, Promise>) -> Self {
    unsafe { transmute(l) }
  }
}
impl<'sc> From<Local<'sc, Promise>> for Local<'sc, Object> {
  fn from(l: Local<'sc, Promise>) -> Self {
    unsafe { transmute(l) }
  }
}

#[repr(C)]
pub struct PromiseResolver(Opaque);
impl<'sc> From<Local<'sc, PromiseResolver>> for Local<'sc, Data> {
  fn from(l: Local<'sc, PromiseResolver>) -> Self {
    unsafe { transmute(l) }
  }
}
impl<'sc> From<Local<'sc, PromiseResolver>> for Local<'sc, Value> {
  fn from(l: Local<'sc, PromiseResolver>) -> Self {
    unsafe { transmute(l) }
  }
}
impl<'sc> From<Local<'sc, PromiseResolver>> for Local<'sc, Object> {
  fn from(l: Local<'sc, PromiseResolver>) -> Self {
    unsafe { transmute(l) }
  }
}

/// An instance of the built-in Proxy constructor (ECMA-262, 6th Edition,
/// 26.2.1).
#[repr(C)]
pub struct Proxy(Opaque);
impl<'sc> From<Local<'sc, Proxy>> for Local<'sc, Data> {
  fn from(l: Local<'sc, Proxy>) -> Self {
    unsafe { transmute(l) }
  }
}
impl<'sc> From<Local<'sc, Proxy>> for Local<'sc, Value> {
  fn from(l: Local<'sc, Proxy>) -> Self {
    unsafe { transmute(l) }
  }
}
impl<'sc> From<Local<'sc, Proxy>> for Local<'sc, Object> {
  fn from(l: Local<'sc, Proxy>) -> Self {
    unsafe { transmute(l) }
  }
}

/// An instance of the built-in RegExp constructor (ECMA-262, 15.10).
#[repr(C)]
pub struct RegExp(Opaque);
impl<'sc> From<Local<'sc, RegExp>> for Local<'sc, Data> {
  fn from(l: Local<'sc, RegExp>) -> Self {
    unsafe { transmute(l) }
  }
}
impl<'sc> From<Local<'sc, RegExp>> for Local<'sc, Value> {
  fn from(l: Local<'sc, RegExp>) -> Self {
    unsafe { transmute(l) }
  }
}
impl<'sc> From<Local<'sc, RegExp>> for Local<'sc, Object> {
  fn from(l: Local<'sc, RegExp>) -> Self {
    unsafe { transmute(l) }
  }
}

/// An instance of the built-in Set constructor (ECMA-262, 6th Edition, 23.2.1).
#[repr(C)]
pub struct Set(Opaque);
impl<'sc> From<Local<'sc, Set>> for Local<'sc, Data> {
  fn from(l: Local<'sc, Set>) -> Self {
    unsafe { transmute(l) }
  }
}
impl<'sc> From<Local<'sc, Set>> for Local<'sc, Value> {
  fn from(l: Local<'sc, Set>) -> Self {
    unsafe { transmute(l) }
  }
}
impl<'sc> From<Local<'sc, Set>> for Local<'sc, Object> {
  fn from(l: Local<'sc, Set>) -> Self {
    unsafe { transmute(l) }
  }
}

/// An instance of the built-in SharedArrayBuffer constructor.
/// This API is experimental and may change significantly.
#[repr(C)]
pub struct SharedArrayBuffer(Opaque);
impl<'sc> From<Local<'sc, SharedArrayBuffer>> for Local<'sc, Data> {
  fn from(l: Local<'sc, SharedArrayBuffer>) -> Self {
    unsafe { transmute(l) }
  }
}
impl<'sc> From<Local<'sc, SharedArrayBuffer>> for Local<'sc, Value> {
  fn from(l: Local<'sc, SharedArrayBuffer>) -> Self {
    unsafe { transmute(l) }
  }
}
impl<'sc> From<Local<'sc, SharedArrayBuffer>> for Local<'sc, Object> {
  fn from(l: Local<'sc, SharedArrayBuffer>) -> Self {
    unsafe { transmute(l) }
  }
}

/// A String object (ECMA-262, 4.3.18).
#[repr(C)]
pub struct StringObject(Opaque);
impl<'sc> From<Local<'sc, StringObject>> for Local<'sc, Data> {
  fn from(l: Local<'sc, StringObject>) -> Self {
    unsafe { transmute(l) }
  }
}
impl<'sc> From<Local<'sc, StringObject>> for Local<'sc, Value> {
  fn from(l: Local<'sc, StringObject>) -> Self {
    unsafe { transmute(l) }
  }
}
impl<'sc> From<Local<'sc, StringObject>> for Local<'sc, Object> {
  fn from(l: Local<'sc, StringObject>) -> Self {
    unsafe { transmute(l) }
  }
}

/// A Symbol object (ECMA-262 edition 6).
#[repr(C)]
pub struct SymbolObject(Opaque);
impl<'sc> From<Local<'sc, SymbolObject>> for Local<'sc, Data> {
  fn from(l: Local<'sc, SymbolObject>) -> Self {
    unsafe { transmute(l) }
  }
}
impl<'sc> From<Local<'sc, SymbolObject>> for Local<'sc, Value> {
  fn from(l: Local<'sc, SymbolObject>) -> Self {
    unsafe { transmute(l) }
  }
}
impl<'sc> From<Local<'sc, SymbolObject>> for Local<'sc, Object> {
  fn from(l: Local<'sc, SymbolObject>) -> Self {
    unsafe { transmute(l) }
  }
}

#[repr(C)]
pub struct WasmModuleObject(Opaque);
impl<'sc> From<Local<'sc, WasmModuleObject>> for Local<'sc, Data> {
  fn from(l: Local<'sc, WasmModuleObject>) -> Self {
    unsafe { transmute(l) }
  }
}
impl<'sc> From<Local<'sc, WasmModuleObject>> for Local<'sc, Value> {
  fn from(l: Local<'sc, WasmModuleObject>) -> Self {
    unsafe { transmute(l) }
  }
}
impl<'sc> From<Local<'sc, WasmModuleObject>> for Local<'sc, Object> {
  fn from(l: Local<'sc, WasmModuleObject>) -> Self {
    unsafe { transmute(l) }
  }
}

/// The superclass of primitive values.  See ECMA-262 4.3.2.
#[repr(C)]
pub struct Primitive(Opaque);
impl<'sc> From<Local<'sc, Primitive>> for Local<'sc, Data> {
  fn from(l: Local<'sc, Primitive>) -> Self {
    unsafe { transmute(l) }
  }
}
impl<'sc> From<Local<'sc, Primitive>> for Local<'sc, Value> {
  fn from(l: Local<'sc, Primitive>) -> Self {
    unsafe { transmute(l) }
  }
}

/// A JavaScript BigInt value (https://tc39.github.io/proposal-bigint)
#[repr(C)]
pub struct BigInt(Opaque);
impl<'sc> From<Local<'sc, BigInt>> for Local<'sc, Data> {
  fn from(l: Local<'sc, BigInt>) -> Self {
    unsafe { transmute(l) }
  }
}
impl<'sc> From<Local<'sc, BigInt>> for Local<'sc, Value> {
  fn from(l: Local<'sc, BigInt>) -> Self {
    unsafe { transmute(l) }
  }
}
impl<'sc> From<Local<'sc, BigInt>> for Local<'sc, Primitive> {
  fn from(l: Local<'sc, BigInt>) -> Self {
    unsafe { transmute(l) }
  }
}

/// A primitive boolean value (ECMA-262, 4.3.14).  Either the true
/// or false value.
#[repr(C)]
pub struct Boolean(Opaque);
impl<'sc> From<Local<'sc, Boolean>> for Local<'sc, Data> {
  fn from(l: Local<'sc, Boolean>) -> Self {
    unsafe { transmute(l) }
  }
}
impl<'sc> From<Local<'sc, Boolean>> for Local<'sc, Value> {
  fn from(l: Local<'sc, Boolean>) -> Self {
    unsafe { transmute(l) }
  }
}
impl<'sc> From<Local<'sc, Boolean>> for Local<'sc, Primitive> {
  fn from(l: Local<'sc, Boolean>) -> Self {
    unsafe { transmute(l) }
  }
}

/// A superclass for symbols and strings.
#[repr(C)]
pub struct Name(Opaque);
impl<'sc> From<Local<'sc, Name>> for Local<'sc, Data> {
  fn from(l: Local<'sc, Name>) -> Self {
    unsafe { transmute(l) }
  }
}
impl<'sc> From<Local<'sc, Name>> for Local<'sc, Value> {
  fn from(l: Local<'sc, Name>) -> Self {
    unsafe { transmute(l) }
  }
}
impl<'sc> From<Local<'sc, Name>> for Local<'sc, Primitive> {
  fn from(l: Local<'sc, Name>) -> Self {
    unsafe { transmute(l) }
  }
}

/// A JavaScript string value (ECMA-262, 4.3.17).
#[repr(C)]
pub struct String(Opaque);
impl<'sc> From<Local<'sc, String>> for Local<'sc, Data> {
  fn from(l: Local<'sc, String>) -> Self {
    unsafe { transmute(l) }
  }
}
impl<'sc> From<Local<'sc, String>> for Local<'sc, Value> {
  fn from(l: Local<'sc, String>) -> Self {
    unsafe { transmute(l) }
  }
}
impl<'sc> From<Local<'sc, String>> for Local<'sc, Primitive> {
  fn from(l: Local<'sc, String>) -> Self {
    unsafe { transmute(l) }
  }
}
impl<'sc> From<Local<'sc, String>> for Local<'sc, Name> {
  fn from(l: Local<'sc, String>) -> Self {
    unsafe { transmute(l) }
  }
}

/// A JavaScript symbol (ECMA-262 edition 6)
#[repr(C)]
pub struct Symbol(Opaque);
impl<'sc> From<Local<'sc, Symbol>> for Local<'sc, Data> {
  fn from(l: Local<'sc, Symbol>) -> Self {
    unsafe { transmute(l) }
  }
}
impl<'sc> From<Local<'sc, Symbol>> for Local<'sc, Value> {
  fn from(l: Local<'sc, Symbol>) -> Self {
    unsafe { transmute(l) }
  }
}
impl<'sc> From<Local<'sc, Symbol>> for Local<'sc, Primitive> {
  fn from(l: Local<'sc, Symbol>) -> Self {
    unsafe { transmute(l) }
  }
}
impl<'sc> From<Local<'sc, Symbol>> for Local<'sc, Name> {
  fn from(l: Local<'sc, Symbol>) -> Self {
    unsafe { transmute(l) }
  }
}

/// A JavaScript number value (ECMA-262, 4.3.20)
#[repr(C)]
pub struct Number(Opaque);
impl<'sc> From<Local<'sc, Number>> for Local<'sc, Data> {
  fn from(l: Local<'sc, Number>) -> Self {
    unsafe { transmute(l) }
  }
}
impl<'sc> From<Local<'sc, Number>> for Local<'sc, Value> {
  fn from(l: Local<'sc, Number>) -> Self {
    unsafe { transmute(l) }
  }
}
impl<'sc> From<Local<'sc, Number>> for Local<'sc, Primitive> {
  fn from(l: Local<'sc, Number>) -> Self {
    unsafe { transmute(l) }
  }
}

/// A JavaScript value representing a signed integer.
#[repr(C)]
pub struct Integer(Opaque);
impl<'sc> From<Local<'sc, Integer>> for Local<'sc, Data> {
  fn from(l: Local<'sc, Integer>) -> Self {
    unsafe { transmute(l) }
  }
}
impl<'sc> From<Local<'sc, Integer>> for Local<'sc, Value> {
  fn from(l: Local<'sc, Integer>) -> Self {
    unsafe { transmute(l) }
  }
}
impl<'sc> From<Local<'sc, Integer>> for Local<'sc, Primitive> {
  fn from(l: Local<'sc, Integer>) -> Self {
    unsafe { transmute(l) }
  }
}
impl<'sc> From<Local<'sc, Integer>> for Local<'sc, Number> {
  fn from(l: Local<'sc, Integer>) -> Self {
    unsafe { transmute(l) }
  }
}

/// A JavaScript value representing a 32-bit signed integer.
#[repr(C)]
pub struct Int32(Opaque);
impl<'sc> From<Local<'sc, Int32>> for Local<'sc, Data> {
  fn from(l: Local<'sc, Int32>) -> Self {
    unsafe { transmute(l) }
  }
}
impl<'sc> From<Local<'sc, Int32>> for Local<'sc, Value> {
  fn from(l: Local<'sc, Int32>) -> Self {
    unsafe { transmute(l) }
  }
}
impl<'sc> From<Local<'sc, Int32>> for Local<'sc, Primitive> {
  fn from(l: Local<'sc, Int32>) -> Self {
    unsafe { transmute(l) }
  }
}
impl<'sc> From<Local<'sc, Int32>> for Local<'sc, Number> {
  fn from(l: Local<'sc, Int32>) -> Self {
    unsafe { transmute(l) }
  }
}
impl<'sc> From<Local<'sc, Int32>> for Local<'sc, Integer> {
  fn from(l: Local<'sc, Int32>) -> Self {
    unsafe { transmute(l) }
  }
}

/// A JavaScript value representing a 32-bit unsigned integer.
#[repr(C)]
pub struct Uint32(Opaque);
impl<'sc> From<Local<'sc, Uint32>> for Local<'sc, Data> {
  fn from(l: Local<'sc, Uint32>) -> Self {
    unsafe { transmute(l) }
  }
}
impl<'sc> From<Local<'sc, Uint32>> for Local<'sc, Value> {
  fn from(l: Local<'sc, Uint32>) -> Self {
    unsafe { transmute(l) }
  }
}
impl<'sc> From<Local<'sc, Uint32>> for Local<'sc, Primitive> {
  fn from(l: Local<'sc, Uint32>) -> Self {
    unsafe { transmute(l) }
  }
}
impl<'sc> From<Local<'sc, Uint32>> for Local<'sc, Number> {
  fn from(l: Local<'sc, Uint32>) -> Self {
    unsafe { transmute(l) }
  }
}
impl<'sc> From<Local<'sc, Uint32>> for Local<'sc, Integer> {
  fn from(l: Local<'sc, Uint32>) -> Self {
    unsafe { transmute(l) }
  }
}
