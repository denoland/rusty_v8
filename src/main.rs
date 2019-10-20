#![allow(dead_code)]
#![allow(non_snake_case)]

mod support;
mod v8;

mod example {
  use crate::support::UniquePtr;
  use crate::v8::inspector::channel::*;
  use crate::v8::*;

  pub struct Example {
    a: i32,
    channel_base: ChannelBase,
    b: i32,
  }

  impl ChannelImpl for Example {
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

  impl Example {
    pub fn new() -> Self {
      Self {
        channel_base: ChannelBase::new::<Self>(),
        a: 2,
        b: 3,
      }
    }
  }
}

fn main() {
  use crate::v8::inspector::channel::*;
  use crate::v8::*;
  use example::*;
  let mut ex = Example::new();
  let chan = ex.as_channel_mut();
  let message = b"hello";
  let message = StringView::from(&message[..]);
  let message = StringBuffer::create(&message);
  chan.sendResponse(3, message);
}
