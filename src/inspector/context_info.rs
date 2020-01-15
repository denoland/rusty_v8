#![allow(unused)]

use super::StringView;
use crate::support::int;
use crate::support::Opaque;
use crate::support::UniqueRef;
use crate::Context;
use crate::Local;
use std::marker::PhantomData;
use std::mem::MaybeUninit;

extern "C" {
  fn v8_inspector__V8ContextInfo__CONSTRUCT(
    buf: &mut MaybeUninit<V8ContextInfo>,
    context: *mut Context,
    context_group_id: int,
    human_readable_name: *mut StringView,
  );
}

#[repr(C)]
pub struct V8ContextInfo<'sc>([usize; 12], PhantomData<&'sc ()>);

impl<'sc> V8ContextInfo<'sc> {
  pub fn new(
    mut context: Local<Context>,
    context_group_id: int,
    human_readable_name: &mut StringView,
  ) -> Self {
    unsafe {
      let mut buf = std::mem::MaybeUninit::<V8ContextInfo>::uninit();
      v8_inspector__V8ContextInfo__CONSTRUCT(
        &mut buf,
        &mut *context,
        context_group_id,
        human_readable_name,
      );
      buf.assume_init()
    }
  }
}
