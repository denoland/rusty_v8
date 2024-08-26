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

use std::alloc::alloc;
use std::alloc::dealloc;
use std::alloc::realloc;
use std::alloc::Layout;
use std::mem::MaybeUninit;
use std::ptr::addr_of;
use std::sync::atomic::AtomicUsize;

use crate::support::CxxVTable;
use crate::support::FieldOffset;
use crate::support::MaybeBool;

use std::ffi::c_void;
use std::pin::Pin;

// Must be == sizeof(v8::ValueSerializer::Delegate),
// see v8__ValueSerializer__Delegate__CONSTRUCT().
#[repr(C)]
pub struct CxxValueSerializerDelegate {
  _cxx_vtable: CxxVTable,
}

#[no_mangle]
pub unsafe extern "C" fn v8__ValueSerializer__Delegate__ThrowDataCloneError(
  this: &CxxValueSerializerDelegate,
  message: Local<String>,
) {
  let value_serializer_heap = ValueSerializerHeap::dispatch(this);
  let scope = &mut CallbackScope::new(
    value_serializer_heap.isolate_ptr.as_mut().unwrap(),
  );
  let context = value_serializer_heap.context.get(scope).unwrap();
  let scope = &mut ContextScope::new(scope, context);
  value_serializer_heap
    .value_serializer_impl
    .throw_data_clone_error(scope, message)
}

#[no_mangle]
pub unsafe extern "C" fn v8__ValueSerializer__Delegate__HasCustomHostObject(
  this: &CxxValueSerializerDelegate,
  isolate: *mut Isolate,
) -> bool {
  let value_serializer_heap = ValueSerializerHeap::dispatch(this);
  value_serializer_heap
    .value_serializer_impl
    .has_custom_host_object(&mut *isolate)
}

#[no_mangle]
pub unsafe extern "C" fn v8__ValueSerializer__Delegate__IsHostObject(
  this: &CxxValueSerializerDelegate,
  isolate: *mut Isolate,
  object: Local<Object>,
) -> MaybeBool {
  let value_serializer_heap = ValueSerializerHeap::dispatch(this);
  let scope = &mut CallbackScope::new(isolate.as_mut().unwrap());
  let context = value_serializer_heap.context.get(scope).unwrap();
  let scope = &mut ContextScope::new(scope, context);

  MaybeBool::from(
    value_serializer_heap
      .value_serializer_impl
      .is_host_object(scope, object),
  )
}

#[no_mangle]
pub unsafe extern "C" fn v8__ValueSerializer__Delegate__WriteHostObject(
  this: &CxxValueSerializerDelegate,
  isolate: *mut Isolate,
  object: Local<Object>,
) -> MaybeBool {
  let value_serializer_heap = ValueSerializerHeap::dispatch(this);
  let scope = &mut CallbackScope::new(isolate.as_mut().unwrap());
  let context = value_serializer_heap.context.get(scope).unwrap();
  let scope = &mut ContextScope::new(scope, context);
  let value_serializer_impl =
    value_serializer_heap.value_serializer_impl.as_ref();
  MaybeBool::from(value_serializer_impl.write_host_object(
    scope,
    object,
    &value_serializer_heap.cxx_value_serializer,
  ))
}

#[no_mangle]
pub unsafe extern "C" fn v8__ValueSerializer__Delegate__GetSharedArrayBufferId(
  this: &CxxValueSerializerDelegate,
  isolate: *mut Isolate,
  shared_array_buffer: Local<SharedArrayBuffer>,
  clone_id: *mut u32,
) -> bool {
  let value_serializer_heap = ValueSerializerHeap::dispatch(this);
  let scope = &mut CallbackScope::new(isolate.as_mut().unwrap());
  let context = value_serializer_heap.context.get(scope).unwrap();
  let scope = &mut ContextScope::new(scope, context);
  match value_serializer_heap
    .value_serializer_impl
    .get_shared_array_buffer_id(scope, shared_array_buffer)
  {
    Some(x) => {
      *clone_id = x;
      true
    }
    None => false,
  }
}

#[no_mangle]
pub unsafe extern "C" fn v8__ValueSerializer__Delegate__GetWasmModuleTransferId(
  this: &CxxValueSerializerDelegate,
  isolate: *mut Isolate,
  module: Local<WasmModuleObject>,
  transfer_id: *mut u32,
) -> bool {
  let value_serializer_heap = ValueSerializerHeap::dispatch(this);
  let scope = &mut CallbackScope::new(isolate.as_mut().unwrap());
  let context = value_serializer_heap.context.get(scope).unwrap();
  let scope = &mut ContextScope::new(scope, context);
  match value_serializer_heap
    .value_serializer_impl
    .get_wasm_module_transfer_id(scope, module)
  {
    Some(x) => {
      *transfer_id = x;
      true
    }
    None => false,
  }
}

#[no_mangle]
pub unsafe extern "C" fn v8__ValueSerializer__Delegate__ReallocateBufferMemory(
  this: &CxxValueSerializerDelegate,
  old_buffer: *mut c_void,
  size: usize,
  actual_size: *mut usize,
) -> *mut c_void {
  let base = ValueSerializerHeap::dispatch(this);

  let buffer_size = base
    .buffer_size
    .swap(size, std::sync::atomic::Ordering::Release);
  let new_buffer = if old_buffer.is_null() {
    let layout = Layout::from_size_align(size, 1).unwrap();
    alloc(layout)
  } else {
    let old_layout = Layout::from_size_align(buffer_size, 1).unwrap();
    realloc(old_buffer as *mut _, old_layout, size)
  };

  *actual_size = size;
  new_buffer as *mut c_void
}

#[no_mangle]
pub unsafe extern "C" fn v8__ValueSerializer__Delegate__FreeBufferMemory(
  this: &mut CxxValueSerializerDelegate,
  buffer: *mut c_void,
) {
  let base = ValueSerializerHeap::dispatch(this);
  if !buffer.is_null() {
    let layout = Layout::from_size_align(
      base.buffer_size.load(std::sync::atomic::Ordering::Relaxed),
      1,
    )
    .unwrap();
    dealloc(buffer as *mut _, layout)
  };
}

extern "C" {
  fn v8__ValueSerializer__Delegate__CONSTRUCT(
    buf: *mut MaybeUninit<CxxValueSerializerDelegate>,
  );
}

// Must be == sizeof(v8::ValueSerializer), see v8__ValueSerializer__CONSTRUCT().
#[repr(C)]
pub struct CxxValueSerializer {
  _cxx_vtable: CxxVTable,
}

extern "C" {
  fn v8__ValueSerializer__CONSTRUCT(
    buf: *mut MaybeUninit<CxxValueSerializer>,
    isolate: *mut Isolate,
    delegate: *mut CxxValueSerializerDelegate,
  );

  fn v8__ValueSerializer__DESTRUCT(this: *mut CxxValueSerializer);

  fn v8__ValueSerializer__Release(
    this: *mut CxxValueSerializer,
    ptr: *mut *mut u8,
    size: *mut usize,
  );

  fn v8__ValueSerializer__TransferArrayBuffer(
    this: *mut CxxValueSerializer,
    transfer_id: u32,
    array_buffer: Local<ArrayBuffer>,
  );

  fn v8__ValueSerializer__WriteHeader(this: *mut CxxValueSerializer);
  fn v8__ValueSerializer__WriteValue(
    this: *mut CxxValueSerializer,
    context: Local<Context>,
    value: Local<Value>,
  ) -> MaybeBool;
  fn v8__ValueSerializer__WriteUint32(
    this: *mut CxxValueSerializer,
    value: u32,
  );
  fn v8__ValueSerializer__WriteUint64(
    this: *mut CxxValueSerializer,
    value: u64,
  );
  fn v8__ValueSerializer__WriteDouble(
    this: *mut CxxValueSerializer,
    value: f64,
  );
  fn v8__ValueSerializer__WriteRawBytes(
    this: *mut CxxValueSerializer,
    source: *const c_void,
    length: usize,
  );
  fn v8__ValueSerializer__SetTreatArrayBufferViewsAsHostObjects(
    this: *mut CxxValueSerializer,
    mode: bool,
  );
}

/// The ValueSerializerImpl trait allows for
/// custom callback functions used by v8.
pub trait ValueSerializerImpl: GarbageCollected {
  fn throw_data_clone_error<'s>(
    &self,
    scope: &mut HandleScope<'s>,
    message: Local<'s, String>,
  );

  fn has_custom_host_object(&self, _isolate: &mut Isolate) -> bool {
    false
  }

  fn is_host_object<'s>(
    &self,
    scope: &mut HandleScope<'s>,
    _object: Local<'s, Object>,
  ) -> Option<bool> {
    let msg =
      String::new(scope, "Deno serializer: is_host_object not implemented")
        .unwrap();
    let exc = Exception::error(scope, msg);
    scope.throw_exception(exc);
    None
  }

  fn write_host_object<'s>(
    &self,
    scope: &mut HandleScope<'s>,
    _object: Local<'s, Object>,
    _value_serializer: &dyn ValueSerializerHelper,
  ) -> Option<bool> {
    let msg =
      String::new(scope, "Deno serializer: write_host_object not implemented")
        .unwrap();
    let exc = Exception::error(scope, msg);
    scope.throw_exception(exc);
    None
  }

  fn get_shared_array_buffer_id<'s>(
    &self,
    _scope: &mut HandleScope<'s>,
    _shared_array_buffer: Local<'s, SharedArrayBuffer>,
  ) -> Option<u32> {
    None
  }

  fn get_wasm_module_transfer_id(
    &self,
    scope: &mut HandleScope<'_>,
    _module: Local<WasmModuleObject>,
  ) -> Option<u32> {
    let msg = String::new(
      scope,
      "Deno serializer: get_wasm_module_transfer_id not implemented",
    )
    .unwrap();
    let exc = Exception::error(scope, msg);
    scope.throw_exception(exc);
    None
  }
}

/// The ValueSerializerHeap object contains all objects related to serializer.
/// This object has to be pinned to the heap because of the Cpp pointers that
/// have to remain valid. Moving this object would result in the Cpp pointer
/// to the delegate to become invalid and thus causing the delegate callback
/// to fail. Additionally the serializer and implementation are also pinned
/// in memory because these have to be accessable from within the delegate
/// callback methods.
pub struct ValueSerializerHeap<'a> {
  value_serializer_impl: Box<dyn ValueSerializerImpl + 'a>,
  cxx_value_serializer_delegate: CxxValueSerializerDelegate,
  cxx_value_serializer: CxxValueSerializer,
  buffer_size: AtomicUsize,
  context: TracedReference<Context>,
  isolate_ptr: *mut Isolate,
}

impl<'a> crate::cppgc::GarbageCollected for ValueSerializerHeap<'a> {
  fn trace(&self, visitor: &crate::cppgc::Visitor) {
    self.value_serializer_impl.trace(visitor);
    self.context.trace(visitor);
  }
}

impl<'a> ValueSerializerHeap<'a> {
  fn get_cxx_value_serializer_delegate_offset(
  ) -> FieldOffset<CxxValueSerializerDelegate> {
    let buf = std::mem::MaybeUninit::<Self>::uninit();
    let delegate =
      unsafe { addr_of!((*buf.as_ptr()).cxx_value_serializer_delegate) };
    FieldOffset::from_ptrs(buf.as_ptr(), delegate)
  }

  /// Starting from 'this' pointer a ValueSerializerHeap ref can be created
  #[allow(dead_code)]
  pub unsafe fn dispatch(
    value_serializer_delegate: &CxxValueSerializerDelegate,
  ) -> &Self {
    Self::get_cxx_value_serializer_delegate_offset()
      .to_embedder::<Self>(value_serializer_delegate)
  }
}

impl<'a> Drop for ValueSerializerHeap<'a> {
  fn drop(&mut self) {
    unsafe { v8__ValueSerializer__DESTRUCT(&mut self.cxx_value_serializer) };
  }
}

fn cast_to_ptr<T>(x: &T) -> *mut T {
  x as *const T as *mut T
}

/// Trait used for direct write to the serialization buffer.
/// Mostly used by the write_host_object callback function in the
/// ValueSerializerImpl trait to create custom serialization logic.
pub trait ValueSerializerHelper {
  fn get_cxx_value_serializer(&self) -> &CxxValueSerializer;

  fn write_header(&self) {
    unsafe {
      v8__ValueSerializer__WriteHeader(cast_to_ptr(
        self.get_cxx_value_serializer(),
      ))
    };
  }

  fn write_value(
    &self,
    context: Local<Context>,
    value: Local<Value>,
  ) -> Option<bool> {
    unsafe {
      v8__ValueSerializer__WriteValue(
        cast_to_ptr(self.get_cxx_value_serializer()),
        context,
        value,
      )
    }
    .into()
  }

  fn write_uint32(&self, value: u32) {
    unsafe {
      v8__ValueSerializer__WriteUint32(
        cast_to_ptr(self.get_cxx_value_serializer()),
        value,
      )
    };
  }

  fn write_uint64(&self, value: u64) {
    unsafe {
      v8__ValueSerializer__WriteUint64(
        cast_to_ptr(self.get_cxx_value_serializer()),
        value,
      )
    };
  }

  fn write_double(&self, value: f64) {
    unsafe {
      v8__ValueSerializer__WriteDouble(
        cast_to_ptr(self.get_cxx_value_serializer()),
        value,
      )
    };
  }

  fn write_raw_bytes(&self, source: &[u8]) {
    unsafe {
      v8__ValueSerializer__WriteRawBytes(
        cast_to_ptr(self.get_cxx_value_serializer()),
        source.as_ptr() as *const _,
        source.len(),
      )
    };
  }

  fn transfer_array_buffer(
    &self,
    transfer_id: u32,
    array_buffer: Local<ArrayBuffer>,
  ) {
    unsafe {
      v8__ValueSerializer__TransferArrayBuffer(
        cast_to_ptr(self.get_cxx_value_serializer()),
        transfer_id,
        array_buffer,
      )
    };
  }

  fn set_treat_array_buffer_views_as_host_objects(&self, mode: bool) {
    unsafe {
      v8__ValueSerializer__SetTreatArrayBufferViewsAsHostObjects(
        cast_to_ptr(self.get_cxx_value_serializer()),
        mode,
      )
    };
  }
}

impl ValueSerializerHelper for CxxValueSerializer {
  fn get_cxx_value_serializer(&self) -> &CxxValueSerializer {
    self
  }
}

impl<'a> ValueSerializerHelper for ValueSerializerHeap<'a> {
  fn get_cxx_value_serializer(&self) -> &CxxValueSerializer {
    &self.cxx_value_serializer
  }
}

impl<'a> ValueSerializerHelper for ValueSerializer<'a> {
  fn get_cxx_value_serializer(&self) -> &CxxValueSerializer {
    &self.value_serializer_heap.cxx_value_serializer
  }
}

pub struct ValueSerializer<'a> {
  value_serializer_heap: Pin<Box<ValueSerializerHeap<'a>>>,
}

impl<'a> crate::cppgc::GarbageCollected for ValueSerializer<'a> {
  fn trace(&self, visitor: &crate::cppgc::Visitor) {
    self.value_serializer_heap.trace(visitor);
  }
}

/// ValueSerializer is a stack object used as entry-point for an owned and
/// pinned heap object ValueSerializerHeap.
/// The 'a lifetime is the lifetime of the ValueSerializerImpl implementation.
/// The 's lifetime is the lifetime of the HandleScope which is used to retrieve
/// a Local<'s, Context> for the CallbackScopes
impl<'a> ValueSerializer<'a> {
  pub fn new<D: ValueSerializerImpl + 'a>(
    scope: &mut HandleScope,
    value_serializer_impl: Box<D>,
  ) -> Self {
    let context = scope.get_current_context();
    // create dummy ValueSerializerHeap 'a, and move to heap + pin to address
    let value_serializer_heap_ptr =
      Box::into_raw(Box::new(ValueSerializerHeap {
        value_serializer_impl,
        cxx_value_serializer: CxxValueSerializer {
          _cxx_vtable: CxxVTable(std::ptr::null()),
        },
        cxx_value_serializer_delegate: CxxValueSerializerDelegate {
          _cxx_vtable: CxxVTable(std::ptr::null()),
        },
        buffer_size: AtomicUsize::new(0),
        context: TracedReference::new(scope, context),
        isolate_ptr: scope.get_isolate_ptr(),
      }));

    unsafe {
      let delegate_ptr = std::ptr::addr_of_mut!(
        (*value_serializer_heap_ptr).cxx_value_serializer_delegate
      );
      let serializer_ptr = std::ptr::addr_of_mut!(
        (*value_serializer_heap_ptr).cxx_value_serializer
      );
      v8__ValueSerializer__Delegate__CONSTRUCT(
        delegate_ptr
          .cast::<std::mem::MaybeUninit<CxxValueSerializerDelegate>>(),
      );

      v8__ValueSerializer__CONSTRUCT(
        serializer_ptr.cast::<std::mem::MaybeUninit<CxxValueSerializer>>(),
        scope.get_isolate_ptr(),
        delegate_ptr,
      );
    };

    // SAFETY: pointer from `Box::into_raw` is valid
    let value_serializer_heap =
      Pin::new(unsafe { Box::from_raw(value_serializer_heap_ptr) });

    Self {
      value_serializer_heap,
    }
  }
}

impl<'a> ValueSerializer<'a> {
  pub fn release(mut self) -> Vec<u8> {
    unsafe {
      let mut size: usize = 0;
      let mut ptr: *mut u8 = &mut 0;
      v8__ValueSerializer__Release(
        &mut self.value_serializer_heap.cxx_value_serializer,
        &mut ptr,
        &mut size,
      );
      Vec::from_raw_parts(
        ptr,
        size,
        self
          .value_serializer_heap
          .buffer_size
          .load(std::sync::atomic::Ordering::Relaxed),
      )
    }
  }

  pub fn write_value(
    &self,
    context: Local<Context>,
    value: Local<Value>,
  ) -> Option<bool> {
    self.value_serializer_heap.write_value(context, value)
  }
}
