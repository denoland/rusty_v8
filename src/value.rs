use crate::support::Maybe;
use crate::BigInt;
use crate::Context;
use crate::Int32;
use crate::Integer;
use crate::Local;
use crate::Number;
use crate::Object;
use crate::String;
use crate::ToLocal;
use crate::Uint32;
use crate::Value;

extern "C" {
  fn v8__Value__IsUndefined(this: &Value) -> bool;
  fn v8__Value__IsNull(this: &Value) -> bool;
  fn v8__Value__IsNullOrUndefined(this: &Value) -> bool;
  fn v8__Value__IsTrue(this: &Value) -> bool;
  fn v8__Value__IsFalse(this: &Value) -> bool;
  fn v8__Value__IsName(this: &Value) -> bool;
  fn v8__Value__IsString(this: &Value) -> bool;
  fn v8__Value__IsSymbol(this: &Value) -> bool;
  fn v8__Value__IsFunction(this: &Value) -> bool;
  fn v8__Value__IsArray(this: &Value) -> bool;
  fn v8__Value__IsObject(this: &Value) -> bool;
  fn v8__Value__IsBigInt(this: &Value) -> bool;
  fn v8__Value__IsBoolean(this: &Value) -> bool;
  fn v8__Value__IsNumber(this: &Value) -> bool;
  fn v8__Value__IsExternal(this: &Value) -> bool;
  fn v8__Value__IsInt32(this: &Value) -> bool;
  fn v8__Value__IsUint32(this: &Value) -> bool;
  fn v8__Value__IsDate(this: &Value) -> bool;
  fn v8__Value__IsArgumentsObject(this: &Value) -> bool;
  fn v8__Value__IsBigIntObject(this: &Value) -> bool;
  fn v8__Value__IsBooleanObject(this: &Value) -> bool;
  fn v8__Value__IsNumberObject(this: &Value) -> bool;
  fn v8__Value__IsStringObject(this: &Value) -> bool;
  fn v8__Value__IsSymbolObject(this: &Value) -> bool;
  fn v8__Value__IsNativeError(this: &Value) -> bool;
  fn v8__Value__IsRegExp(this: &Value) -> bool;
  fn v8__Value__IsAsyncFunction(this: &Value) -> bool;
  fn v8__Value__IsGeneratorFunction(this: &Value) -> bool;
  fn v8__Value__IsGeneratorObject(this: &Value) -> bool;
  fn v8__Value__IsPromise(this: &Value) -> bool;
  fn v8__Value__IsMap(this: &Value) -> bool;
  fn v8__Value__IsSet(this: &Value) -> bool;
  fn v8__Value__IsMapIterator(this: &Value) -> bool;
  fn v8__Value__IsSetIterator(this: &Value) -> bool;
  fn v8__Value__IsWeakMap(this: &Value) -> bool;
  fn v8__Value__IsWeakSet(this: &Value) -> bool;
  fn v8__Value__IsArrayBuffer(this: &Value) -> bool;
  fn v8__Value__IsArrayBufferView(this: &Value) -> bool;
  fn v8__Value__IsTypedArray(this: &Value) -> bool;
  fn v8__Value__IsUint8Array(this: &Value) -> bool;
  fn v8__Value__IsUint8ClampedArray(this: &Value) -> bool;
  fn v8__Value__IsInt8Array(this: &Value) -> bool;
  fn v8__Value__IsUint16Array(this: &Value) -> bool;
  fn v8__Value__IsInt16Array(this: &Value) -> bool;
  fn v8__Value__IsUint32Array(this: &Value) -> bool;
  fn v8__Value__IsInt32Array(this: &Value) -> bool;
  fn v8__Value__IsFloat32Array(this: &Value) -> bool;
  fn v8__Value__IsFloat64Array(this: &Value) -> bool;
  fn v8__Value__IsBigInt64Array(this: &Value) -> bool;
  fn v8__Value__IsBigUint64Array(this: &Value) -> bool;
  fn v8__Value__IsDataView(this: &Value) -> bool;
  fn v8__Value__IsSharedArrayBuffer(this: &Value) -> bool;
  fn v8__Value__IsProxy(this: &Value) -> bool;
  fn v8__Value__IsWebAssemblyCompiledModule(this: &Value) -> bool;
  fn v8__Value__IsModuleNamespaceObject(this: &Value) -> bool;
  fn v8__Value__StrictEquals(this: &Value, that: &Value) -> bool;
  fn v8__Value__SameValue(this: &Value, that: &Value) -> bool;

  fn v8__Value__ToBigInt(this: &Value, context: *mut Context) -> *mut BigInt;
  fn v8__Value__ToNumber(this: &Value, context: *mut Context) -> *mut Number;
  fn v8__Value__ToString(this: &Value, context: *mut Context) -> *mut String;
  fn v8__Value__ToDetailString(
    this: &Value,
    context: *mut Context,
  ) -> *mut String;
  fn v8__Value__ToObject(this: &Value, context: *mut Context) -> *mut Object;
  fn v8__Value__ToInteger(this: &Value, context: *mut Context) -> *mut Integer;
  fn v8__Value__ToUint32(this: &Value, context: *mut Context) -> *mut Uint32;
  fn v8__Value__ToInt32(this: &Value, context: *mut Context) -> *mut Int32;

  fn v8__Value__NumberValue(
    this: &Value,
    context: *mut Context,
    out: *mut Maybe<f64>,
  );
  fn v8__Value__IntegerValue(
    this: &Value,
    context: *mut Context,
    out: *mut Maybe<i64>,
  );
  fn v8__Value__Uint32Value(
    this: &Value,
    context: *mut Context,
    out: *mut Maybe<u32>,
  );
  fn v8__Value__Int32Value(
    this: &Value,
    context: *mut Context,
    out: *mut Maybe<i32>,
  );
}

impl Value {
  /// Returns true if this value is the undefined value.  See ECMA-262 4.3.10.
  pub fn is_undefined(&self) -> bool {
    unsafe { v8__Value__IsUndefined(self) }
  }

  /// Returns true if this value is the null value.  See ECMA-262 4.3.11.
  pub fn is_null(&self) -> bool {
    unsafe { v8__Value__IsNull(self) }
  }

  /// Returns true if this value is either the null or the undefined value.
  /// See ECMA-262 4.3.11. and 4.3.12
  pub fn is_null_or_undefined(&self) -> bool {
    unsafe { v8__Value__IsNullOrUndefined(self) }
  }

  /// Returns true if this value is true.
  /// This is not the same as `BooleanValue()`. The latter performs a
  /// conversion to boolean, i.e. the result of `Boolean(value)` in JS, whereas
  /// this checks `value === true`.
  pub fn is_true(&self) -> bool {
    unsafe { v8__Value__IsTrue(self) }
  }

  /// Returns true if this value is false.
  /// This is not the same as `!BooleanValue()`. The latter performs a
  /// conversion to boolean, i.e. the result of `!Boolean(value)` in JS, whereas
  /// this checks `value === false`.
  pub fn is_false(&self) -> bool {
    unsafe { v8__Value__IsFalse(self) }
  }

  /// Returns true if this value is a symbol or a string.
  /// This is equivalent to
  /// `typeof value === 'string' || typeof value === 'symbol'` in JS.
  pub fn is_name(&self) -> bool {
    unsafe { v8__Value__IsName(self) }
  }

  /// Returns true if this value is an instance of the String type.
  /// See ECMA-262 8.4.
  pub fn is_string(&self) -> bool {
    unsafe { v8__Value__IsString(self) }
  }

  /// Returns true if this value is a symbol.
  /// This is equivalent to `typeof value === 'symbol'` in JS.
  pub fn is_symbol(&self) -> bool {
    unsafe { v8__Value__IsSymbol(self) }
  }

  /// Returns true if this value is a function.
  pub fn is_function(&self) -> bool {
    unsafe { v8__Value__IsFunction(self) }
  }

  /// Returns true if this value is an array. Note that it will return false for
  /// an Proxy for an array.
  pub fn is_array(&self) -> bool {
    unsafe { v8__Value__IsArray(self) }
  }

  /// Returns true if this value is an object.
  pub fn is_object(&self) -> bool {
    unsafe { v8__Value__IsObject(self) }
  }

  /// Returns true if this value is a bigint.
  /// This is equivalent to `typeof value === 'bigint'` in JS.
  pub fn is_big_int(&self) -> bool {
    unsafe { v8__Value__IsBigInt(self) }
  }

  /// Returns true if this value is boolean.
  /// This is equivalent to `typeof value === 'boolean'` in JS.
  pub fn is_boolean(&self) -> bool {
    unsafe { v8__Value__IsBoolean(self) }
  }

  /// Returns true if this value is a number.
  pub fn is_number(&self) -> bool {
    unsafe { v8__Value__IsNumber(self) }
  }

  /// Returns true if this value is an `External` object.
  pub fn is_external(&self) -> bool {
    unsafe { v8__Value__IsExternal(self) }
  }

  /// Returns true if this value is a 32-bit signed integer.
  pub fn is_int32(&self) -> bool {
    unsafe { v8__Value__IsInt32(self) }
  }

  /// Returns true if this value is a 32-bit unsigned integer.
  pub fn is_uint32(&self) -> bool {
    unsafe { v8__Value__IsUint32(self) }
  }

  /// Returns true if this value is a Date.
  pub fn is_date(&self) -> bool {
    unsafe { v8__Value__IsDate(self) }
  }

  /// Returns true if this value is an Arguments object.
  pub fn is_arguments_object(&self) -> bool {
    unsafe { v8__Value__IsArgumentsObject(self) }
  }

  /// Returns true if this value is a BigInt object.
  pub fn is_big_int_object(&self) -> bool {
    unsafe { v8__Value__IsBigIntObject(self) }
  }

  /// Returns true if this value is a Boolean object.
  pub fn is_boolean_object(&self) -> bool {
    unsafe { v8__Value__IsBooleanObject(self) }
  }

  /// Returns true if this value is a Number object.
  pub fn is_number_object(&self) -> bool {
    unsafe { v8__Value__IsNumberObject(self) }
  }

  /// Returns true if this value is a String object.
  pub fn is_string_object(&self) -> bool {
    unsafe { v8__Value__IsStringObject(self) }
  }

  /// Returns true if this value is a Symbol object.
  pub fn is_symbol_object(&self) -> bool {
    unsafe { v8__Value__IsSymbolObject(self) }
  }

  /// Returns true if this value is a NativeError.
  pub fn is_native_error(&self) -> bool {
    unsafe { v8__Value__IsNativeError(self) }
  }

  /// Returns true if this value is a RegExp.
  pub fn is_reg_exp(&self) -> bool {
    unsafe { v8__Value__IsRegExp(self) }
  }

  /// Returns true if this value is an async function.
  pub fn is_async_function(&self) -> bool {
    unsafe { v8__Value__IsAsyncFunction(self) }
  }

  /// Returns true if this value is a Generator function.
  pub fn is_generator_function(&self) -> bool {
    unsafe { v8__Value__IsGeneratorFunction(self) }
  }

  /// Returns true if this value is a Promise.
  pub fn is_promise(&self) -> bool {
    unsafe { v8__Value__IsPromise(self) }
  }

  /// Returns true if this value is a Map.
  pub fn is_map(&self) -> bool {
    unsafe { v8__Value__IsMap(self) }
  }

  /// Returns true if this value is a Set.
  pub fn is_set(&self) -> bool {
    unsafe { v8__Value__IsSet(self) }
  }

  /// Returns true if this value is a Map Iterator.
  pub fn is_map_iterator(&self) -> bool {
    unsafe { v8__Value__IsMapIterator(self) }
  }

  /// Returns true if this value is a Set Iterator.
  pub fn is_set_iterator(&self) -> bool {
    unsafe { v8__Value__IsSetIterator(self) }
  }

  /// Returns true if this value is a WeakMap.
  pub fn is_weak_map(&self) -> bool {
    unsafe { v8__Value__IsWeakMap(self) }
  }

  /// Returns true if this value is a WeakSet.
  pub fn is_weak_set(&self) -> bool {
    unsafe { v8__Value__IsWeakSet(self) }
  }

  /// Returns true if this value is an ArrayBuffer.
  pub fn is_array_buffer(&self) -> bool {
    unsafe { v8__Value__IsArrayBuffer(self) }
  }

  /// Returns true if this value is an ArrayBufferView.
  pub fn is_array_buffer_view(&self) -> bool {
    unsafe { v8__Value__IsArrayBufferView(self) }
  }

  /// Returns true if this value is one of TypedArrays.
  pub fn is_typed_array(&self) -> bool {
    unsafe { v8__Value__IsTypedArray(self) }
  }

  /// Returns true if this value is an Uint8Array.
  pub fn is_uint8_array(&self) -> bool {
    unsafe { v8__Value__IsUint8Array(self) }
  }

  /// Returns true if this value is an Uint8ClampedArray.
  pub fn is_uint8_clamped_array(&self) -> bool {
    unsafe { v8__Value__IsUint8ClampedArray(self) }
  }

  /// Returns true if this value is an Int8Array.
  pub fn is_int8_array(&self) -> bool {
    unsafe { v8__Value__IsInt8Array(self) }
  }

  /// Returns true if this value is an Uint16Array.
  pub fn is_uint16_array(&self) -> bool {
    unsafe { v8__Value__IsUint16Array(self) }
  }

  /// Returns true if this value is an Int16Array.
  pub fn is_int16_array(&self) -> bool {
    unsafe { v8__Value__IsInt16Array(self) }
  }

  /// Returns true if this value is an Uint32Array.
  pub fn is_uint32_array(&self) -> bool {
    unsafe { v8__Value__IsUint32Array(self) }
  }

  /// Returns true if this value is an Int32Array.
  pub fn is_int32_array(&self) -> bool {
    unsafe { v8__Value__IsInt32Array(self) }
  }

  /// Returns true if this value is a Float32Array.
  pub fn is_float32_array(&self) -> bool {
    unsafe { v8__Value__IsFloat32Array(self) }
  }

  /// Returns true if this value is a Float64Array.
  pub fn is_float64_array(&self) -> bool {
    unsafe { v8__Value__IsFloat64Array(self) }
  }

  /// Returns true if this value is a BigInt64Array.
  pub fn is_big_int64_array(&self) -> bool {
    unsafe { v8__Value__IsBigInt64Array(self) }
  }

  /// Returns true if this value is a BigUint64Array.
  pub fn is_big_uint64_array(&self) -> bool {
    unsafe { v8__Value__IsBigUint64Array(self) }
  }

  /// Returns true if this value is a DataView.
  pub fn is_data_view(&self) -> bool {
    unsafe { v8__Value__IsDataView(self) }
  }

  /// Returns true if this value is a SharedArrayBuffer.
  /// This is an experimental feature.
  pub fn is_shared_array_buffer(&self) -> bool {
    unsafe { v8__Value__IsSharedArrayBuffer(self) }
  }

  /// Returns true if this value is a JavaScript Proxy.
  pub fn is_proxy(&self) -> bool {
    unsafe { v8__Value__IsProxy(self) }
  }

  pub fn is_web_assembly_compiled_module(&self) -> bool {
    unsafe { v8__Value__IsWebAssemblyCompiledModule(self) }
  }

  /// Returns true if the value is a Module Namespace Object.
  pub fn is_module_namespace_object(&self) -> bool {
    unsafe { v8__Value__IsModuleNamespaceObject(self) }
  }

  pub fn strict_equals<'sc>(&self, that: impl Into<Local<'sc, Value>>) -> bool {
    unsafe { v8__Value__StrictEquals(self, &*that.into()) }
  }

  pub fn same_value<'sc>(&self, that: impl Into<Local<'sc, Value>>) -> bool {
    unsafe { v8__Value__SameValue(self, &*that.into()) }
  }

  pub fn to_big_int<'sc>(
    &self,
    scope: &mut impl ToLocal<'sc>,
  ) -> Option<Local<'sc, BigInt>> {
    let isolate = scope.isolate();
    let mut context = isolate.get_current_context();
    unsafe { Local::from_raw(v8__Value__ToBigInt(self, &mut *context)) }
  }

  pub fn to_number<'sc>(
    &self,
    scope: &mut impl ToLocal<'sc>,
  ) -> Option<Local<'sc, Number>> {
    let isolate = scope.isolate();
    let mut context = isolate.get_current_context();
    unsafe { Local::from_raw(v8__Value__ToNumber(self, &mut *context)) }
  }

  pub fn to_string<'sc>(
    &self,
    scope: &mut impl ToLocal<'sc>,
  ) -> Option<Local<'sc, String>> {
    let isolate = scope.isolate();
    let mut context = isolate.get_current_context();
    unsafe { Local::from_raw(v8__Value__ToString(self, &mut *context)) }
  }

  pub fn to_detail_string<'sc>(
    &self,
    scope: &mut impl ToLocal<'sc>,
  ) -> Option<Local<'sc, String>> {
    let isolate = scope.isolate();
    let mut context = isolate.get_current_context();
    unsafe { Local::from_raw(v8__Value__ToDetailString(self, &mut *context)) }
  }

  pub fn to_object<'sc>(
    &self,
    scope: &mut impl ToLocal<'sc>,
  ) -> Option<Local<'sc, Object>> {
    let isolate = scope.isolate();
    let mut context = isolate.get_current_context();
    unsafe { Local::from_raw(v8__Value__ToObject(self, &mut *context)) }
  }

  pub fn to_integer<'sc>(
    &self,
    scope: &mut impl ToLocal<'sc>,
  ) -> Option<Local<'sc, Integer>> {
    let isolate = scope.isolate();
    let mut context = isolate.get_current_context();
    unsafe { Local::from_raw(v8__Value__ToInteger(self, &mut *context)) }
  }

  pub fn to_uint32<'sc>(
    &self,
    scope: &mut impl ToLocal<'sc>,
  ) -> Option<Local<'sc, Uint32>> {
    let isolate = scope.isolate();
    let mut context = isolate.get_current_context();
    unsafe { Local::from_raw(v8__Value__ToUint32(self, &mut *context)) }
  }

  pub fn to_int32<'sc>(
    &self,
    scope: &mut impl ToLocal<'sc>,
  ) -> Option<Local<'sc, Int32>> {
    let isolate = scope.isolate();
    let mut context = isolate.get_current_context();
    unsafe { Local::from_raw(v8__Value__ToInt32(self, &mut *context)) }
  }

  pub fn number_value<'sc>(
    &self,
    scope: &mut impl ToLocal<'sc>,
  ) -> Option<f64> {
    let isolate = scope.isolate();
    let mut context = isolate.get_current_context();
    let mut out = Maybe::<f64>::default();
    unsafe { v8__Value__NumberValue(self, &mut *context, &mut out) };
    out.into()
  }

  pub fn integer_value<'sc>(
    &self,
    scope: &mut impl ToLocal<'sc>,
  ) -> Option<i64> {
    let isolate = scope.isolate();
    let mut context = isolate.get_current_context();
    let mut out = Maybe::<i64>::default();
    unsafe { v8__Value__IntegerValue(self, &mut *context, &mut out) };
    out.into()
  }

  pub fn uint32_value<'sc>(
    &self,
    scope: &mut impl ToLocal<'sc>,
  ) -> Option<u32> {
    let isolate = scope.isolate();
    let mut context = isolate.get_current_context();
    let mut out = Maybe::<u32>::default();
    unsafe { v8__Value__Uint32Value(self, &mut *context, &mut out) };
    out.into()
  }

  pub fn int32_value<'sc>(&self, scope: &mut impl ToLocal<'sc>) -> Option<i32> {
    let isolate = scope.isolate();
    let mut context = isolate.get_current_context();
    let mut out = Maybe::<i32>::default();
    unsafe { v8__Value__Int32Value(self, &mut *context, &mut out) };
    out.into()
  }
}
