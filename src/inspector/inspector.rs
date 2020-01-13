use super::channel::AsChannel;
use super::client::AsClient;
use super::context_info::V8ContextInfo;
use super::session::V8InspectorSession;
use super::StringView;
use crate::support::int;
use crate::support::UniquePtr;
use crate::Isolate;

extern "C" {

  // -> *mut v8_inspector::V8InspectorSession
}

pub struct V8Inspector {}

impl V8Inspector {
  pub fn create<T>(isolate: &Isolate, client: &T) -> Self
  where
    T: AsClient,
  {
    todo!()
  }

  pub fn connect<T>(
    &mut self,
    context_group_id: int,
    channel: &T,
    state: &StringView,
  ) -> UniquePtr<V8InspectorSession>
  where
    T: AsChannel,
  {
    todo!()
  }

  pub fn context_created(&mut self, context_info: &V8ContextInfo) {}
}
