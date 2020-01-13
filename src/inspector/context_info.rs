use super::StringView;
use crate::support::int;
use crate::Context;
use crate::Local;

extern "C" {

  // -> *mut v8_inspector::V8InspectorSession
}

pub struct V8ContextInfo {}

impl V8ContextInfo {
  pub fn new<'sc>(
    context: Local<'sc, Context>,
    context_group_id: int,
    human_readable_name: &StringView,
  ) -> Self {
    todo!()
  }
}
