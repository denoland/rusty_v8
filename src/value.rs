use crate::Context;
use crate::Local;
use crate::Number;
use crate::Object;
use crate::String;
use crate::ToLocal;
use crate::Value;

extern "C" {
  fn v8__Value__IsUndefined(this: &Value) -> bool;
  fn v8__Value__IsNull(this: &Value) -> bool;
  fn v8__Value__IsNullOrUndefined(this: &Value) -> bool;
  fn v8__Value__IsString(this: &Value) -> bool;
  fn v8__Value__IsNumber(this: &Value) -> bool;
  fn v8__Value__IsArray(this: &Value) -> bool;
  fn v8__Value__IsFunction(this: &Value) -> bool;
  fn v8__Value__IsObject(this: &Value) -> bool;
  fn v8__Value__StrictEquals(this: &Value, that: &Value) -> bool;
  fn v8__Value__SameValue(this: &Value, that: &Value) -> bool;
  fn v8__Value__ToString(this: &Value, context: *mut Context) -> *mut String;
  fn v8__Value__ToNumber(this: &Value, context: *mut Context) -> *mut Number;
  fn v8__Value__ToObject(this: &Value, context: *mut Context) -> *mut Object;
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

  /// Returns true if this value is an instance of the String type.
  /// See ECMA-262 8.4.
  pub fn is_string(&self) -> bool {
    unsafe { v8__Value__IsString(self) }
  }

  /// Returns true if this value is an array. Note that it will return false for
  /// an Proxy for an array.
  pub fn is_array(&self) -> bool {
    unsafe { v8__Value__IsArray(self) }
  }

  /// Returns true if this value is a function.
  pub fn is_function(&self) -> bool {
    unsafe { v8__Value__IsFunction(self) }
  }

  /// Returns true if this value is an object.
  pub fn is_object(&self) -> bool {
    unsafe { v8__Value__IsObject(self) }
  }

  /// Returns true if this value is a number.
  pub fn is_number(&self) -> bool {
    unsafe { v8__Value__IsNumber(self) }
  }

  pub fn strict_equals<'sc>(&self, that: Local<'sc, Value>) -> bool {
    unsafe { v8__Value__StrictEquals(self, &that) }
  }

  pub fn same_value<'sc>(&self, that: Local<'sc, Value>) -> bool {
    unsafe { v8__Value__SameValue(self, &that) }
  }

  pub fn to_string<'sc>(
    &self,
    scope: &mut impl ToLocal<'sc>,
  ) -> Option<Local<'sc, String>> {
    let isolate = scope.isolate();
    let mut context = isolate.get_current_context();
    unsafe { Local::from_raw(v8__Value__ToString(self, &mut *context)) }
  }

  pub fn to_number<'sc>(
    &self,
    scope: &mut impl ToLocal<'sc>,
  ) -> Option<Local<'sc, Number>> {
    let isolate = scope.isolate();
    let mut context = isolate.get_current_context();
    unsafe { Local::from_raw(v8__Value__ToNumber(self, &mut *context)) }
  }

  pub fn to_object<'sc>(
    &self,
    scope: &mut impl ToLocal<'sc>,
  ) -> Option<Local<'sc, Object>> {
    let isolate = scope.isolate();
    let mut context = isolate.get_current_context();
    unsafe { Local::from_raw(v8__Value__ToObject(self, &mut *context)) }
  }
}
