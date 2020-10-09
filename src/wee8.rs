// Copyright 2019-2020 the Deno authors. All rights reserved. MIT license.

#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]

use crate::support::Opaque;
use paste::paste;

macro_rules! WASM_DECLARE_OWN {
  ($name:ident) => {
    paste! {
      #[repr(C)]
      pub struct [<wasm_ $name _t>](Opaque);
      extern "C" {
        pub fn [<wasm_ $name _delete>](_: *mut [<wasm_ $name _t>]);
      }
    }
  };
}

macro_rules! WASM_DECLARE_VEC {
  ($name:ident, $ty:ty) => {
    paste! {
      #[repr(C)]
      pub struct [<wasm_ $name _vec_t>] {
        pub size: usize,
        pub data: *mut $ty,
      }
      extern "C" {
        pub fn [<wasm_ $name _vec_new_empty>](_: *mut [<wasm_ $name _vec_t>]);
        pub fn [<wasm_ $name _vec_new_uninitialized>](
          _: *mut [<wasm_ $name _vec_t>],
          _: usize,
        );
        pub fn [<wasm_ $name _vec_new>](
          _: *mut [<wasm_ $name _vec_t>],
          _: usize,
          _: *const $ty,
        );
        pub fn [<wasm_ $name _vec_copy>](
          _: *mut [<wasm_ $name _vec_t>],
          _: *const [<wasm_ $name _vec_t>],
        );
        pub fn [<wasm_ $name _vec_delete>](_: *mut [<wasm_ $name _vec_t>]);
      }
    }
  };
}

macro_rules! WASM_DECLARE_TYPE {
  ($name:ident) => {
    paste! {
      WASM_DECLARE_OWN!($name);
      WASM_DECLARE_VEC!($name, *mut [<wasm_ $name _t>]);
      extern "C" {
        pub fn [<wasm_ $name _copy>](_: *mut [<wasm_ $name _t>]);
      }
    }
  };
}

pub type wasm_byte_t = i8;

WASM_DECLARE_VEC!(byte, wasm_byte_t);

WASM_DECLARE_OWN!(config);

extern "C" {
  pub fn wasm_config_new() -> *mut wasm_config_t;
}

WASM_DECLARE_OWN!(engine);

extern "C" {
  pub fn wasm_engine_new() -> *mut wasm_engine_t;
  pub fn wasm_engine_new_with_config(
    _: *mut wasm_config_t,
  ) -> *mut wasm_engine_t;
}

WASM_DECLARE_OWN!(store);

extern "C" {
  pub fn wasm_store_new(_: *mut wasm_engine_t) -> *mut wasm_store_t;
}

#[repr(u8)]
pub enum wasm_mutability_enum {
  WASM_CONST,
  WASM_VAR,
}

#[repr(C)]
pub struct wasm_limits_t {
  min: u32,
  max: u32,
}

const wasm_limits_max_default: u32 = 0xffffffff;

WASM_DECLARE_TYPE!(valtype);
