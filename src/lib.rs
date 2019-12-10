// Copyright 2018-2019 the Deno authors. All rights reserved. MIT license.

#![warn(clippy::all)]
#![allow(clippy::missing_safety_doc)]
#![allow(clippy::new_without_default)]
#![allow(dead_code)]

#[macro_use]
extern crate bitflags;
#[macro_use]
extern crate lazy_static;
extern crate libc;

mod context;
mod exception;
mod function;
mod handle_scope;
mod isolate;
mod json;
mod local;
mod locker;
mod number;
mod object;
mod primitives;
mod script;
mod string;
mod support;
mod value;

pub mod array_buffer;
pub mod inspector;
pub mod platform;
// This module is intentionally named "V8" rather than "v8" to match the
// C++ namespace "v8::V8".
#[allow(non_snake_case)]
pub mod V8;

pub use context::Context;
pub use exception::Exception;
pub use function::{
  Function, FunctionCallback, FunctionCallbackInfo, FunctionTemplate,
};
pub use handle_scope::HandleScope;
pub use isolate::Isolate;
pub use json::JSON;
pub use local::Local;
pub use locker::Locker;
pub use number::{Integer, Number};
pub use object::Object;
pub use primitives::*;
pub use script::{Script, ScriptOrigin};
pub use string::NewStringType;
pub use string::String;
pub use value::Value;
