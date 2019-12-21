// Copyright 2018-2019 the Deno authors. All rights reserved. MIT license.

#![allow(clippy::missing_safety_doc)]
#![allow(dead_code)]

#[macro_use]
extern crate bitflags;
#[macro_use]
extern crate lazy_static;
extern crate libc;

mod array_buffer;
mod context;
mod exception;
mod function;
mod handle_scope;
mod isolate;
mod local;
mod locker;
mod module;
mod number;
mod object;
mod primitive_array;
mod primitives;
mod promise;
mod property;
mod script;
mod script_or_module;
mod string;
mod support;
mod try_catch;
mod value;

pub mod inspector;
pub mod json;
pub mod platform;
pub mod script_compiler;
// This module is intentionally named "V8" rather than "v8" to match the
// C++ namespace "v8::V8".
#[allow(non_snake_case)]
pub mod V8;

// Rather than use the namespace array_buffer, which will contain only the
// Allocator we opt in Rust to allow it to live in the top level: v8::Allocator
pub use array_buffer::Allocator;
pub use context::Context;
pub use exception::*;
pub use function::{
  Function, FunctionCallbackInfo, FunctionTemplate, ReturnValue,
};
pub use handle_scope::HandleScope;
pub use isolate::Isolate;
pub use isolate::OwnedIsolate;
pub use local::Local;
pub use locker::Locker;
pub use module::Module;
pub use number::{Integer, Number};
pub use object::Object;
pub use primitive_array::PrimitiveArray;
pub use primitives::*;
pub use promise::{
  Promise, PromiseRejectEvent, PromiseRejectMessage, PromiseResolver,
  PromiseState,
};
pub use property::PropertyCallbackInfo;
pub use script::{Script, ScriptOrigin};
pub use script_or_module::ScriptOrModule;
pub use string::NewStringType;
pub use string::String;
pub use try_catch::{TryCatch, TryCatchScope};
pub use value::Value;
