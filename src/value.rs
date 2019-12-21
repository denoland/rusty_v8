use crate::support;
use crate::Local;
use crate::MaybeLocal;
use std::mem::MaybeUninit;

extern "C" {
  fn v8__Value__IsUndefined(this: &Value) -> bool;
  fn v8__Value__IsNull(this: &Value) -> bool;
  fn v8__Value__IsNullOrUndefined(this: &Value) -> bool;
  fn v8__Value__IsString(this: &Value) -> bool;
  fn v8__Value__IsNumber(this: &Value) -> bool;
  fn v8__Value__MaybeLocal(
    this: *mut Value,
    out: &mut MaybeUninit<MaybeLocal<Value>>,
  );
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

impl Into<MaybeLocal<Value>> for Local<'_, Value> {
  fn into(mut self) -> MaybeLocal<Value> {
    let mut ptr = MaybeUninit::<MaybeLocal<Value>>::uninit();
    unsafe {
      v8__Value__MaybeLocal(&mut *self, &mut ptr);
      ptr.assume_init()
    }
  }
}
