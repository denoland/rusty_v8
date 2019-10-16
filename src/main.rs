mod channel {
  use super::util;

  extern "C" {
    // Call a method/destructor; virtual methods use C++ dynamic dispatch.
    fn v8__Channel__DTOR(this: &mut Channel) -> ();
    fn v8__Channel__method1(this: &mut Channel, arg: i32) -> ();
    fn v8__Channel__method2(this: &Channel) -> i32;

    // Call a method of a specific class implementation, bypassing dynamic
    // dispatch. C++ equivalent: `my_channel.Channel::a()`.
    fn v8__Channel__Channel__method1(this: &mut Channel, arg: i32) -> ();

    // Constructs a special class derived from Channel that forwards all
    // virtual method invocations to rust. It is assumed that this subclass
    // has the same size and memory layout as the class it's deriving from.
    fn v8__Channel__EXTENDER__CTOR(
      buf: &mut std::mem::MaybeUninit<Channel>,
    ) -> ();
  }

    #[allow(non_snake_case)]
    #[no_mangle]
    pub unsafe extern "C"  fn v8__Channel__EXTENDER__method1(
      this: &mut Channel,
      arg: i32,
    ) {
      ChannelExtender::dispatch_mut(this).method1(arg)
    }

    #[allow(non_snake_case)]
    #[no_mangle]
    pub unsafe extern "C" fn v8__Channel__EXTENDER__method2(
      this: &Channel,
    ) -> i32 {
      ChannelExtender::dispatch(this).method2()
    }

  #[repr(C)]
  pub struct Channel {
    _cxx_vtable: *const util::Opaque,
  }

  impl Channel {
    pub fn method1(&mut self, arg: i32) {
      unsafe { v8__Channel__method1(self, arg) }
    }
    pub fn method2(&self) -> i32 {
      unsafe { v8__Channel__method2(self) }
    }
  }

  impl Drop for Channel {
    fn drop(&mut self) {
      unsafe { v8__Channel__DTOR(self) }
    }
  }

  pub trait AsChannel {
    fn as_channel(&self) -> &Channel;
    fn as_channel_mut(&mut self) -> &mut Channel;
  }

  impl AsChannel for Channel {
    fn as_channel(&self) -> &Channel {
      self
    }
    fn as_channel_mut(&mut self) -> &mut Channel {
      self
    }
  }

  impl<T> AsChannel for T
  where
    T: ChannelOverrides,
  {
    fn as_channel(&self) -> &Channel {
      &self.extender().cxx_channel
    }
    fn as_channel_mut(&mut self) -> &mut Channel {
      &mut self.extender_mut().cxx_channel
    }
  }

  pub struct ChannelDefaults;
  impl ChannelDefaults {
    pub fn method1(channel: &mut Channel, arg: i32) {
      unsafe { v8__Channel__Channel__method1(channel, arg) }
    }
  }

  pub trait ChannelOverrides: AsChannel {
    fn extender(&self) -> &ChannelExtender;
    fn extender_mut(&mut self) -> &mut ChannelExtender;

    fn method1(&mut self, arg: i32) {
      ChannelDefaults::method1(self.as_channel_mut(), arg)
    }
    fn method2(&self) -> i32;
  }

  pub struct ChannelExtender {
    cxx_channel: Channel,
    extender_offset: util::FieldOffset<Self>,
    rust_vtable: util::RustVTable<&'static dyn ChannelOverrides>,
  }

  impl ChannelExtender {
    fn construct_cxx_channel() -> Channel {
      unsafe {
        let mut buf = std::mem::MaybeUninit::<Channel>::uninit();
        v8__Channel__EXTENDER__CTOR(&mut buf);
        buf.assume_init()
      }
    }

    fn get_extender_offset<T>() -> util::FieldOffset<Self>
    where
      T: ChannelOverrides,
    {
      let buf = std::mem::MaybeUninit::<T>::uninit();
      let embedder_ptr: *const T = buf.as_ptr();
      let self_ptr: *const Self = unsafe { (*embedder_ptr).extender() };
      util::FieldOffset::from_ptrs(embedder_ptr, self_ptr)
    }

    fn get_rust_vtable<T>() -> util::RustVTable<&'static dyn ChannelOverrides>
    where
      T: ChannelOverrides,
    {
      let buf = std::mem::MaybeUninit::<T>::uninit();
      let embedder_ptr = buf.as_ptr();
      let trait_object: *const dyn ChannelOverrides = embedder_ptr;
      let (data_ptr, vtable): (*const T, util::RustVTable<_>) =
        unsafe { std::mem::transmute(trait_object) };
      assert_eq!(data_ptr, embedder_ptr);
      vtable
    }

    pub fn new<T>() -> Self
    where
      T: ChannelOverrides,
    {
      Self {
        cxx_channel: Self::construct_cxx_channel(),
        extender_offset: Self::get_extender_offset::<T>(),
        rust_vtable: Self::get_rust_vtable::<T>(),
      }
    }

    fn get_channel_offset() -> util::FieldOffset<Channel> {
      let buf = std::mem::MaybeUninit::<Self>::uninit();
      util::FieldOffset::from_ptrs(buf.as_ptr(), unsafe {
        &(*buf.as_ptr()).cxx_channel
      })
    }

    pub unsafe fn dispatch(channel: &Channel) -> &dyn ChannelOverrides {
      let this = Self::get_channel_offset().to_embedder::<Self>(channel);
      let embedder = this.extender_offset.to_embedder::<util::Opaque>(this);
      std::mem::transmute((embedder, this.rust_vtable))
    }

    pub unsafe fn dispatch_mut(channel: &mut Channel) -> &mut dyn ChannelOverrides {
      let this = Self::get_channel_offset().to_embedder_mut::<Self>(channel);
      let vtable = this.rust_vtable;
      let embedder = this.extender_offset.to_embedder_mut::<util::Opaque>(this);
      std::mem::transmute((embedder, vtable))
    }
  }
}

mod util {
  use std::marker::PhantomData;
  use std::mem::size_of;

  pub type Opaque = [usize; 0];

  #[repr(transparent)]
  #[derive(Copy, Clone, Debug)]
  pub struct RustVTable<DynT>(pub *const Opaque, pub PhantomData<DynT>);

  #[repr(transparent)]
  #[derive(Debug)]
  pub struct FieldOffset<F>(usize, PhantomData<F>);

  unsafe impl<F> Send for FieldOffset<F> where F: Send {}
  unsafe impl<F> Sync for FieldOffset<F> where F: Sync {}
  impl<F> Copy for FieldOffset<F> {}

  impl<F> Clone for FieldOffset<F> {
    fn clone(&self) -> Self {
      Self(self.0, self.1)
    }
  }

  impl<F> FieldOffset<F> {
    pub fn from_ptrs<E>(embedder_ptr: *const E, field_ptr: *const F) -> Self {
      let embedder_addr = embedder_ptr as usize;
      let field_addr = field_ptr as usize;
      assert!(field_addr >= embedder_addr);
      assert!(
        (field_addr + size_of::<F>()) <= (embedder_addr + size_of::<E>())
      );
      Self(embedder_addr - field_addr, PhantomData)
    }

    pub unsafe fn to_embedder<E>(self, field: &F) -> &E {
      (((field as *const _ as usize) - self.0) as *const E)
        .as_ref()
        .unwrap()
    }

    pub unsafe fn to_embedder_mut<E>(self, field: &mut F) -> &mut E {
      (((field as *mut _ as usize) - self.0) as *mut E)
        .as_mut()
        .unwrap()
    }
  }
}

mod example {
  use super::channel::*;

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
  use channel::*;
  use example::*;
  let mut ex = Example::new();
  let chan = ex.as_channel_mut();
  chan.method1(3);
  println!("{}", chan.method2());
}
