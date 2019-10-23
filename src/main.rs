#![warn(clippy::all)]
#![allow(dead_code)]
#![allow(non_snake_case)]

mod support;
mod v8;

mod example {
  use crate::support::UniquePtr;
  use crate::v8::inspector::channel::*;
  use crate::v8::*;

  // Using repr(C) to preserve field ordering and test that everything works
  // when the ChannelBase field is not the first element of the struct.
  #[repr(C)]
  pub struct TestChannel {
    field1: i32,
    channel_base: ChannelBase,
    field2: f64,
  }

  impl ChannelImpl for TestChannel {
    fn base(&self) -> &ChannelBase {
      &self.channel_base
    }
    fn base_mut(&mut self) -> &mut ChannelBase {
      &mut self.channel_base
    }
    fn sendResponse(
      &mut self,
      call_id: i32,
      mut message: UniquePtr<StringBuffer>,
    ) {
      println!(
        "call_id: {:?}, message: '{:?}'",
        call_id,
        message.as_mut().unwrap().string().characters16().unwrap()
      );
    }
    fn sendNotification(&mut self, _message: UniquePtr<StringBuffer>) {}
    fn flushProtocolNotifications(&mut self) {}
  }

  impl TestChannel {
    pub fn new() -> Self {
      Self {
        channel_base: ChannelBase::new::<Self>(),
        field1: -42,
        field2: 4.2,
      }
    }
  }
}

fn main() {
  use crate::v8::inspector::channel::*;
  use crate::v8::*;
  use example::*;
  let mut ex = TestChannel::new();
  let chan = ex.as_channel_mut();
  let message: &[u8] = b"hello";
  let message = StringView::from(message);
  let message = StringBuffer::create(&message);
  chan.sendResponse(3, message);
}
