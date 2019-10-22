use crate::support::int;
use crate::support::CxxVTable;
use crate::support::FieldOffset;
use crate::support::Opaque;
use crate::support::RustVTable;
use crate::support::UniquePtr;
use crate::v8::StringBuffer;

// class Channel {
//  public:
//   virtual ~Channel() = default;
//   virtual void sendResponse(int callId,
//                             std::unique_ptr<StringBuffer> message) = 0;
//   virtual void sendNotification(std::unique_ptr<StringBuffer> message) = 0;
//   virtual void flushProtocolNotifications() = 0;
// };

extern "C" {
  fn v8_inspector__V8Inspector__Channel__BASE__CONSTRUCT(
    buf: &mut std::mem::MaybeUninit<Channel>,
  ) -> ();

  fn v8_inspector__V8Inspector__Channel__sendResponse(
    this: &mut Channel,
    callId: int,
    message: UniquePtr<StringBuffer>,
  ) -> ();
  fn v8_inspector__V8Inspector__Channel__sendNotification(
    this: &mut Channel,
    message: UniquePtr<StringBuffer>,
  ) -> ();
  fn v8_inspector__V8Inspector__Channel__flushProtocolNotifications(
    this: &mut Channel,
  ) -> ();
}

#[no_mangle]
pub unsafe extern "C" fn v8_inspector__V8Inspector__Channel__BASE__sendResponse(
  this: &mut Channel,
  callId: int,
  message: UniquePtr<StringBuffer>,
) -> () {
  ChannelBase::dispatch_mut(this).sendResponse(callId, message)
}

#[no_mangle]
pub unsafe extern "C" fn v8_inspector__V8Inspector__Channel__BASE__sendNotification(
  this: &mut Channel,
  message: UniquePtr<StringBuffer>,
) -> () {
  ChannelBase::dispatch_mut(this).sendNotification(message)
}

#[no_mangle]
pub unsafe extern "C" fn v8_inspector__V8Inspector__Channel__BASE__flushProtocolNotifications(
  this: &mut Channel,
) -> () {
  ChannelBase::dispatch_mut(this).flushProtocolNotifications()
}

#[repr(C)]
pub struct Channel {
  _cxx_vtable: CxxVTable,
}

impl Channel {
  pub fn sendResponse(
    &mut self,
    callId: int,
    message: UniquePtr<StringBuffer>,
  ) -> () {
    unsafe {
      v8_inspector__V8Inspector__Channel__sendResponse(self, callId, message)
    }
  }
  pub fn sendNotification(&mut self, message: UniquePtr<StringBuffer>) -> () {
    unsafe {
      v8_inspector__V8Inspector__Channel__sendNotification(self, message)
    }
  }
  pub fn flushProtocolNotifications(&mut self) -> () {
    unsafe {
      v8_inspector__V8Inspector__Channel__flushProtocolNotifications(self)
    }
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
  T: ChannelImpl,
{
  fn as_channel(&self) -> &Channel {
    &self.base().cxx_base
  }
  fn as_channel_mut(&mut self) -> &mut Channel {
    &mut self.base_mut().cxx_base
  }
}

pub trait ChannelImpl: AsChannel {
  fn base(&self) -> &ChannelBase;
  fn base_mut(&mut self) -> &mut ChannelBase;

  fn sendResponse(
    &mut self,
    callId: int,
    message: UniquePtr<StringBuffer>,
  ) -> ();
  fn sendNotification(&mut self, message: UniquePtr<StringBuffer>) -> ();
  fn flushProtocolNotifications(&mut self) -> ();
}

pub struct ChannelBase {
  cxx_base: Channel,
  offset_within_embedder: FieldOffset<Self>,
  rust_vtable: RustVTable<&'static dyn ChannelImpl>,
}

impl ChannelBase {
  fn construct_cxx_base() -> Channel {
    unsafe {
      let mut buf = std::mem::MaybeUninit::<Channel>::uninit();
      v8_inspector__V8Inspector__Channel__BASE__CONSTRUCT(&mut buf);
      buf.assume_init()
    }
  }

  fn get_cxx_base_offset() -> FieldOffset<Channel> {
    let buf = std::mem::MaybeUninit::<Self>::uninit();
    FieldOffset::from_ptrs(buf.as_ptr(), unsafe { &(*buf.as_ptr()).cxx_base })
  }

  fn get_offset_within_embedder<T>() -> FieldOffset<Self>
  where
    T: ChannelImpl,
  {
    let buf = std::mem::MaybeUninit::<T>::uninit();
    let embedder_ptr: *const T = buf.as_ptr();
    let self_ptr: *const Self = unsafe { (*embedder_ptr).base() };
    FieldOffset::from_ptrs(embedder_ptr, self_ptr)
  }

  fn get_rust_vtable<T>() -> RustVTable<&'static dyn ChannelImpl>
  where
    T: ChannelImpl,
  {
    let buf = std::mem::MaybeUninit::<T>::uninit();
    let embedder_ptr = buf.as_ptr();
    let trait_object: *const dyn ChannelImpl = embedder_ptr;
    let (data_ptr, vtable): (*const T, RustVTable<_>) =
      unsafe { std::mem::transmute(trait_object) };
    assert_eq!(data_ptr, embedder_ptr);
    vtable
  }

  pub fn new<T>() -> Self
  where
    T: ChannelImpl,
  {
    Self {
      cxx_base: Self::construct_cxx_base(),
      offset_within_embedder: Self::get_offset_within_embedder::<T>(),
      rust_vtable: Self::get_rust_vtable::<T>(),
    }
  }

  pub unsafe fn dispatch(channel: &Channel) -> &dyn ChannelImpl {
    let this = Self::get_cxx_base_offset().to_embedder::<Self>(channel);
    let embedder = this.offset_within_embedder.to_embedder::<Opaque>(this);
    std::mem::transmute((embedder, this.rust_vtable))
  }

  pub unsafe fn dispatch_mut(channel: &mut Channel) -> &mut dyn ChannelImpl {
    let this = Self::get_cxx_base_offset().to_embedder_mut::<Self>(channel);
    let vtable = this.rust_vtable;
    let embedder = this.offset_within_embedder.to_embedder_mut::<Opaque>(this);
    std::mem::transmute((embedder, vtable))
  }
}
