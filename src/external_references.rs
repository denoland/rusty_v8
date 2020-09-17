// Copyright 2019-2020 the Deno authors. All rights reserved. MIT license.
use crate::impl_accessor_name_getter_callback;
use crate::impl_function_callback;
use crate::impl_message_callback;
use crate::support::intptr_t;
use crate::RawAccessorNameGetterCallback;
use crate::RawFunctionCallback;
use crate::RawMessageCallback;
use std::mem::size_of;

#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct ExternalReference(intptr_t);

impl ExternalReference {
  fn new<P>(pointer: P) -> Self {
    assert_eq!(size_of::<P>(), size_of::<intptr_t>());
    let address = unsafe { *(&pointer as *const P as *const intptr_t) };
    Self(address)
  }

  pub fn accessor_name_getter_callback(
    cb: impl_accessor_name_getter_callback!(),
  ) -> Self {
    Self::new::<RawAccessorNameGetterCallback>(cb.into())
  }

  pub fn function_callback(cb: impl_function_callback!()) -> Self {
    Self::new::<RawFunctionCallback>(cb.into())
  }

  pub fn message_callback(cb: impl_message_callback!()) -> Self {
    Self::new::<RawMessageCallback>(cb.into())
  }
}

pub struct ExternalReferences {
  null_terminated: Vec<intptr_t>,
}

unsafe impl Sync for ExternalReferences {}

impl ExternalReferences {
  pub fn new(refs: &[ExternalReference]) -> Self {
    let null_terminated = refs
      .iter()
      .map(|&ext_ref| ext_ref.0)
      .inspect(|&address| assert_ne!(address, 0))
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
