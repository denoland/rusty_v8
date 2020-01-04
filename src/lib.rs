// Copyright 2019-2020 the Deno authors. All rights reserved. MIT license.

//! # Example
//!
//! ```
//! use rusty_v8 as v8;
//!
//! let platform = v8::platform::new_default_platform();
//! v8::V8::initialize_platform(platform);
//! v8::V8::initialize();
//!
//! let mut create_params = v8::Isolate::create_params();
//! create_params.set_array_buffer_allocator(v8::new_default_allocator());
//! let isolate = v8::Isolate::new(create_params);
//! let mut locker = v8::Locker::new(&isolate);
//! {
//!   let mut handle_scope = v8::HandleScope::new(&mut locker);
//!   let scope = handle_scope.enter();
//!   let mut context = v8::Context::new(scope);
//!   context.enter();
//!
//!   let code = v8::String::new(scope, "'Hello' + ' World!'").unwrap();
//!   code.to_rust_string_lossy(scope);
//!   let mut script = v8::Script::compile(scope, context, code, None).unwrap();
//!   let result = script.run(scope, context).unwrap();
//!   let result = result.to_string(scope).unwrap();
//!   let str = result.to_rust_string_lossy(scope);
//!   println!("{}", str);
//!
//!   context.exit();
//! }
//! drop(locker);
//! ```

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
mod data;
mod exception;
mod external_references;
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
mod scope_traits;
mod script;
mod script_or_module;
mod shared_array_buffer;
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
pub use callback_scope::{
  CallbackScope, FunctionCallbackScope, PropertyCallbackScope,
};
pub use context::Context;
pub use data::*;
pub use exception::*;
pub use external_references::ExternalReference;
pub use external_references::ExternalReferences;
pub use function::*;
pub use global::Global;
pub use handle_scope::{EscapableHandleScope, HandleScope};
pub use isolate::CreateParams;
pub use isolate::HostImportModuleDynamicallyCallback;
pub use isolate::HostInitializeImportMetaObjectCallback;
pub use isolate::Isolate;
pub use isolate::MessageCallback;
pub use isolate::OwnedIsolate;
pub use isolate::PromiseRejectCallback;
pub use local::Local;
pub use locker::Locker;
pub use module::*;
pub use object::*;
pub use primitive_array::PrimitiveArray;
pub use primitives::*;
pub use promise::{PromiseRejectEvent, PromiseRejectMessage, PromiseState};
pub use scope_traits::*;
pub use script::{Script, ScriptOrigin};
pub use script_or_module::ScriptOrModule;
pub use snapshot::FunctionCodeHandling;
pub use snapshot::OwnedStartupData;
pub use snapshot::SnapshotCreator;
pub use snapshot::StartupData;
pub use string::NewStringType;
pub use support::MaybeBool;
pub use support::SharedRef;
pub use support::UniqueRef;
pub use try_catch::{TryCatch, TryCatchScope};

// TODO(piscisaureus): Ideally this trait would not be exported.
pub use support::MapFnTo;
