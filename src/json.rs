// Copyright 2019-2020 the Deno authors. All rights reserved. MIT license.
//! A JSON Parser and Stringifier.
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

/// Tries to parse the string `json_string` and returns it as value if
/// successful.
pub fn parse<'sc>(
  mut context: Local<Context>,
  mut json_string: Local<String>,
) -> Option<Local<Value>> {
  unsafe { Local::from_raw(v8__JSON__Parse(&mut *context, &mut *json_string)) }
}

/// Tries to stringify the JSON-serializable object `json_object` and returns
/// it as string if successful.
pub fn stringify<'sc>(
  mut context: Local<Context>,
  mut json_object: Local<Value>,
) -> Option<Local<String>> {
  unsafe {
    Local::from_raw(v8__JSON__Stringify(&mut *context, &mut *json_object))
  }
}
