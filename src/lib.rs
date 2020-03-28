// Copyright 2019-2020 the Deno authors. All rights reserved. MIT license.

//! # Example
//!
//! ```rust
//! use rusty_v8 as v8;
//!
//! let platform = v8::new_default_platform();
//! v8::V8::initialize_platform(platform);
//! v8::V8::initialize();
//!
//! let mut create_params = v8::Isolate::create_params();
//! create_params.set_array_buffer_allocator(v8::new_default_allocator());
//! let mut isolate = v8::Isolate::new(create_params);
//!
//! let mut handle_scope = v8::HandleScope::new(&mut isolate);
//! let scope = handle_scope.enter();
//!
//! let context = v8::Context::new(scope);
//! let mut context_scope = v8::ContextScope::new(scope, context);
//! let scope = context_scope.enter();
//!
//! let code = v8::String::new(scope, "'Hello' + ' World!'").unwrap();
//! println!("javascript code: {}", code.to_rust_string_lossy(scope));
//!
//! let mut script = v8::Script::compile(scope, context, code, None).unwrap();
//! let result = script.run(scope, context).unwrap();
//! let result = result.to_string(scope).unwrap();
//! println!("result: {}", result.to_rust_string_lossy(scope));
//! ```
//!
//! # Design of Scopes
//!
//! Although the end is in sight, the design is still a bit in flux.
//!
//! The general idea is that the various scope classes mediate access to the v8
//! Isolate and the various items on its heap (Local/Global handles,
//! return/escape slots, etc.). At any point in time there exists only one scope
//! object that is directly accessible, which guarantees that all interactions
//! with the Isolate are safe.
//!
//! A Scope as such is not a trait (there is currently an internal
//! ScopeDefinition trait but that's only there to make implementation easier).
//!
//! Rather, there are a number of traits that are implemented for the scopes
//! they're applicable to, you've probably seen most of them already. The
//! InIsolate which gives access to &mut Isolate is implemented for all scopes,
//! ToLocal (I might rename that) is implemented for all Scopes in which new
//! Local handles can be created and it sets the appropriate lifetime on them.
//!
//! Furthermore, many callbacks will receive receive an appropriate Scope object
//! as their first argument, which 'encodes' the the state the isolate is in
//! when the callback is called. E.g. a FunctionCallbackScope implements
//! InIsolate + and ToLocal (it acts as a HandleScope).
//! HostImportModuleDynamicallyScope would also implement InIsolate plus
//! EscapeLocal (it doesn't act like a HandleScope, but it lets you safely
//! escape one MaybeLocal which is returned to the caller).
//!
//! In a nutshell, that's it.
//!
//! Open TODOs are:
//! - Add these automatic scopes to more callbacks (in progress) and get rid of
//!   the necessity to import the MapFnTo trait.
//! - Fully integrate TryCatch blocks into the scope system (currently a
//!   TryCatch works like a scope internally but it's not integrated).
//! - Add methods to some on some of the scopes like get_context() for ContextScope.
//! - Rename/reorganize/document.

#![allow(clippy::missing_safety_doc)]
#![allow(dead_code)]

#[macro_use]
extern crate bitflags;
#[macro_use]
extern crate lazy_static;
extern crate libc;

mod array_buffer;
mod array_buffer_view;
mod context;
mod data;
mod exception;
mod external_references;
mod function;
mod global;
mod handle_scope;
mod isolate;
mod local;
mod module;
mod number;
mod object;
mod platform;
mod primitive_array;
mod primitives;
mod promise;
mod property_attribute;
mod proxy;
mod scope_traits;
mod script;
mod script_or_module;
mod shared_array_buffer;
mod snapshot;
mod string;
mod support;
mod template;
mod try_catch;
mod uint8_array;
mod value;

pub mod inspector;
pub mod json;
pub mod scope;
pub mod script_compiler;
// This module is intentionally named "V8" rather than "v8" to match the
// C++ namespace "v8::V8".
#[allow(non_snake_case)]
pub mod V8;

pub use array_buffer::*;
pub use data::*;
pub use exception::*;
pub use external_references::ExternalReference;
pub use external_references::ExternalReferences;
pub use function::*;
pub use global::{DefaultWeakable, Global, WeakCallback, Weakable};
pub use handle_scope::EscapableHandleScope;
pub use handle_scope::HandleScope;
pub use isolate::CreateParams;
pub use isolate::GarbageCollectionType;
pub use isolate::HostImportModuleDynamicallyCallback;
pub use isolate::HostInitializeImportMetaObjectCallback;
pub use isolate::Isolate;
pub use isolate::IsolateHandle;
pub use isolate::MessageCallback;
pub use isolate::OwnedIsolate;
pub use isolate::PromiseRejectCallback;
pub use local::Local;
pub use module::*;
pub use object::*;
pub use platform::new_default_platform;
pub use platform::Platform;
pub use platform::Task;
// TODO(ry) TaskBase and TaskImpl ideally shouldn't be part of the public API.
pub use platform::TaskBase;
pub use platform::TaskImpl;
pub use primitives::*;
pub use promise::{PromiseRejectEvent, PromiseRejectMessage, PromiseState};
pub use property_attribute::*;
pub use proxy::*;
pub use scope::CallbackScope;
pub use scope::ContextScope;
pub use scope::FunctionCallbackScope;
pub use scope::PropertyCallbackScope;
pub use scope::Scope;
pub use scope_traits::*;
pub use script::ScriptOrigin;
pub use snapshot::FunctionCodeHandling;
pub use snapshot::OwnedStartupData;
pub use snapshot::SnapshotCreator;
pub use snapshot::StartupData;
pub use string::NewStringType;
pub use support::SharedRef;
pub use support::UniquePtr;
pub use support::UniqueRef;
pub use template::*;
pub use try_catch::{TryCatch, TryCatchScope};

// TODO(piscisaureus): Ideally this trait would not be exported.
pub use support::MapFnTo;
