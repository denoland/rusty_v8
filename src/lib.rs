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

mod handle_scope;
mod inspector;
mod isolate;
mod local;
mod locker;
mod number;
mod string;
mod string_buffer;
mod string_view;
mod support;

pub mod array_buffer;
pub mod platform;
// This module is intentionally named "V8" rather than "v8" to match the
// C++ namespace "v8::V8".
#[allow(non_snake_case)]
pub mod V8;

pub use handle_scope::HandleScope;
pub use isolate::Isolate;
pub use local::Local;
pub use locker::Locker;
pub use number::{Integer, Number};
pub use string::NewStringType;
pub use string::String;
pub use string_buffer::StringBuffer;
pub use string_view::StringView;
