// Copyright 2019-2020 the Deno authors. All rights reserved. MIT license.
//! A JSON Parser and Stringifier.
use crate::Context;
use crate::Local;
use crate::String;
use crate::ToLocal;
use crate::Value;

extern "C" {
  fn v8__JSON__Parse(
    context: *const Context,
    json_string: *const String,
  ) -> *const Value;
  fn v8__JSON__Stringify(
    context: *const Context,
    json_object: *const Value,
  ) -> *const String;
}

/// Tries to parse the string `json_string` and returns it as value if
/// successful.
pub fn parse<'sc>(
  scope: &mut impl ToLocal<'sc>,
  context: Local<'_, Context>,
  json_string: Local<'_, String>,
) -> Option<Local<'sc, Value>> {
  unsafe { scope.cast_local(|_| v8__JSON__Parse(&*context, &*json_string)) }
}

/// Tries to stringify the JSON-serializable object `json_object` and returns
/// it as string if successful.
pub fn stringify<'sc>(
  scope: &mut impl ToLocal<'sc>,
  context: Local<'sc, Context>,
  json_object: Local<'sc, Value>,
) -> Option<Local<'sc, String>> {
  unsafe { scope.cast_local(|_| v8__JSON__Stringify(&*context, &*json_object)) }
}
