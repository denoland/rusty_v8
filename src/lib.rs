#![warn(clippy::all)]
#![allow(dead_code)]

pub mod inspector;
pub mod platform;
pub mod string_buffer;
pub mod string_view;
pub mod support;

pub use string_buffer::StringBuffer;
pub use string_view::StringView;
