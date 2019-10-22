use crate::support::int;
use crate::support::CxxVTable;
use crate::support::FieldOffset;
use crate::support::Opaque;
use crate::support::RustVTable;

// class V8InspectorClient {
//  public:
//   virtual ~V8InspectorClient() = default;
//
//   virtual void runMessageLoopOnPause(int contextGroupId) {}
//   virtual void quitMessageLoopOnPause() {}
//   virtual void runIfWaitingForDebugger(int contextGroupId) {}
//
//   virtual void muteMetrics(int contextGroupId) {}
//   virtual void unmuteMetrics(int contextGroupId) {}
//
//   virtual void beginUserGesture() {}
//   virtual void endUserGesture() {}
//
//   virtual std::unique_ptr<StringBuffer> valueSubtype(v8::Local<v8::Value>) {
//     return nullptr;
//   }
//   virtual bool formatAccessorsAsProperties(v8::Local<v8::Value>) {
//     return false;
//   }
//   virtual bool isInspectableHeapObject(v8::Local<v8::Object>) {
//     return true;
//   }
//
//   virtual v8::Local<v8::Context> ensureDefaultContextInGroup(
//       int contextGroupId) {
//     return v8::Local<v8::Context>();
//   }
//   virtual void beginEnsureAllContextsInGroup(int contextGroupId) {}
//   virtual void endEnsureAllContextsInGroup(int contextGroupId) {}
//
//   virtual void installAdditionalCommandLineAPI(v8::Local<v8::Context>,
//                                                v8::Local<v8::Object>) {}
//   virtual void consoleAPIMessage(int contextGroupId,
//                                  v8::Isolate::MessageErrorLevel level,
//                                  const StringView& message,
//                                  const StringView& url, unsigned lineNumber,
//                                  unsigned columnNumber, V8StackTrace*) {}
//   virtual v8::MaybeLocal<v8::Value> memoryInfo(v8::Isolate*,
//                                                v8::Local<v8::Context>) {
//     return v8::MaybeLocal<v8::Value>();
//   }
//
//   virtual void consoleTime(const StringView& title) {}
//   virtual void consoleTimeEnd(const StringView& title) {}
//   virtual void consoleTimeStamp(const StringView& title) {}
//   virtual void consoleClear(int contextGroupId) {}
//   virtual double currentTimeMS() { return 0; }
//   typedef void (*TimerCallback)(void*);
//   virtual void startRepeatingTimer(double, TimerCallback, void* data) {}
//   virtual void cancelTimer(void* data) {}
//
//   virtual bool canExecuteScripts(int contextGroupId) { return true; }
//
//   virtual void maxAsyncCallStackDepthChanged(int depth) {}
//
//   virtual std::unique_ptr<StringBuffer> resourceNameToUrl(
//       const StringView& resourceName) {
//     return nullptr;
//   }
// };

extern "C" {
  fn v8_inspector__V8InspectorClient__BASE__CONSTRUCT(
    buf: &mut std::mem::MaybeUninit<Client>,
  ) -> ();

  fn v8_inspector__V8InspectorClient__runMessageLoopOnPause(
    this: &mut Client,
    contextGroupId: int,
  ) -> ();
  fn v8_inspector__V8InspectorClient__quitMessageLoopOnPause(
    this: &mut Client,
  ) -> ();
  fn v8_inspector__V8InspectorClient__runIfWaitingForDebugger(
    this: &mut Client,
    contextGroupId: int,
  ) -> ();
}

#[no_mangle]
pub unsafe extern "C" fn v8_inspector__V8InspectorClient__BASE__runMessageLoopOnPause(
  this: &mut Client,
  contextGroupId: int,
) -> () {
  ClientBase::dispatch_mut(this).runMessageLoopOnPause(contextGroupId)
}

#[no_mangle]
pub unsafe extern "C" fn v8_inspector__V8InspectorClient__BASE__quitMessageLoopOnPause(
  this: &mut Client,
) -> () {
  ClientBase::dispatch_mut(this).quitMessageLoopOnPause()
}

#[no_mangle]
pub unsafe extern "C" fn v8_inspector__V8InspectorClient__BASE__runIfWaitingForDebugger(
  this: &mut Client,
  contextGroupId: int,
) -> () {
  ClientBase::dispatch_mut(this).runIfWaitingForDebugger(contextGroupId)
}

#[repr(C)]
pub struct Client {
  _cxx_vtable: CxxVTable,
}

impl Client {
  pub fn runMessageLoopOnPause(&mut self, contextGroupId: int) -> () {
    unsafe {
      v8_inspector__V8InspectorClient__runMessageLoopOnPause(
        self,
        contextGroupId,
      )
    }
  }
  pub fn quitMessageLoopOnPause(&mut self) -> () {
    unsafe { v8_inspector__V8InspectorClient__quitMessageLoopOnPause(self) }
  }
  pub fn runIfWaitingForDebugger(&mut self, contextGroupId: int) -> () {
    unsafe {
      v8_inspector__V8InspectorClient__runIfWaitingForDebugger(
        self,
        contextGroupId,
      )
    }
  }
}

pub trait AsClient {
  fn as_client(&self) -> &Client;
  fn as_client_mut(&mut self) -> &mut Client;
}

impl AsClient for Client {
  fn as_client(&self) -> &Client {
    self
  }
  fn as_client_mut(&mut self) -> &mut Client {
    self
  }
}

impl<T> AsClient for T
where
  T: ClientImpl,
{
  fn as_client(&self) -> &Client {
    &self.base().cxx_base
  }
  fn as_client_mut(&mut self) -> &mut Client {
    &mut self.base_mut().cxx_base
  }
}

#[allow(unused_variables)]
pub trait ClientImpl: AsClient {
  fn base(&self) -> &ClientBase;
  fn base_mut(&mut self) -> &mut ClientBase;

  fn runMessageLoopOnPause(&mut self, contextGroupId: int) -> () {}
  fn quitMessageLoopOnPause(&mut self) -> () {}
  fn runIfWaitingForDebugger(&mut self, contextGroupId: int) -> () {}
}

pub struct ClientBase {
  cxx_base: Client,
  offset_within_embedder: FieldOffset<Self>,
  rust_vtable: RustVTable<&'static dyn ClientImpl>,
}

impl ClientBase {
  fn construct_cxx_base() -> Client {
    unsafe {
      let mut buf = std::mem::MaybeUninit::<Client>::uninit();
      v8_inspector__V8InspectorClient__BASE__CONSTRUCT(&mut buf);
      buf.assume_init()
    }
  }

  fn get_cxx_base_offset() -> FieldOffset<Client> {
    let buf = std::mem::MaybeUninit::<Self>::uninit();
    FieldOffset::from_ptrs(buf.as_ptr(), unsafe { &(*buf.as_ptr()).cxx_base })
  }

  fn get_offset_within_embedder<T>() -> FieldOffset<Self>
  where
    T: ClientImpl,
  {
    let buf = std::mem::MaybeUninit::<T>::uninit();
    let embedder_ptr: *const T = buf.as_ptr();
    let self_ptr: *const Self = unsafe { (*embedder_ptr).base() };
    FieldOffset::from_ptrs(embedder_ptr, self_ptr)
  }

  fn get_rust_vtable<T>() -> RustVTable<&'static dyn ClientImpl>
  where
    T: ClientImpl,
  {
    let buf = std::mem::MaybeUninit::<T>::uninit();
    let embedder_ptr = buf.as_ptr();
    let trait_object: *const dyn ClientImpl = embedder_ptr;
    let (data_ptr, vtable): (*const T, RustVTable<_>) =
      unsafe { std::mem::transmute(trait_object) };
    assert_eq!(data_ptr, embedder_ptr);
    vtable
  }

  pub fn new<T>() -> Self
  where
    T: ClientImpl,
  {
    Self {
      cxx_base: Self::construct_cxx_base(),
      offset_within_embedder: Self::get_offset_within_embedder::<T>(),
      rust_vtable: Self::get_rust_vtable::<T>(),
    }
  }

  pub unsafe fn dispatch(client: &Client) -> &dyn ClientImpl {
    let this = Self::get_cxx_base_offset().to_embedder::<Self>(client);
    let embedder = this.offset_within_embedder.to_embedder::<Opaque>(this);
    std::mem::transmute((embedder, this.rust_vtable))
  }

  pub unsafe fn dispatch_mut(client: &mut Client) -> &mut dyn ClientImpl {
    let this = Self::get_cxx_base_offset().to_embedder_mut::<Self>(client);
    let vtable = this.rust_vtable;
    let embedder = this.offset_within_embedder.to_embedder_mut::<Opaque>(this);
    std::mem::transmute((embedder, vtable))
  }
}
