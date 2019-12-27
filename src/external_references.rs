// Copyright 2018-2019 the Deno authors. All rights reserved. MIT license.
use crate::function::FunctionCallback;
use crate::support::intptr_t;
use std::ffi::c_void;

pub struct ExternalReferences {
  null_terminated: Vec<*const libc::c_void>,
}

unsafe impl Sync for ExternalReferences {}

impl ExternalReferences {
  pub fn new(refs: &[FunctionCallback]) -> Self {
    let mut null_terminated = Vec::with_capacity(refs.len() + 1);
    for i in 0..refs.len() {
      null_terminated.push(refs[i] as *const c_void);
    }
    null_terminated.push(std::ptr::null());
    Self { null_terminated }
  }

  pub fn as_ptr(&self) -> *const intptr_t {
    self.null_terminated.as_ptr() as *const intptr_t
  }
}
