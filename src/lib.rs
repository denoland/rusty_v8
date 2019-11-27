// Copyright 2018-2019 the Deno authors. All rights reserved. MIT license.

#![warn(clippy::all)]
#![allow(clippy::missing_safety_doc)]
#![allow(clippy::new_without_default)]
#![allow(dead_code)]

#[macro_use]
extern crate lazy_static;
extern crate libc;

pub mod array_buffer;
pub mod handle_scope;
pub mod inspector;
pub mod isolate;
pub mod locker;
pub mod platform;
pub mod string_buffer;
pub mod string_view;
pub mod support;
pub mod v8;

pub use isolate::Isolate;
pub use locker::Locker;
pub use string_buffer::StringBuffer;
pub use string_view::StringView;
