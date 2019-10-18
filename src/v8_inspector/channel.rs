use std::os::raw::c_int;

use crate::cxx_util::CxxVTable;
use crate::cxx_util::FieldOffset;
use crate::cxx_util::Opaque;
use crate::cxx_util::RustVTable;
use crate::cxx_util::UniquePtr;

use super::StringBuffer;

// class Channel {
//  public:
//   virtual ~Channel() = default;
//   virtual void sendResponse(int callId,
//                             std::unique_ptr<StringBuffer> message) = 0;
//   virtual void sendNotification(std::unique_ptr<StringBuffer> message) = 0;
//   virtual void flushProtocolNotifications() = 0;
// };

extern "C" {
  fn v8_inspector__Channel__EXTENDER__CTOR(
    buf: &mut std::mem::MaybeUninit<Channel>,
  ) -> ();
  fn v8_inspector__Channel__DTOR(this: &mut Channel) -> ();

  fn v8_inspector__Channel__sendResponse(
    this: &mut Channel,
    callId: c_int,
    message: UniquePtr<StringBuffer>,
  ) -> ();
  fn v8_inspector__Channel__sendNotification(
    this: &mut Channel,
    message: UniquePtr<StringBuffer>,
  ) -> ();
  fn v8_inspector__Channel__flushProtocolNotifications(
    this: &mut Channel,
  ) -> ();
}

#[no_mangle]
pub unsafe extern "C" fn v8_inspector__Channel__EXTENDER__sendResponse(
  this: &mut Channel,
  callId: c_int,
  message: UniquePtr<StringBuffer>,
) -> () {
  ChannelExtender::dispatch_mut(this).sendResponse(callId, message)
}

#[no_mangle]
pub unsafe extern "C" fn v8_inspector__Channel__EXTENDER__sendNotification(
  this: &mut Channel,
  message: UniquePtr<StringBuffer>,
) -> () {
  ChannelExtender::dispatch_mut(this).sendNotification(message)
}

#[no_mangle]
pub unsafe extern "C" fn v8_inspector__Channel__EXTENDER__flushProtocolNotifications(
  this: &mut Channel,
) -> () {
  ChannelExtender::dispatch_mut(this).flushProtocolNotifications()
}

#[repr(C)]
pub struct Channel {
  _cxx_vtable: CxxVTable,
}

impl Channel {
  pub fn sendResponse(
    &mut self,
    callId: c_int,
    message: UniquePtr<StringBuffer>,
  ) -> () {
    unsafe { v8_inspector__Channel__sendResponse(self, callId, message) }
  }
  pub fn sendNotification(&mut self, message: UniquePtr<StringBuffer>) -> () {
    unsafe { v8_inspector__Channel__sendNotification(self, message) }
  }
  pub fn flushProtocolNotifications(&mut self) -> () {
    unsafe { v8_inspector__Channel__flushProtocolNotifications(self) }
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

pub trait ChannelOverrides: AsChannel {
  fn extender(&self) -> &ChannelExtender;
  fn extender_mut(&mut self) -> &mut ChannelExtender;

  fn sendResponse(
    &mut self,
    callId: i32,
    message: UniquePtr<StringBuffer>,
  ) -> ();
  fn sendNotification(&mut self, message: UniquePtr<StringBuffer>) -> ();
  fn flushProtocolNotifications(&mut self) -> ();
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
