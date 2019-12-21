use crate::support;

extern "C" {
  fn v8__Value__IsUndefined(this: &Value) -> bool;
  fn v8__Value__IsNull(this: &Value) -> bool;
  fn v8__Value__IsNullOrUndefined(this: &Value) -> bool;
  fn v8__Value__IsString(this: &Value) -> bool;
  fn v8__Value__IsNumber(this: &Value) -> bool;
}

/// The superclass of all JavaScript values and objects.
#[repr(C)]
pub struct Value(support::Opaque);

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

  /// Returns true if this value is a number.
  pub fn is_number(&self) -> bool {
    unsafe { v8__Value__IsNumber(self) }
  }
}
