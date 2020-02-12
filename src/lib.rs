// Copyright 2019-2020 the Deno authors. All rights reserved. MIT license.
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
mod scope;
mod script;
mod script_or_module;
mod shared_array_buffer;
mod snapshot;
mod string;
mod support;
mod template;
pub mod try_catch;
mod uint8_array;
mod value;

pub mod inspector;
pub mod json;
pub mod script_compiler;
// This module is intentionally named "V8" rather than "v8" to match the
// C++ namespace "v8::V8".
#[allow(non_snake_case)]
pub mod V8;

pub use array_buffer::*;
pub use context::Context;
pub use data::*;
pub use exception::*;
pub use external_references::ExternalReference;
pub use external_references::ExternalReferences;
pub use function::*;
pub use global::Global;
pub use isolate::CreateParams;
pub use isolate::HostImportModuleDynamicallyCallback;
pub use isolate::HostInitializeImportMetaObjectCallback;
pub use isolate::Isolate;
pub use isolate::MessageCallback;
pub use isolate::OwnedIsolate;
pub use isolate::PromiseRejectCallback;
pub use local::Local;
pub use module::*;
pub use object::*;
pub use platform::new_default_platform;
pub use platform::Platform;
pub use platform::Task;
pub use platform::TaskBase;
pub use platform::TaskImpl;
pub use primitive_array::PrimitiveArray;
pub use primitives::*;
pub use promise::{PromiseRejectEvent, PromiseRejectMessage, PromiseState};
pub use property_attribute::*;
pub use scope::ContextScope;
pub use scope::EscapableHandleScope;
pub use scope::HandleScope;
pub use scope::Scope;
pub use script::{Script, ScriptOrigin};
pub use script_or_module::ScriptOrModule;
pub use snapshot::FunctionCodeHandling;
pub use snapshot::OwnedStartupData;
pub use snapshot::SnapshotCreator;
pub use snapshot::StartupData;
pub use string::NewStringType;
pub use support::SharedRef;
pub use support::UniquePtr;
pub use support::UniqueRef;
pub use template::*;
pub use try_catch::TryCatch;

// TODO(piscisaureus): Ideally this trait would not be exported.
pub use support::MapFnTo;
