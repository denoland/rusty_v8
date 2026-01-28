// Copyright 2024 the Deno authors. All rights reserved. MIT license.

use crate::support::CxxVTable;
use crate::support::Opaque;
use std::cell::UnsafeCell;
use std::mem::MaybeUninit;
use std::pin::Pin;

unsafe extern "C" {
  fn crdtp__FrontendChannel__BASE__CONSTRUCT(
    buf: *mut MaybeUninit<RawFrontendChannel>,
  );
  fn crdtp__FrontendChannel__BASE__SIZE() -> usize;

  fn crdtp__Serializable__DELETE(this: *mut RawSerializable);
  fn crdtp__Serializable__getSerializedSize(this: *const RawSerializable)
    -> usize;
  fn crdtp__Serializable__getSerializedBytes(
    this: *const RawSerializable,
    out: *mut u8,
    len: usize,
  );

  fn crdtp__Dispatchable__new(data: *const u8, len: usize) -> *mut Dispatchable;
  fn crdtp__Dispatchable__DELETE(this: *mut Dispatchable);
  fn crdtp__Dispatchable__ok(this: *const Dispatchable) -> bool;
  fn crdtp__Dispatchable__callId(this: *const Dispatchable) -> i32;
  fn crdtp__Dispatchable__hasCallId(this: *const Dispatchable) -> bool;
  fn crdtp__Dispatchable__methodLen(this: *const Dispatchable) -> usize;
  fn crdtp__Dispatchable__methodCopy(this: *const Dispatchable, out: *mut u8);
  fn crdtp__Dispatchable__sessionIdLen(this: *const Dispatchable) -> usize;
  fn crdtp__Dispatchable__sessionIdCopy(
    this: *const Dispatchable,
    out: *mut u8,
  );
  fn crdtp__Dispatchable__paramsLen(this: *const Dispatchable) -> usize;
  fn crdtp__Dispatchable__paramsCopy(this: *const Dispatchable, out: *mut u8);

  fn crdtp__DispatchResponse__Success() -> *mut DispatchResponseWrapper;
  fn crdtp__DispatchResponse__FallThrough() -> *mut DispatchResponseWrapper;
  fn crdtp__DispatchResponse__ParseError(
    msg: *const u8,
    len: usize,
  ) -> *mut DispatchResponseWrapper;
  fn crdtp__DispatchResponse__InvalidRequest(
    msg: *const u8,
    len: usize,
  ) -> *mut DispatchResponseWrapper;
  fn crdtp__DispatchResponse__MethodNotFound(
    msg: *const u8,
    len: usize,
  ) -> *mut DispatchResponseWrapper;
  fn crdtp__DispatchResponse__InvalidParams(
    msg: *const u8,
    len: usize,
  ) -> *mut DispatchResponseWrapper;
  fn crdtp__DispatchResponse__ServerError(
    msg: *const u8,
    len: usize,
  ) -> *mut DispatchResponseWrapper;
  fn crdtp__DispatchResponse__DELETE(this: *mut DispatchResponseWrapper);
  fn crdtp__DispatchResponse__isSuccess(
    this: *const DispatchResponseWrapper,
  ) -> bool;
  fn crdtp__DispatchResponse__isError(
    this: *const DispatchResponseWrapper,
  ) -> bool;
  fn crdtp__DispatchResponse__isFallThrough(
    this: *const DispatchResponseWrapper,
  ) -> bool;
  fn crdtp__DispatchResponse__code(this: *const DispatchResponseWrapper)
    -> i32;
  fn crdtp__DispatchResponse__messageLen(
    this: *const DispatchResponseWrapper,
  ) -> usize;
  fn crdtp__DispatchResponse__messageCopy(
    this: *const DispatchResponseWrapper,
    out: *mut u8,
  );

  fn crdtp__UberDispatcher__new(
    channel: *mut RawFrontendChannel,
  ) -> *mut UberDispatcher;
  fn crdtp__UberDispatcher__DELETE(this: *mut UberDispatcher);
  fn crdtp__UberDispatcher__channel(
    this: *mut UberDispatcher,
  ) -> *mut RawFrontendChannel;
  fn crdtp__UberDispatcher__Dispatch(
    this: *mut UberDispatcher,
    dispatchable: *const Dispatchable,
  ) -> *mut DispatchResultWrapper;

  fn crdtp__DispatchResult__DELETE(this: *mut DispatchResultWrapper);
  fn crdtp__DispatchResult__MethodFound(
    this: *const DispatchResultWrapper,
  ) -> bool;
  fn crdtp__DispatchResult__Run(this: *mut DispatchResultWrapper);

  fn crdtp__vec_u8__new() -> *mut CppVecU8;
  fn crdtp__vec_u8__DELETE(this: *mut CppVecU8);
  fn crdtp__vec_u8__size(this: *const CppVecU8) -> usize;
  fn crdtp__vec_u8__data(this: *const CppVecU8) -> *const u8;
  fn crdtp__vec_u8__copy(this: *const CppVecU8, out: *mut u8);

  fn crdtp__json__ConvertJSONToCBOR(
    json_data: *const u8,
    json_len: usize,
    cbor_out: *mut CppVecU8,
  ) -> bool;
  fn crdtp__json__ConvertCBORToJSON(
    cbor_data: *const u8,
    cbor_len: usize,
    json_out: *mut CppVecU8,
  ) -> bool;

  fn crdtp__CreateErrorResponse(
    call_id: i32,
    response: *mut DispatchResponseWrapper,
  ) -> *mut RawSerializable;
  fn crdtp__CreateResponse(
    call_id: i32,
    params: *mut RawSerializable,
  ) -> *mut RawSerializable;
  fn crdtp__CreateNotification(
    method: *const u8,
    params: *mut RawSerializable,
  ) -> *mut RawSerializable;
  fn crdtp__CreateErrorNotification(
    response: *mut DispatchResponseWrapper,
  ) -> *mut RawSerializable;
}

#[repr(C)]
pub struct Dispatchable(Opaque);

#[repr(C)]
struct DispatchResponseWrapper(Opaque);

#[repr(C)]
pub struct UberDispatcher(Opaque);

#[repr(C)]
struct DispatchResultWrapper(Opaque);

#[repr(C)]
struct CppVecU8(Opaque);

#[repr(C)]
struct RawSerializable(Opaque);

pub struct Serializable {
  ptr: *mut RawSerializable,
}

impl Serializable {
  pub fn to_bytes(&self) -> Vec<u8> {
    unsafe {
      let len = crdtp__Serializable__getSerializedSize(self.ptr);
      let mut bytes = vec![0u8; len];
      crdtp__Serializable__getSerializedBytes(self.ptr, bytes.as_mut_ptr(), len);
      bytes
    }
  }

  pub fn into_raw(self) -> *mut RawSerializable {
    let ptr = self.ptr;
    std::mem::forget(self);
    ptr
  }
}

impl Drop for Serializable {
  fn drop(&mut self) {
    unsafe {
      crdtp__Serializable__DELETE(self.ptr);
    }
  }
}

impl Dispatchable {
  pub fn new(cbor_data: &[u8]) -> Box<Self> {
    unsafe {
      let ptr = crdtp__Dispatchable__new(cbor_data.as_ptr(), cbor_data.len());
      Box::from_raw(ptr)
    }
  }

  pub fn ok(&self) -> bool {
    unsafe { crdtp__Dispatchable__ok(self) }
  }

  pub fn call_id(&self) -> i32 {
    unsafe { crdtp__Dispatchable__callId(self) }
  }

  pub fn has_call_id(&self) -> bool {
    unsafe { crdtp__Dispatchable__hasCallId(self) }
  }

  pub fn method(&self) -> Vec<u8> {
    unsafe {
      let len = crdtp__Dispatchable__methodLen(self);
      let mut buf = vec![0u8; len];
      crdtp__Dispatchable__methodCopy(self, buf.as_mut_ptr());
      buf
    }
  }

  pub fn method_str(&self) -> String {
    String::from_utf8_lossy(&self.method()).into_owned()
  }

  pub fn session_id(&self) -> Vec<u8> {
    unsafe {
      let len = crdtp__Dispatchable__sessionIdLen(self);
      let mut buf = vec![0u8; len];
      crdtp__Dispatchable__sessionIdCopy(self, buf.as_mut_ptr());
      buf
    }
  }

  pub fn params(&self) -> Vec<u8> {
    unsafe {
      let len = crdtp__Dispatchable__paramsLen(self);
      let mut buf = vec![0u8; len];
      crdtp__Dispatchable__paramsCopy(self, buf.as_mut_ptr());
      buf
    }
  }
}

impl Drop for Dispatchable {
  fn drop(&mut self) {
    unsafe {
      crdtp__Dispatchable__DELETE(self);
    }
  }
}

pub struct DispatchResponse {
  ptr: *mut DispatchResponseWrapper,
}

impl DispatchResponse {
  pub fn success() -> Self {
    unsafe {
      Self {
        ptr: crdtp__DispatchResponse__Success(),
      }
    }
  }

  pub fn fall_through() -> Self {
    unsafe {
      Self {
        ptr: crdtp__DispatchResponse__FallThrough(),
      }
    }
  }

  pub fn parse_error(message: &str) -> Self {
    unsafe {
      Self {
        ptr: crdtp__DispatchResponse__ParseError(
          message.as_ptr(),
          message.len(),
        ),
      }
    }
  }

  pub fn invalid_request(message: &str) -> Self {
    unsafe {
      Self {
        ptr: crdtp__DispatchResponse__InvalidRequest(
          message.as_ptr(),
          message.len(),
        ),
      }
    }
  }

  pub fn method_not_found(message: &str) -> Self {
    unsafe {
      Self {
        ptr: crdtp__DispatchResponse__MethodNotFound(
          message.as_ptr(),
          message.len(),
        ),
      }
    }
  }

  pub fn invalid_params(message: &str) -> Self {
    unsafe {
      Self {
        ptr: crdtp__DispatchResponse__InvalidParams(
          message.as_ptr(),
          message.len(),
        ),
      }
    }
  }

  pub fn server_error(message: &str) -> Self {
    unsafe {
      Self {
        ptr: crdtp__DispatchResponse__ServerError(
          message.as_ptr(),
          message.len(),
        ),
      }
    }
  }

  pub fn is_success(&self) -> bool {
    unsafe { crdtp__DispatchResponse__isSuccess(self.ptr) }
  }

  pub fn is_error(&self) -> bool {
    unsafe { crdtp__DispatchResponse__isError(self.ptr) }
  }

  /// Returns true if this is a fall-through response.
  pub fn is_fall_through(&self) -> bool {
    unsafe { crdtp__DispatchResponse__isFallThrough(self.ptr) }
  }

  /// Get the error code.
  pub fn code(&self) -> i32 {
    unsafe { crdtp__DispatchResponse__code(self.ptr) }
  }

  /// Get the error message.
  pub fn message(&self) -> String {
    unsafe {
      let len = crdtp__DispatchResponse__messageLen(self.ptr);
      let mut buf = vec![0u8; len];
      crdtp__DispatchResponse__messageCopy(self.ptr, buf.as_mut_ptr());
      String::from_utf8_lossy(&buf).into_owned()
    }
  }

  pub fn into_raw(self) -> *mut DispatchResponseWrapper {
    let ptr = self.ptr;
    std::mem::forget(self);
    ptr
  }
}

impl Drop for DispatchResponse {
  fn drop(&mut self) {
    unsafe {
      crdtp__DispatchResponse__DELETE(self.ptr);
    }
  }
}

/// Trait for sending protocol responses and notifications to clients.
pub trait FrontendChannelImpl {
  /// Send a response to a protocol request.
  fn send_protocol_response(&mut self, call_id: i32, message: Serializable);
  /// Send a notification (no call_id).
  fn send_protocol_notification(&mut self, message: Serializable);
  /// Indicate that the message should be handled by another layer.
  fn fall_through(&mut self, call_id: i32, method: &[u8], message: &[u8]);
  /// Flush any queued notifications.
  fn flush_protocol_notifications(&mut self);
}

#[repr(C)]
struct RawFrontendChannel {
  _cxx_vtable: CxxVTable,
}

/// Wraps a Rust `FrontendChannelImpl` for use with the C++ dispatcher.
pub struct FrontendChannel {
  raw: UnsafeCell<RawFrontendChannel>,
  imp: Box<dyn FrontendChannelImpl>,
}

impl FrontendChannel {
  /// Create a new FrontendChannel wrapping the given implementation.
  pub fn new(imp: Box<dyn FrontendChannelImpl>) -> Pin<Box<Self>> {
    let mut channel = Box::new(Self {
      raw: UnsafeCell::new(unsafe { MaybeUninit::zeroed().assume_init() }),
      imp,
    });
    unsafe {
      crdtp__FrontendChannel__BASE__CONSTRUCT(
        channel.raw.get() as *mut _ as *mut MaybeUninit<RawFrontendChannel>,
      );
    }
    Box::into_pin(channel)
  }

  fn raw_ptr(&self) -> *mut RawFrontendChannel {
    self.raw.get()
  }

  unsafe fn from_raw<'a>(ptr: *mut RawFrontendChannel) -> &'a mut Self {
    let channel_ptr = ptr as *mut u8;
    let offset = std::mem::offset_of!(FrontendChannel, raw);
    let self_ptr = channel_ptr.sub(offset) as *mut Self;
    &mut *self_ptr
  }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn crdtp__FrontendChannel__BASE__sendProtocolResponse(
  this: *mut RawFrontendChannel,
  call_id: i32,
  message: *mut RawSerializable,
) {
  unsafe {
    let channel = FrontendChannel::from_raw(this);
    let msg = Serializable { ptr: message };
    channel.imp.send_protocol_response(call_id, msg);
  }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn crdtp__FrontendChannel__BASE__sendProtocolNotification(
  this: *mut RawFrontendChannel,
  message: *mut RawSerializable,
) {
  unsafe {
    let channel = FrontendChannel::from_raw(this);
    let msg = Serializable { ptr: message };
    channel.imp.send_protocol_notification(msg);
  }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn crdtp__FrontendChannel__BASE__fallThrough(
  this: *mut RawFrontendChannel,
  call_id: i32,
  method_data: *const u8,
  method_len: usize,
  message_data: *const u8,
  message_len: usize,
) {
  unsafe {
    let channel = FrontendChannel::from_raw(this);
    let method = std::slice::from_raw_parts(method_data, method_len);
    let message = std::slice::from_raw_parts(message_data, message_len);
    channel.imp.fall_through(call_id, method, message);
  }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn crdtp__FrontendChannel__BASE__flushProtocolNotifications(
  this: *mut RawFrontendChannel,
) {
  unsafe {
    let channel = FrontendChannel::from_raw(this);
    channel.imp.flush_protocol_notifications();
  }
}

/// Result of dispatching a protocol message through UberDispatcher.
pub struct DispatchResult {
  ptr: *mut DispatchResultWrapper,
}

impl DispatchResult {
  /// Returns true if a handler was found for the method.
  pub fn method_found(&self) -> bool {
    unsafe { crdtp__DispatchResult__MethodFound(self.ptr) }
  }

  /// Run the dispatched handler.
  pub fn run(self) {
    unsafe {
      crdtp__DispatchResult__Run(self.ptr);
    }
    std::mem::forget(self);
  }
}

impl Drop for DispatchResult {
  fn drop(&mut self) {
    unsafe {
      crdtp__DispatchResult__DELETE(self.ptr);
    }
  }
}

impl UberDispatcher {
  /// Create a new UberDispatcher with the given frontend channel.
  pub fn new(channel: &Pin<Box<FrontendChannel>>) -> Box<Self> {
    unsafe {
      let ptr = crdtp__UberDispatcher__new(channel.raw_ptr());
      Box::from_raw(ptr)
    }
  }

  /// Get the frontend channel.
  pub fn channel(&mut self) -> *mut RawFrontendChannel {
    unsafe { crdtp__UberDispatcher__channel(self) }
  }

  /// Dispatch a protocol message.
  pub fn dispatch(&mut self, dispatchable: &Dispatchable) -> DispatchResult {
    unsafe {
      let ptr = crdtp__UberDispatcher__Dispatch(self, dispatchable);
      DispatchResult { ptr }
    }
  }

  /// Get the raw pointer for passing to C++ Wire functions.
  pub fn as_raw(&mut self) -> *mut Self {
    self as *mut Self
  }
}

impl Drop for UberDispatcher {
  fn drop(&mut self) {
    unsafe {
      crdtp__UberDispatcher__DELETE(self);
    }
  }
}

/// Convert JSON bytes to CBOR bytes.
pub fn json_to_cbor(json: &[u8]) -> Option<Vec<u8>> {
  unsafe {
    let vec = crdtp__vec_u8__new();
    let ok = crdtp__json__ConvertJSONToCBOR(json.as_ptr(), json.len(), vec);
    if ok {
      let len = crdtp__vec_u8__size(vec);
      let mut result = vec![0u8; len];
      crdtp__vec_u8__copy(vec, result.as_mut_ptr());
      crdtp__vec_u8__DELETE(vec);
      Some(result)
    } else {
      crdtp__vec_u8__DELETE(vec);
      None
    }
  }
}

/// Convert CBOR bytes to JSON bytes.
pub fn cbor_to_json(cbor: &[u8]) -> Option<Vec<u8>> {
  unsafe {
    let vec = crdtp__vec_u8__new();
    let ok = crdtp__json__ConvertCBORToJSON(cbor.as_ptr(), cbor.len(), vec);
    if ok {
      let len = crdtp__vec_u8__size(vec);
      let mut result = vec![0u8; len];
      crdtp__vec_u8__copy(vec, result.as_mut_ptr());
      crdtp__vec_u8__DELETE(vec);
      Some(result)
    } else {
      crdtp__vec_u8__DELETE(vec);
      None
    }
  }
}

/// Create an error response message.
pub fn create_error_response(call_id: i32, response: DispatchResponse) -> Serializable {
  unsafe {
    let ptr = crdtp__CreateErrorResponse(call_id, response.into_raw());
    Serializable { ptr }
  }
}

/// Create an error notification message.
pub fn create_error_notification(response: DispatchResponse) -> Serializable {
  unsafe {
    let ptr = crdtp__CreateErrorNotification(response.into_raw());
    Serializable { ptr }
  }
}
