// Copyright 2019-2021 the Deno authors. All rights reserved. MIT license.

use crate::function::FunctionCallbackArguments;
use crate::function::FunctionCallbackInfo;
use crate::scope::CallbackScope;
use crate::scope::HandleScope;
use crate::support::UnitType;
use crate::Isolate;
use crate::Local;
use crate::Value;
use std::ptr::null;
use std::ptr::null_mut;

// Type-erased std::shared_ptr<v8::WasmStreaming>. Assumes it's safe
// to move around (no backlinks). Not generally true for shared_ptrs
// but it is in this case - other shared_ptrs that point to the same
// v8::WasmStreaming exist but are managed by V8 and don't leak out.
//
// We don't use crate::support::SharedPtr because it only allows
// immutable borrows and derefs to avoid aliasing but that's not
// a problem here, only a single instance outside V8 exists.
//
// Note: uses *mut u8 rather than e.g. usize to enforce !Send and !Sync.
#[repr(C)]
struct WasmStreamingSharedPtr([*mut u8; 2]);

/// The V8 interface for WebAssembly streaming compilation.
/// When streaming compilation is initiated, V8 passes a [Self]
/// object to the embedder such that the embedder can pass the
/// input bytes for streaming compilation to V8.
#[repr(C)]
pub struct WasmStreaming(WasmStreamingSharedPtr);

impl WasmStreaming {
  /// Pass a new chunk of bytes to WebAssembly streaming compilation.
  pub fn on_bytes_received(&mut self, data: &[u8]) {
    unsafe {
      v8__WasmStreaming__OnBytesReceived(&mut self.0, data.as_ptr(), data.len())
    }
  }

  /// Should be called after all received bytes where passed to
  /// [`Self::on_bytes_received()`] to tell V8 that there will be no
  /// more bytes. Does not have to be called after [`Self::abort()`]
  /// has been called already.
  pub fn finish(mut self) {
    unsafe { v8__WasmStreaming__Finish(&mut self.0) }
  }

  /// Abort streaming compilation. If {exception} has a value, then the promise
  /// associated with streaming compilation is rejected with that value. If
  /// {exception} does not have value, the promise does not get rejected.
  pub fn abort(mut self, exception: Option<Local<Value>>) {
    let exception = exception.map(|v| &*v as *const Value).unwrap_or(null());
    unsafe { v8__WasmStreaming__Abort(&mut self.0, exception) }
  }
}

impl Drop for WasmStreaming {
  fn drop(&mut self) {
    unsafe { v8__WasmStreaming__shared_ptr_DESTRUCT(&mut self.0) }
  }
}

pub(crate) fn trampoline<F>() -> extern "C" fn(*const FunctionCallbackInfo)
where
  F: UnitType + Fn(&mut HandleScope, Local<Value>, WasmStreaming),
{
  extern "C" fn c_fn<F>(info: *const FunctionCallbackInfo)
  where
    F: UnitType + Fn(&mut HandleScope, Local<Value>, WasmStreaming),
  {
    let scope = &mut unsafe { CallbackScope::new(&*info) };
    let args = FunctionCallbackArguments::from_function_callback_info(info);
    let data = args.data().unwrap(); // Always present.
    let data = &*data as *const Value;
    let zero = null_mut();
    let mut that = WasmStreamingSharedPtr([zero, zero]);
    unsafe {
      v8__WasmStreaming__Unpack(scope.get_isolate_ptr(), data, &mut that)
    };
    let source = args.get(0);
    (F::get())(scope, source, WasmStreaming(that));
  }
  c_fn::<F>
}

extern "C" {
  fn v8__WasmStreaming__Unpack(
    isolate: *mut Isolate,
    value: *const Value,
    that: *mut WasmStreamingSharedPtr, // Out parameter.
  );
  fn v8__WasmStreaming__shared_ptr_DESTRUCT(this: *mut WasmStreamingSharedPtr);
  fn v8__WasmStreaming__OnBytesReceived(
    this: *mut WasmStreamingSharedPtr,
    data: *const u8,
    len: usize,
  );
  fn v8__WasmStreaming__Finish(this: *mut WasmStreamingSharedPtr);
  fn v8__WasmStreaming__Abort(
    this: *mut WasmStreamingSharedPtr,
    exception: *const Value,
  );
}
