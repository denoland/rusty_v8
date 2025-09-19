// Copyright 2019-2021 the Deno authors. All rights reserved. MIT license.

//! Bindings to the V8 Inspector API. Documentation for the V8 inspector API is
//! very sparse, so here are a few references for the next sorry soul who has to
//! dig into it.
//!
//! https://medium.com/@hyperandroid/v8-inspector-from-an-embedder-standpoint-7f9c0472e2b7
//! https://v8.dev/docs/inspector
//! https://chromedevtools.github.io/debugger-protocol-viewer/tot/
//! https://cs.chromium.org/chromium/src/v8/include/v8-inspector.h
//! https://github.com/nodejs/node/blob/v13.7.0/src/inspector_agent.cc
//! https://github.com/nodejs/node/blob/v13.7.0/src/inspector_agent.h
//! https://github.com/nodejs/node/tree/v13.7.0/src/inspector
//! https://github.com/denoland/deno/blob/v0.38.0/cli/inspector.rs

use crate::Context;
use crate::Isolate;
use crate::Local;
use crate::StackTrace;
use crate::Value;
use crate::isolate::RealIsolate;
use crate::support::CxxVTable;
use crate::support::Opaque;
use crate::support::UniquePtr;
use crate::support::UniqueRef;
use crate::support::int;
use std::cell::UnsafeCell;
use std::fmt::{self, Debug, Formatter};

unsafe extern "C" {
  fn v8_inspector__V8Inspector__Channel__BASE__CONSTRUCT(
    buf: *mut MaybeUninit<RawChannel>,
  );

  fn v8_inspector__V8Inspector__Channel__sendResponse(
    this: *mut RawChannel,
    call_id: int,
    message: UniquePtr<StringBuffer>,
  );
  fn v8_inspector__V8Inspector__Channel__sendNotification(
    this: *mut RawChannel,
    message: UniquePtr<StringBuffer>,
  );
  fn v8_inspector__V8Inspector__Channel__flushProtocolNotifications(
    this: *mut RawChannel,
  );

  fn v8_inspector__V8InspectorClient__BASE__CONSTRUCT(
    buf: *mut MaybeUninit<RawV8InspectorClient>,
  );

  fn v8_inspector__V8InspectorClient__generateUniqueId(
    this: *mut RawV8InspectorClient,
  ) -> i64;
  fn v8_inspector__V8InspectorClient__runMessageLoopOnPause(
    this: *mut RawV8InspectorClient,
    context_group_id: int,
  );
  fn v8_inspector__V8InspectorClient__quitMessageLoopOnPause(
    this: *mut RawV8InspectorClient,
  );
  fn v8_inspector__V8InspectorClient__runIfWaitingForDebugger(
    this: *mut RawV8InspectorClient,
    context_group_id: int,
  );
  fn v8_inspector__V8InspectorClient__consoleAPIMessage(
    this: *mut RawV8InspectorClient,
    context_group_id: int,
    level: int,
    message: &StringView,
    url: &StringView,
    line_number: u32,
    column_number: u32,
    stack_trace: &mut V8StackTrace,
  );

  fn v8_inspector__V8InspectorSession__DELETE(this: *mut RawV8InspectorSession);
  fn v8_inspector__V8InspectorSession__dispatchProtocolMessage(
    session: *mut RawV8InspectorSession,
    message: StringView,
  );
  fn v8_inspector__V8InspectorSession__schedulePauseOnNextStatement(
    session: *mut RawV8InspectorSession,
    break_reason: StringView,
    break_details: StringView,
  );
  fn v8_inspector__V8InspectorSession__canDispatchMethod(
    method: StringView,
  ) -> bool;

  fn v8_inspector__StringBuffer__DELETE(this: *mut StringBuffer);
  fn v8_inspector__StringBuffer__string(this: &StringBuffer) -> StringView<'_>;
  fn v8_inspector__StringBuffer__create(
    source: StringView,
  ) -> UniquePtr<StringBuffer>;

  fn v8_inspector__V8Inspector__DELETE(this: *mut RawV8Inspector);
  fn v8_inspector__V8Inspector__create(
    isolate: *mut RealIsolate,
    client: *mut RawV8InspectorClient,
  ) -> *mut RawV8Inspector;
  fn v8_inspector__V8Inspector__connect(
    inspector: *mut RawV8Inspector,
    context_group_id: int,
    channel: *mut RawChannel,
    state: StringView,
    client_trust_level: V8InspectorClientTrustLevel,
  ) -> *mut RawV8InspectorSession;
  fn v8_inspector__V8Inspector__contextCreated(
    this: *mut RawV8Inspector,
    context: *const Context,
    contextGroupId: int,
    humanReadableName: StringView,
    auxData: StringView,
  );
  fn v8_inspector__V8Inspector__contextDestroyed(
    this: *mut RawV8Inspector,
    context: *const Context,
  );
  fn v8_inspector__V8Inspector__exceptionThrown(
    this: *mut RawV8Inspector,
    context: *const Context,
    message: StringView,
    exception: *const Value,
    detailed_message: StringView,
    url: StringView,
    line_number: u32,
    column_number: u32,
    stack_trace: *mut V8StackTrace,
    script_id: int,
  ) -> u32;
  fn v8_inspector__V8Inspector__createStackTrace(
    this: *mut RawV8Inspector,
    stack_trace: *const StackTrace,
  ) -> *mut V8StackTrace;
  fn v8_inspector__V8StackTrace__DELETE(this: *mut V8StackTrace);
}

#[unsafe(no_mangle)]
unsafe extern "C" fn v8_inspector__V8Inspector__Channel__BASE__sendResponse(
  this: *mut RawChannel,
  call_id: int,
  message: UniquePtr<StringBuffer>,
) {
  unsafe {
    let channel = ChannelHeap::from_raw(this);
    channel.imp.send_response(call_id, message);
  }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn v8_inspector__V8Inspector__Channel__BASE__sendNotification(
  this: *mut RawChannel,
  message: UniquePtr<StringBuffer>,
) {
  unsafe {
    ChannelHeap::from_raw(this).imp.send_notification(message);
  }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn v8_inspector__V8Inspector__Channel__BASE__flushProtocolNotifications(
  this: *mut RawChannel,
) {
  unsafe {
    ChannelHeap::from_raw(this)
      .imp
      .flush_protocol_notifications();
  }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn v8_inspector__V8InspectorClient__BASE__generateUniqueId(
  this: *mut RawV8InspectorClient,
) -> i64 {
  unsafe {
    V8InspectorClientHeap::from_raw(this)
      .imp
      .generate_unique_id()
  }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn v8_inspector__V8InspectorClient__BASE__runMessageLoopOnPause(
  this: *mut RawV8InspectorClient,
  context_group_id: int,
) {
  unsafe {
    V8InspectorClientHeap::from_raw(this)
      .imp
      .run_message_loop_on_pause(context_group_id);
  }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn v8_inspector__V8InspectorClient__BASE__quitMessageLoopOnPause(
  this: *mut RawV8InspectorClient,
) {
  unsafe {
    V8InspectorClientHeap::from_raw(this)
      .imp
      .quit_message_loop_on_pause();
  }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn v8_inspector__V8InspectorClient__BASE__runIfWaitingForDebugger(
  this: *mut RawV8InspectorClient,
  context_group_id: int,
) {
  unsafe {
    V8InspectorClientHeap::from_raw(this)
      .imp
      .run_if_waiting_for_debugger(context_group_id);
  }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn v8_inspector__V8InspectorClient__BASE__consoleAPIMessage(
  this: *mut RawV8InspectorClient,
  context_group_id: int,
  level: int,
  message: &StringView,
  url: &StringView,
  line_number: u32,
  column_number: u32,
  stack_trace: &mut V8StackTrace,
) {
  unsafe {
    V8InspectorClientHeap::from_raw(this)
      .imp
      .console_api_message(
        context_group_id,
        level,
        message,
        url,
        line_number,
        column_number,
        stack_trace,
      );
  }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn v8_inspector__V8InspectorClient__BASE__ensureDefaultContextInGroup(
  this: *mut RawV8InspectorClient,
  context_group_id: int,
) -> *const Context {
  unsafe {
    match V8InspectorClientHeap::from_raw(this)
      .imp
      .ensure_default_context_in_group(context_group_id)
    {
      Some(h) => &*h,
      None => std::ptr::null_mut(),
    }
  }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn v8_inspector__V8InspectorClient__BASE__resourceNameToUrl(
  this: *mut RawV8InspectorClient,
  resource_name: &StringView,
) -> *mut StringBuffer {
  unsafe {
    V8InspectorClientHeap::from_raw(this)
      .imp
      .resource_name_to_url(resource_name)
      .and_then(|mut v| v.take())
      .map(|r| r.into_raw())
      .unwrap_or(std::ptr::null_mut())
  }
}

#[repr(C)]
#[derive(Debug)]
struct RawChannel {
  _cxx_vtable: CxxVTable,
}

#[repr(C)]
pub struct Channel {
  heap: Pin<Box<ChannelHeap>>,
}

impl std::fmt::Debug for Channel {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("Channel").finish()
  }
}

#[repr(C)]
struct ChannelHeap {
  raw: UnsafeCell<RawChannel>,
  imp: Box<dyn ChannelImpl>,
  _pinned: PhantomPinned,
}

impl ChannelHeap {
  unsafe fn from_raw<'b>(this: *const RawChannel) -> &'b ChannelHeap {
    unsafe { &(*this.cast::<ChannelHeap>()) }
  }
}

impl Channel {
  pub fn new(imp: Box<dyn ChannelImpl>) -> Self {
    let heap = Box::into_raw(Box::new(MaybeUninit::<ChannelHeap>::uninit()))
      .cast::<ChannelHeap>();

    unsafe {
      let raw = &raw mut (*heap).raw;
      v8_inspector__V8Inspector__Channel__BASE__CONSTRUCT(raw.cast());
      let imp_ptr = &raw mut (*heap).imp;
      imp_ptr.write(imp);
    }

    Self {
      heap: unsafe { Box::into_pin(Box::from_raw(heap.cast::<ChannelHeap>())) },
    }
  }

  fn raw(&self) -> *mut RawChannel {
    self.heap.raw.get()
  }

  pub fn send_response(&self, call_id: i32, message: UniquePtr<StringBuffer>) {
    unsafe {
      v8_inspector__V8Inspector__Channel__sendResponse(
        self.raw(),
        call_id,
        message,
      );
    }
  }
  pub fn send_notification(&self, message: UniquePtr<StringBuffer>) {
    unsafe {
      v8_inspector__V8Inspector__Channel__sendNotification(self.raw(), message);
    }
  }
  pub fn flush_protocol_notifications(&self) {
    unsafe {
      v8_inspector__V8Inspector__Channel__flushProtocolNotifications(
        self.raw(),
      );
    }
  }
}

pub trait ChannelImpl {
  fn send_response(&self, call_id: i32, message: UniquePtr<StringBuffer>);
  fn send_notification(&self, message: UniquePtr<StringBuffer>);
  fn flush_protocol_notifications(&self);
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::support::UniquePtr;
  use std::sync::atomic::AtomicUsize;
  use std::sync::atomic::Ordering::SeqCst;

  static MESSAGE: &[u8] = b"Hello Pluto!";
  static CALL_COUNT: AtomicUsize = AtomicUsize::new(0);

  // Using repr(C) to preserve field ordering and test that everything works
  // when the ChannelBase field is not the first element of the struct.
  #[repr(C)]
  #[derive(Debug)]
  pub struct TestChannel {
    field1: i32,
    field2: u64,
  }

  impl ChannelImpl for TestChannel {
    fn send_response(
      &self,
      call_id: i32,
      mut message: UniquePtr<StringBuffer>,
    ) {
      assert_eq!(call_id, 999);
      assert_eq!(message.as_mut().unwrap().string().len(), MESSAGE.len());
      self.log_call();
    }
    fn send_notification(&self, mut message: UniquePtr<StringBuffer>) {
      assert_eq!(message.as_mut().unwrap().string().len(), MESSAGE.len());
      self.log_call();
    }
    fn flush_protocol_notifications(&self) {
      self.log_call();
    }
  }

  impl TestChannel {
    pub fn new() -> Self {
      Self {
        field1: -42,
        field2: 420,
      }
    }

    fn log_call(&self) {
      assert_eq!(self.field1, -42);
      assert_eq!(self.field2, 420);
      CALL_COUNT.fetch_add(1, SeqCst);
    }
  }

  #[test]
  fn test_channel() {
    let channel = TestChannel::new();
    let msg_view = StringView::from(MESSAGE);
    channel.send_response(999, StringBuffer::create(msg_view));
    assert_eq!(CALL_COUNT.swap(0, SeqCst), 1);
    channel.send_notification(StringBuffer::create(msg_view));
    assert_eq!(CALL_COUNT.swap(0, SeqCst), 1);
    channel.flush_protocol_notifications();
    assert_eq!(CALL_COUNT.swap(0, SeqCst), 1);
  }
}

#[repr(C)]
#[derive(Debug)]
pub struct RawV8InspectorClient {
  _cxx_vtable: CxxVTable,
}

impl V8InspectorClient {
  pub fn run_message_loop_on_pause(&self, context_group_id: i32) {
    unsafe {
      v8_inspector__V8InspectorClient__runMessageLoopOnPause(
        self.raw(),
        context_group_id,
      );
    }
  }

  pub fn quit_message_loop_on_pause(&self) {
    unsafe {
      v8_inspector__V8InspectorClient__quitMessageLoopOnPause(self.raw())
    }
  }

  pub fn run_if_waiting_for_debugger(&self, context_group_id: i32) {
    unsafe {
      v8_inspector__V8InspectorClient__runIfWaitingForDebugger(
        self.raw(),
        context_group_id,
      );
    }
  }

  #[allow(clippy::too_many_arguments)]
  pub fn console_api_message(
    &self,
    context_group_id: i32,
    level: i32,
    message: &StringView,
    url: &StringView,
    line_number: u32,
    column_number: u32,
    stack_trace: &mut V8StackTrace,
  ) {
    unsafe {
      v8_inspector__V8InspectorClient__consoleAPIMessage(
        self.raw(),
        context_group_id,
        level,
        message,
        url,
        line_number,
        column_number,
        stack_trace,
      );
    }
  }

  pub fn generate_unique_id(&self) -> i64 {
    unsafe { v8_inspector__V8InspectorClient__generateUniqueId(self.raw()) }
  }
}

#[allow(unused_variables)]
pub trait V8InspectorClientImpl {
  fn run_message_loop_on_pause(&self, context_group_id: i32) {}
  fn quit_message_loop_on_pause(&self) {}
  fn run_if_waiting_for_debugger(&self, context_group_id: i32) {}

  fn generate_unique_id(&self) -> i64 {
    0 // 0 = let V8 pick a unique id itself
  }

  #[allow(clippy::too_many_arguments)]
  fn console_api_message(
    &self,
    context_group_id: i32,
    level: i32,
    message: &StringView,
    url: &StringView,
    line_number: u32,
    column_number: u32,
    stack_trace: &mut V8StackTrace,
  ) {
  }

  fn ensure_default_context_in_group(
    &self,
    context_group_id: i32,
  ) -> Option<Local<'_, Context>> {
    None
  }

  fn resource_name_to_url(
    &self,
    resource_name: &StringView,
  ) -> Option<UniquePtr<StringBuffer>> {
    None
  }
}

// V8 will hold onto a raw pointer to the RawV8InspectorClient, so we need to
// make sure it stays pinned.
#[repr(C)]
struct V8InspectorClientHeap {
  raw: UnsafeCell<RawV8InspectorClient>,
  // this doesn't need to be pinned, but it's convenient to keep it here
  // so we can access it from a pointer to the RawV8InspectorClient
  imp: Box<dyn V8InspectorClientImpl>,
  _pinned: PhantomPinned,
}

impl V8InspectorClientHeap {
  unsafe fn from_raw<'b>(
    this: *const RawV8InspectorClient,
  ) -> &'b V8InspectorClientHeap {
    unsafe { &(*this.cast::<V8InspectorClientHeap>()) }
  }
}

pub struct V8InspectorClient {
  heap: Pin<Box<V8InspectorClientHeap>>,
}

impl V8InspectorClient {
  pub fn new(imp: Box<dyn V8InspectorClientImpl>) -> V8InspectorClient {
    let heap = unsafe {
      let heap =
        Box::into_raw(Box::new(MaybeUninit::<V8InspectorClientHeap>::uninit()));
      let raw = &raw mut (*heap.cast::<V8InspectorClientHeap>()).raw;
      v8_inspector__V8InspectorClient__BASE__CONSTRUCT(
        raw.cast::<MaybeUninit<RawV8InspectorClient>>(),
      );
      let imp_ptr = &raw mut (*heap.cast::<V8InspectorClientHeap>()).imp;
      imp_ptr.write(imp);
      Box::into_pin(Box::from_raw(heap.cast::<V8InspectorClientHeap>()))
    };

    Self { heap }
  }

  fn raw(&self) -> *mut RawV8InspectorClient {
    self.heap.raw.get()
  }
}

impl Debug for V8InspectorClient {
  fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
    f.debug_struct("V8InspectorClient").finish()
  }
}

#[repr(C)]
#[derive(Debug)]
pub struct RawV8InspectorSession(Opaque);

pub struct V8InspectorSession {
  raw: UniqueRef<RawV8InspectorSession>,
  // this isn't actually used, but it needs to live
  // as long as the session
  _channel: Channel,
}

impl V8InspectorSession {
  pub fn can_dispatch_method(method: StringView) -> bool {
    unsafe { v8_inspector__V8InspectorSession__canDispatchMethod(method) }
  }

  pub fn dispatch_protocol_message(&self, message: StringView) {
    unsafe {
      v8_inspector__V8InspectorSession__dispatchProtocolMessage(
        self.raw.as_ptr(),
        message,
      );
    }
  }

  pub fn schedule_pause_on_next_statement(
    &self,
    reason: StringView,
    detail: StringView,
  ) {
    unsafe {
      v8_inspector__V8InspectorSession__schedulePauseOnNextStatement(
        self.raw.as_ptr(),
        reason,
        detail,
      );
    }
  }
}

impl Drop for V8InspectorSession {
  fn drop(&mut self) {
    unsafe { v8_inspector__V8InspectorSession__DELETE(self.raw.as_ptr()) };
  }
}

// TODO: in C++, this class is intended to be user-extensible, just like
// like `Task`, `Client`, `Channel`. In Rust this would ideally also be the
// case, but currently to obtain a `UniquePtr<StringBuffer>` is by making a
// copy using `StringBuffer::create()`.
#[repr(C)]
#[derive(Debug)]
pub struct StringBuffer {
  _cxx_vtable: CxxVTable,
}

// TODO: make it possible to obtain a `UniquePtr<StringBuffer>` directly from
// an owned `Vec<u8>` or `Vec<u16>`,
impl StringBuffer {
  // The C++ class definition does not declare `string()` to be a const method,
  // therefore we declare self as mutable here.
  // TODO: figure out whether it'd be safe to assume a const receiver here.
  // That would make it possible to implement `Deref<Target = StringBuffer>`.
  pub fn string(&self) -> StringView<'_> {
    unsafe { v8_inspector__StringBuffer__string(self) }
  }

  /// This method copies contents.
  pub fn create(source: StringView) -> UniquePtr<StringBuffer> {
    unsafe { v8_inspector__StringBuffer__create(source) }
  }
}

impl Drop for StringBuffer {
  fn drop(&mut self) {
    unsafe { v8_inspector__StringBuffer__DELETE(self) }
  }
}

unsafe impl Send for StringBuffer {}
use std::iter::ExactSizeIterator;
use std::iter::IntoIterator;
use std::marker::PhantomData;
use std::marker::PhantomPinned;
use std::mem::MaybeUninit;
use std::ops::Deref;
use std::pin::Pin;
use std::ptr::NonNull;
use std::ptr::null;
use std::slice;
use std::string;

// Notes:
//  * This class is ported, not wrapped using bindings.
//  * Since Rust `repr(bool)` is not allowed, we're assuming that `bool` and
//    `u8` have the same size. This is assumption is checked in 'support.h'.
//    TODO: find/open upstream issue to allow #[repr(bool)] support.

#[derive(Clone, Debug, Copy)]
#[repr(u8)]
pub enum StringView<'a> {
  // Do not reorder!
  U16(CharacterArray<'a, u16>),
  U8(CharacterArray<'a, u8>),
}

impl StringView<'static> {
  pub fn empty() -> Self {
    Self::U8(CharacterArray::<'static, u8>::empty())
  }
}

impl fmt::Display for StringView<'_> {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      Self::U16(v) => write!(f, "{v}"),
      Self::U8(v) => write!(f, "{v}"),
    }
  }
}

impl<'a> From<&'a [u8]> for StringView<'a> {
  fn from(v: &'a [u8]) -> Self {
    Self::U8(CharacterArray::<'a, u8>::from(v))
  }
}

impl<'a> From<&'a [u16]> for StringView<'a> {
  fn from(v: &'a [u16]) -> Self {
    Self::U16(CharacterArray::<'a, u16>::from(v))
  }
}

impl StringView<'_> {
  pub fn is_8bit(&self) -> bool {
    match self {
      Self::U16(..) => false,
      Self::U8(..) => true,
    }
  }

  pub fn is_empty(&self) -> bool {
    self.len() == 0
  }

  pub fn len(&self) -> usize {
    match self {
      Self::U16(v) => v.len(),
      Self::U8(v) => v.len(),
    }
  }

  pub fn characters8(&self) -> Option<&[u8]> {
    match self {
      Self::U16(..) => None,
      Self::U8(v) => Some(v),
    }
  }

  pub fn characters16(&self) -> Option<&[u16]> {
    match self {
      Self::U16(v) => Some(v),
      Self::U8(..) => None,
    }
  }
}

impl<'a> IntoIterator for StringView<'a> {
  type IntoIter = StringViewIterator<'a>;
  type Item = u16;

  fn into_iter(self) -> Self::IntoIter {
    StringViewIterator { view: self, pos: 0 }
  }
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct CharacterArray<'a, T> {
  m_length: usize,
  m_characters: *const T,
  _phantom: PhantomData<&'a T>,
}

impl CharacterArray<'static, u8> {
  pub fn empty() -> Self {
    Self {
      m_length: 0,
      m_characters: null(),
      _phantom: PhantomData,
    }
  }
}

impl<T> CharacterArray<'_, T>
where
  T: Copy,
{
  #[inline(always)]
  fn len(&self) -> usize {
    self.m_length
  }

  #[inline(always)]
  fn get_at(&self, index: usize) -> Option<T> {
    if index < self.m_length {
      Some(unsafe { *self.m_characters.add(index) })
    } else {
      None
    }
  }
}

unsafe impl<T> Send for CharacterArray<'_, T> where T: Copy {}
unsafe impl<T> Sync for CharacterArray<'_, T> where T: Sync {}

impl fmt::Display for CharacterArray<'_, u8> {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    f.write_str(
      self
        .iter()
        .cloned()
        .map(char::from)
        .collect::<string::String>()
        .as_str(),
    )
  }
}

impl fmt::Display for CharacterArray<'_, u16> {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    f.write_str(&string::String::from_utf16_lossy(self))
  }
}

impl<'a, T> From<&'a [T]> for CharacterArray<'a, T> {
  fn from(v: &'a [T]) -> Self {
    Self {
      m_length: v.len(),
      m_characters: v.as_ptr(),
      _phantom: PhantomData,
    }
  }
}

impl<T> Deref for CharacterArray<'_, T> {
  type Target = [T];

  fn deref(&self) -> &[T] {
    let Self {
      m_length,
      mut m_characters,
      ..
    } = *self;
    if m_characters.is_null() {
      assert_eq!(m_length, 0);
      m_characters = NonNull::dangling().as_ptr();
    };
    unsafe { slice::from_raw_parts(m_characters, m_length) }
  }
}

#[derive(Copy, Clone, Debug)]
pub struct StringViewIterator<'a> {
  view: StringView<'a>,
  pos: usize,
}

impl Iterator for StringViewIterator<'_> {
  type Item = u16;

  fn next(&mut self) -> Option<Self::Item> {
    let result = Some(match self.view {
      StringView::U16(v) => v.get_at(self.pos)?,
      StringView::U8(v) => u16::from(v.get_at(self.pos)?),
    });
    self.pos += 1;
    result
  }
}

impl ExactSizeIterator for StringViewIterator<'_> {
  fn len(&self) -> usize {
    self.view.len()
  }
}

#[test]
fn string_view_display() {
  let ok: [u16; 2] = [111, 107];
  assert_eq!("ok", format!("{}", StringView::from(&ok[..])));
  assert_eq!("ok", format!("{}", StringView::from(&b"ok"[..])));
  assert_eq!("ØÞ", format!("{}", StringView::from(&[216u8, 222u8][..])));
}

#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq)]
#[repr(C)]
pub enum V8InspectorClientTrustLevel {
  Untrusted = 0,
  FullyTrusted = 1,
}

#[repr(C)]
#[derive(Debug)]
pub struct RawV8Inspector(Opaque);

pub struct V8Inspector {
  raw: UniqueRef<RawV8Inspector>,
  _client: V8InspectorClient,
}

impl V8Inspector {
  pub fn create(
    isolate: &mut Isolate,
    client: V8InspectorClient,
  ) -> V8Inspector {
    let raw = unsafe {
      UniqueRef::from_raw(v8_inspector__V8Inspector__create(
        isolate.as_real_ptr(),
        client.raw(),
      ))
    };
    V8Inspector {
      raw,
      _client: client,
    }
  }

  // note: in theory v8 could mutate through this pointer.
  // this is fine, though, because we never create a rust reference
  // to the actual RawV8Inspector, we only use raw pointers which
  // don't enforce the immutability guarantee
  fn raw(&self) -> *mut RawV8Inspector {
    self.raw.as_ptr()
  }

  pub fn connect(
    &self,
    context_group_id: i32,
    channel: Channel,
    state: StringView,
    client_trust_level: V8InspectorClientTrustLevel,
  ) -> V8InspectorSession {
    let raw = unsafe {
      UniqueRef::from_raw(v8_inspector__V8Inspector__connect(
        self.raw(),
        context_group_id,
        channel.raw(),
        state,
        client_trust_level,
      ))
    };

    V8InspectorSession {
      raw,
      _channel: channel,
    }
  }

  /// Note: this method deviates from the C++ API here because it's a lot of
  /// work to bind the V8ContextInfo, which is not used elsewhere.
  pub fn context_created(
    &self,
    context: Local<Context>,
    context_group_id: i32,
    human_readable_name: StringView,
    aux_data: StringView,
  ) {
    unsafe {
      v8_inspector__V8Inspector__contextCreated(
        self.raw(),
        &*context,
        context_group_id,
        human_readable_name,
        aux_data,
      );
    }
  }

  pub fn context_destroyed(&self, context: Local<Context>) {
    unsafe {
      v8_inspector__V8Inspector__contextDestroyed(self.raw(), &*context)
    }
  }

  #[allow(clippy::too_many_arguments)]
  pub fn exception_thrown(
    &self,
    context: Local<Context>,
    message: StringView,
    exception: Local<Value>,
    detailed_message: StringView,
    url: StringView,
    line_number: u32,
    column_number: u32,
    stack_trace: UniquePtr<V8StackTrace>,
    script_id: i32,
  ) -> u32 {
    unsafe {
      v8_inspector__V8Inspector__exceptionThrown(
        self.raw(),
        &*context,
        message,
        &*exception,
        detailed_message,
        url,
        line_number,
        column_number,
        stack_trace.into_raw(),
        script_id,
      )
    }
  }

  pub fn create_stack_trace(
    &self,
    stack_trace: Option<Local<StackTrace>>,
  ) -> UniquePtr<V8StackTrace> {
    unsafe {
      UniquePtr::from_raw(v8_inspector__V8Inspector__createStackTrace(
        self.raw(),
        stack_trace.map_or(null(), |v| &*v),
      ))
    }
  }
}

impl Drop for V8Inspector {
  fn drop(&mut self) {
    unsafe { v8_inspector__V8Inspector__DELETE(self.raw()) };
  }
}

#[repr(C)]
#[derive(Debug)]
pub struct V8StackTrace {
  _cxx_vtable: CxxVTable,
}

impl Drop for V8StackTrace {
  fn drop(&mut self) {
    unsafe { v8_inspector__V8StackTrace__DELETE(self) };
  }
}

// TODO(bnoordhuis) This needs to be fleshed out more but that can wait
// until it's actually needed.
