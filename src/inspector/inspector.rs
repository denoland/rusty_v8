use super::channel::AsChannel;
use super::client::AsClient;
use super::context_info::V8ContextInfo;
use super::session::V8InspectorSession;
use super::Channel;
use super::Client;
use super::StringBuffer;
use super::StringView;
use crate::support::int;
use crate::support::Delete;
use crate::support::UniqueRef;
use crate::Isolate;

extern "C" {
  fn v8_inspector__V8Inspector__Create(
    isolate: *mut Isolate,
    client: *mut Client,
  ) -> *mut V8Inspector;
  fn v8_inspector__V8Inspector__Connect(
    inspector: *mut V8Inspector,
    context_group_id: int,
    channel: *mut Channel,
    state: *mut StringBuffer,
  ) -> *mut V8InspectorSession;
  fn v8_inspector__V8Inspector__ContextCreated(
    inspector: *mut V8Inspector,
    context: *const V8ContextInfo,
  );
}

pub struct V8Inspector {}

impl V8Inspector {
  pub fn create<T>(
    isolate: &mut Isolate,
    client: &mut T,
  ) -> UniqueRef<V8Inspector>
  where
    T: AsClient,
  {
    unsafe {
      UniqueRef::from_raw(v8_inspector__V8Inspector__Create(
        isolate,
        client.as_client_mut(),
      ))
    }
  }

  pub fn connect<T>(
    &mut self,
    context_group_id: int,
    channel: &mut T,
    state: &mut StringBuffer,
  ) -> UniqueRef<V8InspectorSession>
  where
    T: AsChannel,
  {
    unsafe {
      UniqueRef::from_raw(v8_inspector__V8Inspector__Connect(
        self,
        context_group_id,
        channel.as_channel_mut(),
        &mut *state,
      ))
    }
  }

  pub fn context_created(&mut self, context_info: &V8ContextInfo) {
    unsafe { v8_inspector__V8Inspector__ContextCreated(self, &*context_info) }
  }
}

impl Delete for V8Inspector {
  fn delete(&'static mut self) {
    todo!()
  }
}
