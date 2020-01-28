use super::{StringView, V8StackTrace};
use crate::support::int;
use crate::support::CxxVTable;
use crate::support::FieldOffset;
use crate::support::Opaque;
use crate::support::RustVTable;

extern "C" {
  fn v8_inspector__V8InspectorClient__BASE__CONSTRUCT(
    buf: &mut std::mem::MaybeUninit<V8InspectorClient>,
  ) -> ();

  fn v8_inspector__V8InspectorClient__runMessageLoopOnPause(
    this: &mut V8InspectorClient,
    context_group_id: int,
  ) -> ();
  fn v8_inspector__V8InspectorClient__quitMessageLoopOnPause(
    this: &mut V8InspectorClient,
  ) -> ();
  fn v8_inspector__V8InspectorClient__runIfWaitingForDebugger(
    this: &mut V8InspectorClient,
    context_group_id: int,
  ) -> ();
  fn v8_inspector__V8InspectorClient__consoleAPIMessage(
    this: &mut V8InspectorClient,
    context_group_id: int,
    level: int,
    message: &StringView,
    url: &StringView,
    line_number: u32,
    column_number: u32,
    stack_trace: &mut V8StackTrace,
  ) -> ();
}

#[no_mangle]
pub unsafe extern "C" fn v8_inspector__V8InspectorClient__BASE__runMessageLoopOnPause(
  this: &mut V8InspectorClient,
  context_group_id: int,
) {
  V8InspectorClientBase::dispatch_mut(this)
    .run_message_loop_on_pause(context_group_id)
}

#[no_mangle]
pub unsafe extern "C" fn v8_inspector__V8InspectorClient__BASE__quitMessageLoopOnPause(
  this: &mut V8InspectorClient,
) {
  V8InspectorClientBase::dispatch_mut(this).quit_message_loop_on_pause()
}

#[no_mangle]
pub unsafe extern "C" fn v8_inspector__V8InspectorClient__BASE__runIfWaitingForDebugger(
  this: &mut V8InspectorClient,
  context_group_id: int,
) {
  V8InspectorClientBase::dispatch_mut(this)
    .run_if_waiting_for_debugger(context_group_id)
}

#[no_mangle]
pub unsafe extern "C" fn v8_inspector__V8InspectorClient__BASE__consoleAPIMessage(
  this: &mut V8InspectorClient,
  context_group_id: int,
  level: int,
  message: &StringView,
  url: &StringView,
  line_number: u32,
  column_number: u32,
  stack_trace: &mut V8StackTrace,
) {
  V8InspectorClientBase::dispatch_mut(this).console_api_message(
    context_group_id,
    level,
    message,
    url,
    line_number,
    column_number,
    stack_trace,
  )
}

#[repr(C)]
pub struct V8InspectorClient {
  _cxx_vtable: CxxVTable,
}

impl V8InspectorClient {
  pub fn run_message_loop_on_pause(&mut self, context_group_id: i32) {
    unsafe {
      v8_inspector__V8InspectorClient__runMessageLoopOnPause(
        self,
        context_group_id,
      )
    }
  }

  pub fn quit_message_loop_on_pause(&mut self) {
    unsafe { v8_inspector__V8InspectorClient__quitMessageLoopOnPause(self) }
  }

  pub fn run_if_waiting_for_debugger(&mut self, context_group_id: i32) {
    unsafe {
      v8_inspector__V8InspectorClient__runIfWaitingForDebugger(
        self,
        context_group_id,
      )
    }
  }

  #[allow(clippy::too_many_arguments)]
  pub fn console_api_message(
    &mut self,
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
        self,
        context_group_id,
        level,
        message,
        url,
        line_number,
        column_number,
        stack_trace,
      )
    }
  }
}

pub trait AsV8InspectorClient {
  fn as_client(&self) -> &V8InspectorClient;
  fn as_client_mut(&mut self) -> &mut V8InspectorClient;
}

impl AsV8InspectorClient for V8InspectorClient {
  fn as_client(&self) -> &V8InspectorClient {
    self
  }
  fn as_client_mut(&mut self) -> &mut V8InspectorClient {
    self
  }
}

impl<T> AsV8InspectorClient for T
where
  T: V8InspectorClientImpl,
{
  fn as_client(&self) -> &V8InspectorClient {
    &self.base().cxx_base
  }
  fn as_client_mut(&mut self) -> &mut V8InspectorClient {
    &mut self.base_mut().cxx_base
  }
}

#[allow(unused_variables)]
pub trait V8InspectorClientImpl: AsV8InspectorClient {
  fn base(&self) -> &V8InspectorClientBase;
  fn base_mut(&mut self) -> &mut V8InspectorClientBase;

  fn run_message_loop_on_pause(&mut self, context_group_id: i32) {}
  fn quit_message_loop_on_pause(&mut self) {}
  fn run_if_waiting_for_debugger(&mut self, context_group_id: i32) {}

  #[allow(clippy::too_many_arguments)]
  fn console_api_message(
    &mut self,
    context_group_id: i32,
    level: i32,
    message: &StringView,
    url: &StringView,
    line_number: u32,
    column_number: u32,
    stack_trace: &mut V8StackTrace,
  ) {
  }
}

pub struct V8InspectorClientBase {
  cxx_base: V8InspectorClient,
  offset_within_embedder: FieldOffset<Self>,
  rust_vtable: RustVTable<&'static dyn V8InspectorClientImpl>,
}

impl V8InspectorClientBase {
  fn construct_cxx_base() -> V8InspectorClient {
    unsafe {
      let mut buf = std::mem::MaybeUninit::<V8InspectorClient>::uninit();
      v8_inspector__V8InspectorClient__BASE__CONSTRUCT(&mut buf);
      buf.assume_init()
    }
  }

  fn get_cxx_base_offset() -> FieldOffset<V8InspectorClient> {
    let buf = std::mem::MaybeUninit::<Self>::uninit();
    FieldOffset::from_ptrs(buf.as_ptr(), unsafe { &(*buf.as_ptr()).cxx_base })
  }

  fn get_offset_within_embedder<T>() -> FieldOffset<Self>
  where
    T: V8InspectorClientImpl,
  {
    let buf = std::mem::MaybeUninit::<T>::uninit();
    let embedder_ptr: *const T = buf.as_ptr();
    let self_ptr: *const Self = unsafe { (*embedder_ptr).base() };
    FieldOffset::from_ptrs(embedder_ptr, self_ptr)
  }

  fn get_rust_vtable<T>() -> RustVTable<&'static dyn V8InspectorClientImpl>
  where
    T: V8InspectorClientImpl,
  {
    let buf = std::mem::MaybeUninit::<T>::uninit();
    let embedder_ptr = buf.as_ptr();
    let trait_object: *const dyn V8InspectorClientImpl = embedder_ptr;
    let (data_ptr, vtable): (*const T, RustVTable<_>) =
      unsafe { std::mem::transmute(trait_object) };
    assert_eq!(data_ptr, embedder_ptr);
    vtable
  }

  pub fn new<T>() -> Self
  where
    T: V8InspectorClientImpl,
  {
    Self {
      cxx_base: Self::construct_cxx_base(),
      offset_within_embedder: Self::get_offset_within_embedder::<T>(),
      rust_vtable: Self::get_rust_vtable::<T>(),
    }
  }

  pub unsafe fn dispatch(
    client: &V8InspectorClient,
  ) -> &dyn V8InspectorClientImpl {
    let this = Self::get_cxx_base_offset().to_embedder::<Self>(client);
    let embedder = this.offset_within_embedder.to_embedder::<Opaque>(this);
    std::mem::transmute((embedder, this.rust_vtable))
  }

  pub unsafe fn dispatch_mut(
    client: &mut V8InspectorClient,
  ) -> &mut dyn V8InspectorClientImpl {
    let this = Self::get_cxx_base_offset().to_embedder_mut::<Self>(client);
    let vtable = this.rust_vtable;
    let embedder = this.offset_within_embedder.to_embedder_mut::<Opaque>(this);
    std::mem::transmute((embedder, vtable))
  }
}
