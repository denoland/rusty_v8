use crate::cppgc::GarbageCollected;
use crate::cppgc::Traced;
use crate::ArrayBuffer;
use crate::CallbackScope;
use crate::Context;
use crate::ContextScope;
use crate::Exception;
use crate::HandleScope;
use crate::Isolate;
use crate::Local;
use crate::Object;
use crate::SharedArrayBuffer;
use crate::String;
use crate::TracedReference;
use crate::Value;
use crate::WasmModuleObject;

use crate::support::CxxVTable;
use crate::support::FieldOffset;
use crate::support::MaybeBool;

use std::ffi::c_void;
use std::mem::MaybeUninit;
use std::pin::Pin;
use std::ptr::addr_of;

// Must be == sizeof(v8::ValueDeserializer::Delegate),
// see v8__ValueDeserializer__Delegate__CONSTRUCT().
#[repr(C)]
pub struct CxxValueDeserializerDelegate {
  _cxx_vtable: CxxVTable,
}

#[no_mangle]
pub unsafe extern "C" fn v8__ValueDeserializer__Delegate__ReadHostObject(
  this: &mut CxxValueDeserializerDelegate,
  isolate: *mut Isolate,
) -> *const Object {
  let value_deserializer_heap = ValueDeserializerHeap::dispatch_mut(this);
  let scope = &mut CallbackScope::new(isolate.as_mut().unwrap());
  let context = value_deserializer_heap.context.get(scope).unwrap();
  let scope = &mut ContextScope::new(scope, context);
  let value_deserializer_impl =
    value_deserializer_heap.value_deserializer_impl.as_mut();
  match value_deserializer_impl.read_host_object(
    scope,
    &mut value_deserializer_heap.cxx_value_deserializer,
  ) {
    None => std::ptr::null(),
    Some(x) => x.as_non_null().as_ptr(),
  }
}

#[no_mangle]
pub unsafe extern "C" fn v8__ValueDeserializer__Delegate__GetSharedArrayBufferFromId(
  this: &mut CxxValueDeserializerDelegate,
  isolate: *mut Isolate,
  transfer_id: u32,
) -> *const SharedArrayBuffer {
  let value_deserializer_heap = ValueDeserializerHeap::dispatch_mut(this);
  let scope = &mut CallbackScope::new(isolate.as_mut().unwrap());
  let context = value_deserializer_heap.context.get(scope).unwrap();
  let scope = &mut ContextScope::new(scope, context);

  let value_deserializer_impl =
    value_deserializer_heap.value_deserializer_impl.as_mut();
  match value_deserializer_impl
    .get_shared_array_buffer_from_id(scope, transfer_id)
  {
    None => std::ptr::null(),
    Some(x) => x.as_non_null().as_ptr(),
  }
}

#[no_mangle]
pub unsafe extern "C" fn v8__ValueDeserializer__Delegate__GetWasmModuleFromId(
  this: &mut CxxValueDeserializerDelegate,
  isolate: *mut Isolate,
  clone_id: u32,
) -> *const WasmModuleObject {
  let value_deserializer_heap = ValueDeserializerHeap::dispatch_mut(this);
  let scope = &mut CallbackScope::new(isolate.as_mut().unwrap());
  let context = value_deserializer_heap.context.get(scope).unwrap();
  let scope = &mut ContextScope::new(scope, context);
  let value_deserializer_impl =
    value_deserializer_heap.value_deserializer_impl.as_mut();
  match value_deserializer_impl.get_wasm_module_from_id(scope, clone_id) {
    None => std::ptr::null(),
    Some(x) => x.as_non_null().as_ptr(),
  }
}

extern "C" {
  fn v8__ValueDeserializer__Delegate__CONSTRUCT(
    buf: *mut MaybeUninit<CxxValueDeserializerDelegate>,
  );
}

// Must be == sizeof(v8::ValueDeserializer),
// see v8__ValueDeserializer__CONSTRUCT().
#[repr(C)]
pub struct CxxValueDeserializer {
  _cxx_vtable: CxxVTable,
}

extern "C" {
  fn v8__ValueDeserializer__CONSTRUCT(
    buf: *mut MaybeUninit<CxxValueDeserializer>,
    isolate: *mut Isolate,
    data: *const u8,
    size: usize,
    delegate: *mut CxxValueDeserializerDelegate,
  );

  fn v8__ValueDeserializer__DESTRUCT(this: *mut CxxValueDeserializer);

  fn v8__ValueDeserializer__TransferArrayBuffer(
    this: *mut CxxValueDeserializer,
    transfer_id: u32,
    array_buffer: Local<ArrayBuffer>,
  );

  fn v8__ValueDeserializer__SetSupportsLegacyWireFormat(
    this: *mut CxxValueDeserializer,
    supports_legacy_wire_format: bool,
  );

  fn v8__ValueDeserializer__ReadHeader(
    this: *mut CxxValueDeserializer,
    context: Local<Context>,
  ) -> MaybeBool;

  fn v8__ValueDeserializer__ReadValue(
    this: *mut CxxValueDeserializer,
    context: Local<Context>,
  ) -> *const Value;

  fn v8__ValueDeserializer__ReadUint32(
    this: *mut CxxValueDeserializer,
    value: *mut u32,
  ) -> bool;

  fn v8__ValueDeserializer__ReadUint64(
    this: *mut CxxValueDeserializer,
    value: *mut u64,
  ) -> bool;

  fn v8__ValueDeserializer__ReadDouble(
    this: *mut CxxValueDeserializer,
    value: *mut f64,
  ) -> bool;

  fn v8__ValueDeserializer__ReadRawBytes(
    this: *mut CxxValueDeserializer,
    length: usize,
    data: *mut *const c_void,
  ) -> bool;
}

/// The ValueDeserializerImpl trait allows for
/// custom callback functions used by v8.
pub trait ValueDeserializerImpl: GarbageCollected {
  fn read_host_object<'s>(
    &mut self,
    scope: &mut HandleScope<'s>,
    _value_deserializer: &mut dyn ValueDeserializerHelper,
  ) -> Option<Local<'s, Object>> {
    let msg =
      String::new(scope, "Deno deserializer: read_host_object not implemented")
        .unwrap();
    let exc = Exception::error(scope, msg);
    scope.throw_exception(exc);
    None
  }

  fn get_shared_array_buffer_from_id<'s>(
    &mut self,
    scope: &mut HandleScope<'s>,
    _transfer_id: u32,
  ) -> Option<Local<'s, SharedArrayBuffer>> {
    let msg = String::new(
      scope,
      "Deno deserializer: get_shared_array_buffer_from_id not implemented",
    )
    .unwrap();
    let exc = Exception::error(scope, msg);
    scope.throw_exception(exc);
    None
  }

  fn get_wasm_module_from_id<'s>(
    &mut self,
    scope: &mut HandleScope<'s>,
    _clone_id: u32,
  ) -> Option<Local<'s, WasmModuleObject>> {
    let msg = String::new(
      scope,
      "Deno deserializer: get_wasm_module_from_id not implemented",
    )
    .unwrap();
    let exc = Exception::error(scope, msg);
    scope.throw_exception(exc);
    None
  }
}

/// The ValueDeserializerHeap object contains all objects related to a
/// deserializer. This object has to be pinned to the heap because of the Cpp
/// pointers that have to remain valid. Moving this object would result in the
/// Cpp pointer to the delegate to become invalid and thus causing the delegate
/// callback to fail. Additionally the deserializer and implementation are also
/// pinned in memory because these have to be accessable from within the
/// delegate callback methods.
pub struct ValueDeserializerHeap<'a> {
  value_deserializer_impl: Box<dyn ValueDeserializerImpl + 'a>,
  cxx_value_deserializer: CxxValueDeserializer,
  cxx_value_deserializer_delegate: CxxValueDeserializerDelegate,
  context: TracedReference<Context>,
}

impl<'a> GarbageCollected for ValueDeserializerHeap<'a> {
  fn trace(&self, visitor: &crate::cppgc::Visitor) {
    self.value_deserializer_impl.trace(visitor);
    self.context.trace(visitor);
  }
}

impl<'a> ValueDeserializerHeap<'a> {
  fn get_cxx_value_deserializer_delegate_offset(
  ) -> FieldOffset<CxxValueDeserializerDelegate> {
    let buf = std::mem::MaybeUninit::<Self>::uninit();
    let delegate =
      unsafe { addr_of!((*buf.as_ptr()).cxx_value_deserializer_delegate) };
    FieldOffset::from_ptrs(buf.as_ptr(), delegate)
  }

  /// Starting from 'this' pointer a ValueDeserializerHeap ref can be created
  #[allow(dead_code)]
  pub unsafe fn dispatch(
    value_serializer_delegate: &CxxValueDeserializerDelegate,
  ) -> &Self {
    Self::get_cxx_value_deserializer_delegate_offset()
      .to_embedder::<Self>(value_serializer_delegate)
  }

  /// Starting from 'this' pointer the ValueDeserializerHeap mut ref can be
  /// created
  pub unsafe fn dispatch_mut(
    value_serializer_delegate: &mut CxxValueDeserializerDelegate,
  ) -> &mut Self {
    Self::get_cxx_value_deserializer_delegate_offset()
      .to_embedder_mut::<Self>(value_serializer_delegate)
  }
}

impl<'a> Drop for ValueDeserializerHeap<'a> {
  fn drop(&mut self) {
    unsafe {
      v8__ValueDeserializer__DESTRUCT(&mut self.cxx_value_deserializer)
    };
  }
}

/// Trait used for direct read from the deserialization buffer.
/// Mostly used by the read_host_object callback function in the
/// ValueDeserializerImpl trait to create custom deserialization logic.
pub trait ValueDeserializerHelper {
  fn get_cxx_value_deserializer(&mut self) -> *mut CxxValueDeserializer;

  fn read_header(&mut self, context: Local<Context>) -> Option<bool> {
    unsafe {
      ValueDeserializer::read_header_raw(
        self.get_cxx_value_deserializer(),
        context,
      )
    }
  }

  fn read_value<'s>(
    &mut self,
    context: Local<'s, Context>,
  ) -> Option<Local<'s, Value>> {
    unsafe {
      ValueDeserializer::read_value_raw(
        self.get_cxx_value_deserializer(),
        context,
      )
    }
  }

  fn read_uint32(&mut self, value: &mut u32) -> bool {
    unsafe {
      ValueDeserializer::read_uint32_raw(
        self.get_cxx_value_deserializer(),
        value,
      )
    }
  }

  fn read_uint64(&mut self, value: &mut u64) -> bool {
    unsafe {
      ValueDeserializer::read_uint64_raw(
        self.get_cxx_value_deserializer(),
        value,
      )
    }
  }

  fn read_double(&mut self, value: &mut f64) -> bool {
    unsafe {
      ValueDeserializer::read_double_raw(
        self.get_cxx_value_deserializer(),
        value,
      )
    }
  }

  fn read_raw_bytes(&mut self, length: usize) -> Option<&[u8]> {
    unsafe {
      ValueDeserializer::read_raw_bytes_raw(
        self.get_cxx_value_deserializer(),
        length,
      )
      .map(|data| std::slice::from_raw_parts(data, length))
    }
  }

  fn transfer_array_buffer(
    &mut self,
    transfer_id: u32,
    array_buffer: Local<ArrayBuffer>,
  ) {
    unsafe {
      ValueDeserializer::transfer_array_buffer_raw(
        self.get_cxx_value_deserializer(),
        transfer_id,
        array_buffer,
      )
    }
  }
}

impl ValueDeserializerHelper for CxxValueDeserializer {
  fn get_cxx_value_deserializer(&mut self) -> *mut CxxValueDeserializer {
    self
  }
}

impl<'a> ValueDeserializerHelper for ValueDeserializerHeap<'a> {
  fn get_cxx_value_deserializer(&mut self) -> *mut CxxValueDeserializer {
    &mut self.cxx_value_deserializer
  }
}

impl<'a> ValueDeserializerHelper for ValueDeserializer<'a> {
  fn get_cxx_value_deserializer(&mut self) -> *mut CxxValueDeserializer {
    &mut self.value_deserializer_heap.cxx_value_deserializer
  }
}

/// ValueDeserializer is a stack object used as entry-point for an owned and
/// pinned heap object ValueDeserializerHeap.
/// The 'a lifetime is the lifetime of the ValueDeserializerImpl implementation.
/// The 's lifetime is the lifetime of the HandleScope which is used to retrieve
/// a Local<'s, Context> for the CallbackScopes
pub struct ValueDeserializer<'a> {
  value_deserializer_heap: Pin<Box<ValueDeserializerHeap<'a>>>,
}

impl<'a> GarbageCollected for ValueDeserializer<'a> {
  fn trace(&self, visitor: &crate::cppgc::Visitor) {
    self.value_deserializer_heap.trace(visitor);
  }
}

impl<'a> ValueDeserializer<'a> {
  pub fn new<D: ValueDeserializerImpl + 'a>(
    scope: &mut HandleScope,
    value_deserializer_impl: Box<D>,
    data: &[u8],
  ) -> Self {
    let context = scope.get_current_context();
    // create dummy ValueDeserializerHeap and move to heap + pin to address
    let mut value_deserializer_heap = Box::pin(ValueDeserializerHeap {
      value_deserializer_impl,
      cxx_value_deserializer: CxxValueDeserializer {
        _cxx_vtable: CxxVTable(std::ptr::null()),
      },
      cxx_value_deserializer_delegate: CxxValueDeserializerDelegate {
        _cxx_vtable: CxxVTable(std::ptr::null()),
      },
      context: TracedReference::new(scope, context),
    });

    unsafe {
      v8__ValueDeserializer__Delegate__CONSTRUCT(
        &mut value_deserializer_heap.cxx_value_deserializer_delegate
          as *mut CxxValueDeserializerDelegate
          as *mut std::mem::MaybeUninit<CxxValueDeserializerDelegate>,
      );

      v8__ValueDeserializer__CONSTRUCT(
        &mut value_deserializer_heap.cxx_value_deserializer
          as *mut CxxValueDeserializer
          as *mut std::mem::MaybeUninit<CxxValueDeserializer>,
        scope.get_isolate_ptr(),
        data.as_ptr(),
        data.len(),
        &mut value_deserializer_heap.cxx_value_deserializer_delegate,
      );
    };

    ValueDeserializer {
      value_deserializer_heap,
    }
  }

  pub unsafe fn read_header_raw(
    de: *mut CxxValueDeserializer,
    context: Local<Context>,
  ) -> Option<bool> {
    v8__ValueDeserializer__ReadHeader(de, context).into()
  }

  pub unsafe fn read_value_raw(
    de: *mut CxxValueDeserializer,
    context: Local<Context>,
  ) -> Option<Local<Value>> {
    Local::from_raw(v8__ValueDeserializer__ReadValue(de, context))
  }

  pub unsafe fn read_uint32_raw(
    de: *mut CxxValueDeserializer,
    value: &mut u32,
  ) -> bool {
    v8__ValueDeserializer__ReadUint32(de, value)
  }

  pub unsafe fn read_uint64_raw(
    de: *mut CxxValueDeserializer,
    value: &mut u64,
  ) -> bool {
    v8__ValueDeserializer__ReadUint64(de, value)
  }

  pub unsafe fn read_double_raw(
    de: *mut CxxValueDeserializer,
    value: &mut f64,
  ) -> bool {
    v8__ValueDeserializer__ReadDouble(de, value)
  }

  pub unsafe fn read_raw_bytes_raw(
    de: *mut CxxValueDeserializer,
    length: usize,
  ) -> Option<*mut u8> {
    let mut data: *const c_void = std::ptr::null_mut();
    let ok = v8__ValueDeserializer__ReadRawBytes(de, length, &mut data);
    if ok {
      assert!(!data.is_null());
      Some(data.cast_mut().cast())
    } else {
      None
    }
  }

  pub unsafe fn transfer_array_buffer_raw(
    de: *mut CxxValueDeserializer,
    transfer_id: u32,
    array_buffer: Local<ArrayBuffer>,
  ) {
    v8__ValueDeserializer__TransferArrayBuffer(de, transfer_id, array_buffer)
  }
}

impl<'a> ValueDeserializer<'a> {
  pub fn set_supports_legacy_wire_format(
    &mut self,
    supports_legacy_wire_format: bool,
  ) {
    unsafe {
      v8__ValueDeserializer__SetSupportsLegacyWireFormat(
        &mut self.value_deserializer_heap.cxx_value_deserializer,
        supports_legacy_wire_format,
      );
    }
  }

  pub fn read_value<'t>(
    &mut self,
    context: Local<'t, Context>,
  ) -> Option<Local<'t, Value>> {
    self.value_deserializer_heap.read_value(context)
  }
}
