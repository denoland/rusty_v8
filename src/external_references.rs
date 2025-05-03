// Copyright 2019-2021 the Deno authors. All rights reserved. MIT license.

use crate::FunctionCallback;
use crate::IndexedDefinerCallback;
use crate::IndexedDeleterCallback;
use crate::IndexedGetterCallback;
use crate::IndexedQueryCallback;
use crate::IndexedSetterCallback;
use crate::MessageCallback;
use crate::NamedDefinerCallback;
use crate::NamedDeleterCallback;
use crate::NamedGetterCallback;
use crate::NamedQueryCallback;
use crate::NamedSetterCallback;
use crate::PropertyEnumeratorCallback;
use crate::fast_api::CFunctionInfo;
use std::ffi::c_void;
use std::fmt::Debug;

#[derive(Clone, Copy)]
pub union ExternalReference<'s> {
  pub function: FunctionCallback,
  pub named_getter: NamedGetterCallback<'s>,
  pub named_setter: NamedSetterCallback<'s>,
  pub named_definer: NamedDefinerCallback<'s>,
  pub named_deleter: NamedDeleterCallback<'s>,
  pub named_query: NamedQueryCallback<'s>,
  pub indexed_getter: IndexedGetterCallback<'s>,
  pub indexed_setter: IndexedSetterCallback<'s>,
  pub indexed_definer: IndexedDefinerCallback<'s>,
  pub indexed_deleter: IndexedDeleterCallback<'s>,
  pub indexed_query: IndexedQueryCallback<'s>,
  pub enumerator: PropertyEnumeratorCallback<'s>,
  pub message: MessageCallback,
  pub pointer: *mut c_void,
  pub type_info: *const CFunctionInfo,
}

impl Debug for ExternalReference<'_> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    // SAFETY: All union fields are the same size
    unsafe { (self.pointer).fmt(f) }
  }
}

impl PartialEq for ExternalReference<'_> {
  fn eq(&self, other: &Self) -> bool {
    unsafe { self.pointer.eq(&other.pointer) }
  }
}
