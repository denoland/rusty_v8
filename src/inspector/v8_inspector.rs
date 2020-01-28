use super::channel::AsChannel;
use super::client::AsV8InspectorClient;
use super::session::V8InspectorSession;
use super::Channel;
use super::StringView;
use super::V8InspectorClient;
use crate::support::int;
use crate::support::Delete;
use crate::support::Opaque;
use crate::support::UniqueRef;
use crate::Context;
use crate::Isolate;
use crate::Local;

extern "C" {
  fn v8_inspector__V8Inspector__DELETE(this: &'static mut V8Inspector);
  fn v8_inspector__V8Inspector__create(
    isolate: *mut Isolate,
    client: *mut V8InspectorClient,
  ) -> *mut V8Inspector;
  fn v8_inspector__V8Inspector__connect(
    inspector: *mut V8Inspector,
    context_group_id: int,
    channel: *mut Channel,
    state: *const StringView,
  ) -> *mut V8InspectorSession;
  fn v8_inspector__V8Inspector__contextCreated(
    inspector: *mut V8Inspector,
    context: *mut Context,
    context_group_id: int,
    human_readable_name: *const StringView,
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
      UniqueRef::from_raw(v8_inspector__V8Inspector__create(
        isolate,
        client.as_client_mut(),
      ))
    }
  }

  pub fn connect<T>(
    &mut self,
    context_group_id: i32,
    channel: &mut T,
    state: &StringView,
  ) -> UniqueRef<V8InspectorSession>
  where
    T: AsChannel,
  {
    unsafe {
      UniqueRef::from_raw(v8_inspector__V8Inspector__connect(
        self,
        context_group_id,
        channel.as_channel_mut(),
        state,
      ))
    }
  }

  /// Note: this method deviates from the C++ API here because it's a lot of
  /// work to bind the V8ContextInfo, which is not used elsewhere.
  pub fn context_created(
    &mut self,
    mut context: Local<Context>,
    context_group_id: i32,
    human_readable_name: &StringView,
  ) {
    unsafe {
      v8_inspector__V8Inspector__contextCreated(
        self,
        &mut *context,
        context_group_id,
        human_readable_name,
      )
    }
  }
}

impl Delete for V8Inspector {
  fn delete(&'static mut self) {
    unsafe { v8_inspector__V8Inspector__DELETE(self) };
  }
}
