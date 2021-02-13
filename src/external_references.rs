// Copyright 2019-2021 the Deno authors. All rights reserved. MIT license.
use crate::support::intptr_t;
use crate::AccessorNameGetterCallback;
use crate::FunctionCallback;
use crate::MessageCallback;

#[derive(Clone, Copy)]
pub union ExternalReference<'s> {
  pub function: FunctionCallback,
  pub getter: AccessorNameGetterCallback<'s>,
  pub message: MessageCallback,
}

#[derive(Debug)]
pub struct ExternalReferences {
  null_terminated: Vec<intptr_t>,
}

unsafe impl Sync for ExternalReferences {}

impl ExternalReferences {
  pub fn new(refs: &[ExternalReference]) -> Self {
    let null_terminated = refs
      .iter()
      .map(|&r| unsafe { std::mem::transmute(r) })
      .chain(std::iter::once(0)) // Add null terminator.
      .collect::<Vec<intptr_t>>();
    Self { null_terminated }
  }

  pub fn as_ptr(&self) -> *const intptr_t {
    self.null_terminated.as_ptr()
  }
}

impl std::ops::Deref for ExternalReferences {
  type Target = [intptr_t];
  fn deref(&self) -> &Self::Target {
    &*self.null_terminated
  }
}

impl std::borrow::Borrow<[intptr_t]> for ExternalReferences {
  fn borrow(&self) -> &[intptr_t] {
    &**self
  }
}
