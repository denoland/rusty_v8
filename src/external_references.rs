// Copyright 2019-2020 the Deno authors. All rights reserved. MIT license.
use crate::support::intptr_t;
use crate::AccessorNameGetterCallback;
use crate::FunctionCallback;
use crate::MessageCallback;

#[derive(Clone, Copy)]
pub union ExternalReference {
  pub function: FunctionCallback,
  pub getter: AccessorNameGetterCallback,
  pub message: MessageCallback,
}

pub struct ExternalReferences {
  null_terminated: Vec<*const std::ffi::c_void>,
}

unsafe impl Sync for ExternalReferences {}

impl ExternalReferences {
  pub fn new(refs: &[ExternalReference]) -> Self {
    let mut null_terminated = Vec::with_capacity(refs.len() + 1);
    for r in refs {
      let ptr = unsafe { std::mem::transmute(*r) };
      null_terminated.push(ptr);
    }
    null_terminated.push(std::ptr::null());
    Self { null_terminated }
  }

  pub fn as_ptr(&self) -> *const intptr_t {
    self.null_terminated.as_ptr() as *const intptr_t
  }
}
