#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]
include!(env!("RUSTY_V8_SRC_BINDING_PATH"));

pub use crate::Isolate as v8_Isolate;
