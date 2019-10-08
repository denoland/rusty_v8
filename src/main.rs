mod channel {
  use super::util;

  extern "C" {
    // Call a method/destructor; virtual methods use C++ dynamic dispatch.
    fn Channel__DTOR(this: &mut Channel) -> ();
    fn Channel__method1(this: &mut Channel) -> ();
    fn Channel__method2(this: &Channel) -> i32;

    // Call a method of a specific class implementation, bypassing dynamic
    // dispatch. C++ equivalent: `my_channel.Channel::a()`.
    fn Channel__Channel__method1(this: &mut Channel) -> ();

    // Constructs a special class derived from Channel that forwards all
    // virtual method invocations to rust. It is assumed that this subclass
    // has the same size and memory layout as the class it's deriving from.
    fn Channel__OVERRIDE__CTOR(this: &mut std::mem::MaybeUninit<Channel>)
      -> ();
  }

  #[repr(C)]
  pub struct Channel {
    _cxx_vtable: *const [usize; 0],
  }

  #[allow(dead_code)]
  impl Channel {
    pub fn method1(&mut self) {
      unsafe { Channel__method1(self) }
    }
    pub fn method2(&self) -> i32 {
      unsafe { Channel__method2(self) }
    }
  }

  impl Drop for Channel {
    fn drop(&mut self) {
      unsafe { Channel__DTOR(self) }
    }
  }

  pub struct ChannelDefaults;
  impl ChannelDefaults {
    pub fn method1(this: &mut Channel) {
      unsafe { Channel__Channel__method1(this) }
    }
  }

  pub trait ChannelOverrides: AsChannel {
    fn extender(&self) -> &ChannelExtender;
    fn extender_mut(&mut self) -> &mut ChannelExtender;

    fn method1(&mut self) {
      ChannelDefaults::method1(self.extender_mut().as_mut())
    }
    fn method2(&self) -> i32;
  }

  pub trait AsChannel {
    fn as_channel(&self) -> &Channel;
    fn as_channel_mut(&mut self) -> &mut Channel;
  }

  impl AsChannel for ChannelExtender {
    fn as_channel(&self) -> &Channel {
      &self.cxx_channel
    }
    fn as_channel_mut(&mut self) -> &mut Channel {
      &mut self.cxx_channel
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

  pub struct ChannelExtender {
    cxx_channel: Channel,
    extender_offset: usize,
    rust_vtable: util::RustVTable<&'static dyn ChannelOverrides>,
  }

  #[no_mangle]
  unsafe extern "C" fn Channel__OVERRIDE__method1__DISPATCH(
    this: &mut Channel,
  ) {
    ChannelExtender::dispatch_mut(this).method1()
  }
  #[no_mangle]
  unsafe extern "C" fn Channel__OVERRIDE__method2__DISPATCH(
    this: &Channel,
  ) -> i32 {
    ChannelExtender::dispatch(this).method2()
  }

  impl ChannelExtender {
    fn construct_cxx_channel() -> Channel {
      unsafe {
        let mut buf = std::mem::MaybeUninit::<Channel>::uninit();
        Channel__OVERRIDE__CTOR(&mut buf);
        buf.assume_init()
      }
    }

    fn get_extender_offset<T>() -> usize
    where
      T: ChannelOverrides,
    {
      let buf = std::mem::MaybeUninit::<T>::uninit();
      let embedder_ptr: *const T = buf.as_ptr();
      let self_ptr: *const Self = unsafe { (*embedder_ptr).extender() };
      util::FieldOffset::from_ptrs(embedder_ptr, self_ptr).offset()
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

    fn channel_offset() -> util::FieldOffset<Self, Channel> {
      let buf = std::mem::MaybeUninit::<Self>::uninit();
      util::FieldOffset::from_ptrs(buf.as_ptr(), unsafe {
        &(*buf.as_ptr()).cxx_channel
      })
    }

    fn embedder_offset(&self) -> util::FieldOffset<util::Opaque, Self> {
      util::FieldOffset::<util::Opaque, Self>::from_offset(self.extender_offset)
    }

    unsafe fn dispatch(channel: &Channel) -> &dyn ChannelOverrides {
      let this = Self::channel_offset().to_outer(channel);
      let embedder = this.embedder_offset().to_outer(this);
      std::mem::transmute((embedder, this.rust_vtable))
    }

    unsafe fn dispatch_mut(channel: &mut Channel) -> &mut dyn ChannelOverrides {
      let this = Self::channel_offset().to_outer_mut(channel);
      let vtable = this.rust_vtable;
      let embedder = this.embedder_offset().to_outer_mut(this);
      std::mem::transmute((embedder, vtable))
    }
  }

  impl std::convert::AsRef<Channel> for ChannelExtender {
    fn as_ref(&self) -> &Channel {
      &self.cxx_channel
    }
  }

  impl std::convert::AsMut<Channel> for ChannelExtender {
    fn as_mut(&mut self) -> &mut Channel {
      &mut self.cxx_channel
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

  #[derive(Copy, Clone, Debug)]
  #[repr(transparent)]
  pub struct FieldOffset<O, I>(isize, PhantomData<(O, I)>);

  impl<O, I> FieldOffset<O, I> {
    pub fn from_ptrs(o_ptr: *const O, i_ptr: *const I) -> Self {
      let o_addr = o_ptr as usize;
      let i_addr = i_ptr as usize;
      assert!(i_addr >= o_addr);
      assert!((i_addr + size_of::<I>()) <= (o_addr + size_of::<O>()));
      let offset = (o_addr - i_addr) as isize;
      assert!(offset > 0);
      Self(offset, PhantomData)
    }

    pub fn from_offset(offset: usize) -> Self {
      assert!((offset as isize) > 0);
      Self(offset as isize, PhantomData)
    }

    pub fn offset(self) -> usize {
      self.0 as usize
    }

    fn shift<PI, PO>(ptr: *const PI, delta: isize) -> *mut PO {
      (ptr as isize + delta) as *mut PO
    }
    pub unsafe fn to_outer<'a>(&self, inner: &'a I) -> &'a O {
      Self::shift::<I, O>(inner, -self.0).as_ref().unwrap()
    }
    #[allow(dead_code)]
    pub unsafe fn to_outer_mut<'a>(&self, inner: &'a mut I) -> &'a mut O {
      Self::shift::<I, O>(inner, -self.0).as_mut().unwrap()
    }
  }

  impl<O, M, I> std::ops::Add<FieldOffset<M, I>> for FieldOffset<O, M> {
    type Output = FieldOffset<O, I>;
    fn add(self, that: FieldOffset<M, I>) -> Self::Output {
      FieldOffset::<O, I>::from_offset(self.offset() + that.offset())
    }
  }
}

mod example {
  use super::channel::*;

  #[allow(dead_code)]
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
    fn method1(&mut self) {
      println!("overriden a() called");
      self.a += self.b;
    }
    fn method2(&self) -> i32 {
      println!("overriden b() called");
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
  chan.method1();
  println!("{}", chan.method2());
}
