mod cxx_util;
mod v8_inspector;

mod example {
  use crate::v8_inspector::channel::*;

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
    fn method1(&mut self, arg: i32) {
      println!("overriden method1({}) called", arg);
      self.a += self.b * arg;
      let arg = self.a;
      ChannelDefaults::method1(self.as_channel_mut(), arg);
    }
    fn method2(&self) -> i32 {
      println!("overriden method2() called");
      self.a * self.b
    }
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
  use example::*;
  let mut ex = Example::new();
  let chan = ex.as_channel_mut();
  chan.method1(3);
  println!("{}", chan.method2());
}
