#![allow(dead_code)]
#![allow(non_snake_case)]

mod support;
mod v8;

mod example {
  use crate::support::UniquePtr;
  use crate::v8::inspector::channel::*;
  use crate::v8::platform::task::*;
  use crate::v8::*;

  pub struct TestChannel {
    a: i32,
    channel_base: ChannelBase,
    b: i32,
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
        a: 2,
        b: 3,
      }
    }
  }

  pub struct TestTask {
    a: i32,
    base: TaskBase,
    b: i32,
  }

  impl TaskImpl for TestTask {
    fn base(&self) -> &TaskBase {
      &self.base
    }
    fn base_mut(&mut self) -> &mut TaskBase {
      &mut self.base
    }
    fn Run(&mut self) -> () {
      println!("TestTask::Run {} {}", self.a, self.b);
    }
  }

  impl TestTask {
    pub fn new() -> Self {
      Self {
        base: TaskBase::new::<Self>(),
        a: 2,
        b: 3,
      }
    }
  }

  impl Drop for TestTask {
    fn drop(&mut self) {
      println!("TestTask::drop()");
    }
  }
}

fn main1() {
  use crate::v8::inspector::channel::*;
  use crate::v8::*;
  use example::*;
  let mut ex = TestChannel::new();
  let chan = ex.as_channel_mut();
  let message = b"hello";
  let message = StringView::from(&message[..]);
  let message = StringBuffer::create(&message);
  chan.sendResponse(3, message);
}

fn main() {
  use crate::v8::platform::task::*;
  use example::*;
  let mut v = TestTask::new();
  v.Run();
  let b = Box::new(v);
  b.into_unique_ptr();
}
