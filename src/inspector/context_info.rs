use super::StringBuffer;
use crate::support::int;
use crate::support::Opaque;
use crate::support::UniqueRef;
use crate::Context;
use crate::Local;

extern "C" {
  fn v8_inspector__V8ContextInfo__New(
    context: *mut Context,
    context_group_id: int,
    human_readable_name: *mut StringBuffer,
  ) -> *mut V8ContextInfo;
}

pub struct V8ContextInfo(Opaque);

impl V8ContextInfo {
  pub fn new<'sc>(
    mut context: Local<'sc, Context>,
    context_group_id: int,
    human_readable_name: &mut StringBuffer,
  ) -> &'sc mut Self {
    unsafe {
      let ci = v8_inspector__V8ContextInfo__New(
        &mut *context,
        context_group_id,
        &mut *human_readable_name,
      );
      &mut *ci
    }
  }
}
