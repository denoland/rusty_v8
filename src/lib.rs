// Copyright 2018-2019 the Deno authors. All rights reserved. MIT license.

#![warn(clippy::all)]
#![allow(dead_code)]

extern crate libc;

pub mod V8;
pub mod inspector;
pub mod platform;
pub mod string_buffer;
pub mod string_view;
pub mod support;

pub use string_buffer::StringBuffer;
pub use string_view::StringView;
