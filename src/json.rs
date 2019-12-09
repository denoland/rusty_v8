#![allow(non_snake_case)]

use crate::Context;
use crate::Local;
use crate::String;
use crate::Value;

extern "C" {
  fn v8__JSON__Parse(
    context: *mut Context,
    json_string: *mut String,
  ) -> *mut Value;
  fn v8__JSON__Stringify(
    context: *mut Context,
    json_object: *mut Value,
  ) -> *mut String;
}

/// A JSON Parser and Stringifier
pub mod JSON {
  use super::*;

  /// Tries to parse the string `json_string` and returns it as value if
  /// successful.
  pub fn Parse<'sc>(
    mut context: Local<'sc, Context>,
    mut json_string: Local<'sc, String>,
  ) -> Option<Local<'sc, Value>> {
    unsafe {
      Local::from_raw(v8__JSON__Parse(&mut *context, &mut *json_string))
    }
  }

  /// Tries to stringify the JSON-serializable object `json_object` and returns
  /// it as string if successful.
  pub fn Stringify<'sc>(
    mut context: Local<'sc, Context>,
    mut json_object: Local<'sc, Value>,
  ) -> Option<Local<'sc, String>> {
    unsafe {
      Local::from_raw(v8__JSON__Stringify(&mut *context, &mut *json_object))
    }
  }
}
