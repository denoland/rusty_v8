use crate::cxx_util::FieldOffset;
use crate::cxx_util::Opaque;
use crate::cxx_util::RustVTable;

extern "C" {
  // Call a method/destructor; virtual methods use C++ dynamic dispatch.
  fn v8_inspector__Channel__DTOR(this: &mut Channel) -> ();
  fn v8_inspector__Channel__method1(this: &mut Channel, arg: i32) -> ();
  fn v8_inspector__Channel__method2(this: &Channel) -> i32;

  // Call a method of a specific class implementation, bypassing dynamic
  // dispatch. C++ equivalent: `my_channel.Channel::a()`.
  fn v8_inspector__Channel__Channel__method1(
    this: &mut Channel,
    arg: i32,
  ) -> ();

  // Constructs a special class derived from Channel that forwards all
  // virtual method invocations to rust. It is assumed that this subclass
  // has the same size and memory layout as the class it's deriving from.
  fn v8_inspector__Channel__EXTENDER__CTOR(
    buf: &mut std::mem::MaybeUninit<Channel>,
  ) -> ();
}

#[allow(non_snake_case)]
#[no_mangle]
pub unsafe extern "C" fn v8_inspector__Channel__EXTENDER__method1(
  this: &mut Channel,
  arg: i32,
) {
  ChannelExtender::dispatch_mut(this).method1(arg)
}

#[allow(non_snake_case)]
#[no_mangle]
pub unsafe extern "C" fn v8_inspector__Channel__EXTENDER__method2(
  this: &Channel,
) -> i32 {
  ChannelExtender::dispatch(this).method2()
}

#[repr(C)]
pub struct Channel {
  _cxx_vtable: *const Opaque,
}

impl Channel {
  pub fn method1(&mut self, arg: i32) {
    unsafe { v8_inspector__Channel__method1(self, arg) }
  }
  pub fn method2(&self) -> i32 {
    unsafe { v8_inspector__Channel__method2(self) }
  }
}

impl Drop for Channel {
  fn drop(&mut self) {
    unsafe { v8_inspector__Channel__DTOR(self) }
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
    unsafe { v8_inspector__Channel__Channel__method1(channel, arg) }
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
  extender_offset: FieldOffset<Self>,
  rust_vtable: RustVTable<&'static dyn ChannelOverrides>,
}

impl ChannelExtender {
  fn construct_cxx_channel() -> Channel {
    unsafe {
      let mut buf = std::mem::MaybeUninit::<Channel>::uninit();
      v8_inspector__Channel__EXTENDER__CTOR(&mut buf);
      buf.assume_init()
    }
  }

  fn get_extender_offset<T>() -> FieldOffset<Self>
  where
    T: ChannelOverrides,
  {
    let buf = std::mem::MaybeUninit::<T>::uninit();
    let embedder_ptr: *const T = buf.as_ptr();
    let self_ptr: *const Self = unsafe { (*embedder_ptr).extender() };
    FieldOffset::from_ptrs(embedder_ptr, self_ptr)
  }

  fn get_rust_vtable<T>() -> RustVTable<&'static dyn ChannelOverrides>
  where
    T: ChannelOverrides,
  {
    let buf = std::mem::MaybeUninit::<T>::uninit();
    let embedder_ptr = buf.as_ptr();
    let trait_object: *const dyn ChannelOverrides = embedder_ptr;
    let (data_ptr, vtable): (*const T, RustVTable<_>) =
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

  fn get_channel_offset() -> FieldOffset<Channel> {
    let buf = std::mem::MaybeUninit::<Self>::uninit();
    FieldOffset::from_ptrs(buf.as_ptr(), unsafe {
      &(*buf.as_ptr()).cxx_channel
    })
  }

  pub unsafe fn dispatch(channel: &Channel) -> &dyn ChannelOverrides {
    let this = Self::get_channel_offset().to_embedder::<Self>(channel);
    let embedder = this.extender_offset.to_embedder::<Opaque>(this);
    std::mem::transmute((embedder, this.rust_vtable))
  }

  pub unsafe fn dispatch_mut(
    channel: &mut Channel,
  ) -> &mut dyn ChannelOverrides {
    let this = Self::get_channel_offset().to_embedder_mut::<Self>(channel);
    let vtable = this.rust_vtable;
    let embedder = this.extender_offset.to_embedder_mut::<Opaque>(this);
    std::mem::transmute((embedder, vtable))
  }
}
