#![allow(dead_code)]
#![allow(non_snake_case)]

mod cxx_util;
mod v8_inspector;

mod example {
  use crate::cxx_util::UniquePtr;
  use crate::v8_inspector::channel::*;
  use crate::v8_inspector::*;

  pub struct Example {
    a: i32,
    channel_extender: ChannelExtender,
    b: i32,
  }

  impl ChannelOverrides for Example {
    fn extender(&self) -> &ChannelExtender {
      &self.channel_extender
    }
    fn extender_mut(&mut self) -> &mut ChannelExtender {
      &mut self.channel_extender
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

  impl Example {
    pub fn new() -> Self {
      Self {
        channel_extender: ChannelExtender::new::<Self>(),
        a: 2,
        b: 3,
      }
    }
  }
}

fn main() {
  use crate::v8_inspector::channel::*;
  use crate::v8_inspector::*;
  use example::*;
  let mut ex = Example::new();
  let chan = ex.as_channel_mut();
  let message = b"hello";
  let message = StringView::from(&message[..]);
  let message = StringBuffer::create(&message);
  chan.sendResponse(3, message);
}
