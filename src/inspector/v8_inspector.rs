use super::channel::AsChannel;
use super::client::AsV8InspectorClient;
use super::session::V8InspectorSession;
use super::Channel;
use super::StringBuffer;
use super::V8InspectorClient;
use crate::support::int;
use crate::support::Delete;
use crate::support::Opaque;
use crate::support::UniqueRef;
use crate::Context;
use crate::Isolate;
use crate::Local;

extern "C" {
  fn v8_inspector__V8Inspector__Create(
    isolate: *mut Isolate,
    client: *mut V8InspectorClient,
  ) -> *mut V8Inspector;
  fn v8_inspector__V8Inspector__Connect(
    inspector: *mut V8Inspector,
    context_group_id: int,
    channel: *mut Channel,
    state: *mut StringBuffer,
  ) -> *mut V8InspectorSession;
  fn v8_inspector__V8Inspector__ContextCreated(
    inspector: *mut V8Inspector,
    context: *mut Context,
    context_group_id: int,
    human_readable_name: *mut StringBuffer,
  );
}

#[repr(C)]
pub struct V8Inspector(Opaque);

impl V8Inspector {
  pub fn create<T>(
    isolate: &mut Isolate,
    client: &mut T,
  ) -> UniqueRef<V8Inspector>
  where
    T: AsV8InspectorClient,
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

  pub fn context_created<'sc>(
    &mut self,
    mut context: Local<'sc, Context>,
    context_group_id: int,
    human_readable_name: &mut StringBuffer,
  ) {
    unsafe {
      v8_inspector__V8Inspector__ContextCreated(
        self,
        &mut *context,
        context_group_id,
        &mut *human_readable_name,
      )
    }
  }
}

impl Delete for V8Inspector {
  fn delete(&'static mut self) {
    // todo!()
  }
}
