// Copyright 2018-2019 the Deno authors. All rights reserved. MIT license.

//! # Hello World Example
//!
//! ```
//! use rusty_v8::{Isolate, V8, Allocator, HandleScope, Script, Context, Locker, Local};
//! use rusty_v8::platform;
//!
//! fn main() {
//!   let platform = platform::new_default_platform();
//!   V8::initialize_platform(platform);
//!   V8::initialize();
//!
//!   let mut create_params = Isolate::create_params();
//!   create_params.set_array_buffer_allocator(Allocator::new_default_allocator());
//!   let isolate = Isolate::new(create_params);
//!   let mut locker = Locker::new(&isolate);
//!
//!   HandleScope::enter(&mut locker, |scope| {
//!     let mut context = Context::new(scope);
//!     context.enter();
//!     let code = rusty_v8::String::new(scope, "'Hello World!'").unwrap();
//!     code.to_rust_string_lossy(scope);
//!     let mut script = Script::compile(scope, context, code, None).unwrap();
//!     let result = script.run(scope, context).unwrap();
//!     let result: Local<rusty_v8::String> = cast(result);
//!
//!     let str = result.to_rust_string_lossy(scope);
//!
//!     println!("{}", str);
//!
//!     context.exit();
//!   });
//!
//!   drop(locker);
//! }
//!
//! // TODO Safer casts https://github.com/denoland/rusty_v8/issues/51
//! fn cast<U, T>(local: Local<T>) -> Local<U> {
//!   let cast_local: Local<U> = unsafe { std::mem::transmute_copy(&local) };
//!   cast_local
//! }
//! ```
//!

#![allow(clippy::missing_safety_doc)]
#![allow(dead_code)]

#[macro_use]
extern crate bitflags;
#[macro_use]
extern crate lazy_static;
extern crate libc;

mod array_buffer;
mod callback_scope;
mod context;
mod exception;
mod function;
mod global;
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
mod snapshot;
mod string;
mod support;
mod try_catch;
mod uint8_array;
mod value;

pub mod array_buffer_view;
pub mod inspector;
pub mod json;
pub mod platform;
pub mod scope;
pub mod script_compiler;
// This module is intentionally named "V8" rather than "v8" to match the
// C++ namespace "v8::V8".
#[allow(non_snake_case)]
pub mod V8;

pub use array_buffer::*;
pub use callback_scope::CallbackScope;
pub use context::Context;
pub use exception::*;
pub use function::{
  Function, FunctionCallbackInfo, FunctionTemplate, ReturnValue,
};
pub use global::Global;
pub use handle_scope::{EscapableHandleScope, HandleScope, ToLocal};
pub use isolate::*;
pub use local::Local;
pub use locker::Locker;
pub use module::*;
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
pub use snapshot::{FunctionCodeHandling, SnapshotCreator, StartupData};
pub use string::NewStringType;
pub use string::String;
pub use support::MaybeBool;
pub use support::SharedRef;
pub use support::UniqueRef;
pub use try_catch::{TryCatch, TryCatchScope};
pub use uint8_array::Uint8Array;
pub use value::Value;
