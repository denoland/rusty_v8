use crate::ArrayBuffer;
use crate::Context;
use crate::Exception;
use crate::HandleScope;
use crate::Isolate;
use crate::Local;
use crate::Object;
use crate::SharedArrayBuffer;
use crate::String;
use crate::Value;
use crate::WasmModuleObject;

use std::alloc::alloc;
use std::alloc::dealloc;
use std::alloc::realloc;
use std::alloc::Layout;
use std::mem::MaybeUninit;

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
  this: &mut CxxValueSerializerDelegate,
  message: Local<String>,
) {
  let value_serializer_heap = ValueSerializerHeap::dispatch_mut(this);
  let scope =
    &mut crate::scope::CallbackScope::new(value_serializer_heap.context);
  value_serializer_heap
    .value_serializer_impl
    .as_mut()
    .throw_data_clone_error(scope, message)
}

#[no_mangle]
pub unsafe extern "C" fn v8__ValueSerializer__Delegate__WriteHostObject(
  this: &mut CxxValueSerializerDelegate,
  _isolate: *mut Isolate,
  object: Local<Object>,
) -> MaybeBool {
  let value_serializer_heap = ValueSerializerHeap::dispatch_mut(this);
  let scope =
    &mut crate::scope::CallbackScope::new(value_serializer_heap.context);
  let value_serializer_impl =
    value_serializer_heap.value_serializer_impl.as_mut();
  MaybeBool::from(value_serializer_impl.write_host_object(
    scope,
    object,
    &mut value_serializer_heap.cxx_value_serializer,
  ))
}

#[no_mangle]
pub unsafe extern "C" fn v8__ValueSerializer__Delegate__GetSharedArrayBufferId(
  this: &mut CxxValueSerializerDelegate,
  _isolate: *mut Isolate,
  shared_array_buffer: Local<SharedArrayBuffer>,
  clone_id: *mut u32,
) -> bool {
  let value_serializer_heap = ValueSerializerHeap::dispatch_mut(this);
  let scope =
    &mut crate::scope::CallbackScope::new(value_serializer_heap.context);
  match value_serializer_heap
    .value_serializer_impl
    .as_mut()
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
  this: &mut CxxValueSerializerDelegate,
  _isolate: *mut Isolate,
  module: Local<WasmModuleObject>,
  transfer_id: *mut u32,
) -> bool {
  let value_serializer_heap = ValueSerializerHeap::dispatch_mut(this);
  let scope =
    &mut crate::scope::CallbackScope::new(value_serializer_heap.context);
  match value_serializer_heap
    .value_serializer_impl
    .as_mut()
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
  this: &mut CxxValueSerializerDelegate,
  old_buffer: *mut c_void,
  size: usize,
  actual_size: *mut usize,
) -> *mut c_void {
  let base = ValueSerializerHeap::dispatch_mut(this);

  let new_buffer = if old_buffer.is_null() {
    let layout = Layout::from_size_align(size, 1).unwrap();
    alloc(layout)
  } else {
    let old_layout = Layout::from_size_align(base.buffer_size, 1).unwrap();
    realloc(old_buffer as *mut _, old_layout, size)
  };

  base.buffer_size = size;

  *actual_size = size;
  new_buffer as *mut c_void
}

#[no_mangle]
pub unsafe extern "C" fn v8__ValueSerializer__Delegate__FreeBufferMemory(
  this: &mut CxxValueSerializerDelegate,
  buffer: *mut c_void,
) {
  let base = ValueSerializerHeap::dispatch_mut(this);
  if !buffer.is_null() {
    let layout = Layout::from_size_align(base.buffer_size, 1).unwrap();
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
}

/// The ValueSerializerImpl trait allows for
/// custom callback functions used by v8.
pub trait ValueSerializerImpl {
  fn throw_data_clone_error<'s>(
    &mut self,
    scope: &mut HandleScope<'s>,
    message: Local<'s, String>,
  );

  #[allow(unused_variables)]
  fn write_host_object<'s>(
    &mut self,
    scope: &mut HandleScope<'s>,
    object: Local<'s, Object>,
    value_serializer: &mut dyn ValueSerializerHelper,
  ) -> Option<bool> {
    let msg =
      String::new(scope, "Deno serializer: write_host_object not implemented")
        .unwrap();
    let exc = Exception::error(scope, msg);
    scope.throw_exception(exc);
    None
  }

  #[allow(unused_variables)]
  fn get_shared_array_buffer_id<'s>(
    &mut self,
    scope: &mut HandleScope<'s>,
    shared_array_buffer: Local<'s, SharedArrayBuffer>,
  ) -> Option<u32> {
    let msg = String::new(
      scope,
      "Deno serializer: get_shared_array_buffer_id not implemented",
    )
    .unwrap();
    let exc = Exception::error(scope, msg);
    scope.throw_exception(exc);
    None
  }

  #[allow(unused_variables)]
  fn get_wasm_module_transfer_id(
    &mut self,
    scope: &mut HandleScope<'_>,
    module: Local<WasmModuleObject>,
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
pub struct ValueSerializerHeap<'a, 's> {
  value_serializer_impl: Box<dyn ValueSerializerImpl + 'a>,
  cxx_value_serializer_delegate: CxxValueSerializerDelegate,
  cxx_value_serializer: CxxValueSerializer,
  buffer_size: usize,
  context: Local<'s, Context>,
}

impl<'a, 's> ValueSerializerHeap<'a, 's> {
  fn get_cxx_value_serializer_delegate_offset(
  ) -> FieldOffset<CxxValueSerializerDelegate> {
    let buf = std::mem::MaybeUninit::<Self>::uninit();
    FieldOffset::from_ptrs(buf.as_ptr(), unsafe {
      &(*buf.as_ptr()).cxx_value_serializer_delegate
    })
  }

  /// Starting from 'this' pointer a ValueSerializerHeap ref can be created
  pub unsafe fn dispatch(
    value_serializer_delegate: &'s CxxValueSerializerDelegate,
  ) -> &Self {
    Self::get_cxx_value_serializer_delegate_offset()
      .to_embedder::<Self>(value_serializer_delegate)
  }

  /// Starting from 'this' pointer the ValueSerializerHeap mut ref can be
  /// created
  pub unsafe fn dispatch_mut(
    value_serializer_delegate: &'s mut CxxValueSerializerDelegate,
  ) -> &mut Self {
    Self::get_cxx_value_serializer_delegate_offset()
      .to_embedder_mut::<Self>(value_serializer_delegate)
  }
}

impl<'a, 's> Drop for ValueSerializerHeap<'a, 's> {
  fn drop(&mut self) {
    unsafe { v8__ValueSerializer__DESTRUCT(&mut self.cxx_value_serializer) };
  }
}

/// Trait used for direct write to the serialization buffer.
/// Mostly used by the write_host_object callback function in the
/// ValueSerializerImpl trait to create custom serialization logic.
pub trait ValueSerializerHelper {
  fn get_cxx_value_serializer(&mut self) -> &mut CxxValueSerializer;

  fn write_header(&mut self) {
    unsafe {
      v8__ValueSerializer__WriteHeader(self.get_cxx_value_serializer())
    };
  }

  fn write_value(
    &mut self,
    context: Local<Context>,
    value: Local<Value>,
  ) -> Option<bool> {
    unsafe {
      v8__ValueSerializer__WriteValue(
        self.get_cxx_value_serializer(),
        context,
        value,
      )
    }
    .into()
  }

  fn write_uint32(&mut self, value: u32) {
    unsafe {
      v8__ValueSerializer__WriteUint32(self.get_cxx_value_serializer(), value)
    };
  }

  fn write_uint64(&mut self, value: u64) {
    unsafe {
      v8__ValueSerializer__WriteUint64(self.get_cxx_value_serializer(), value)
    };
  }

  fn write_double(&mut self, value: f64) {
    unsafe {
      v8__ValueSerializer__WriteDouble(self.get_cxx_value_serializer(), value)
    };
  }

  fn write_raw_bytes(&mut self, source: &[u8]) {
    unsafe {
      v8__ValueSerializer__WriteRawBytes(
        self.get_cxx_value_serializer(),
        source.as_ptr() as *const _,
        source.len(),
      )
    };
  }

  fn transfer_array_buffer(
    &mut self,
    transfer_id: u32,
    array_buffer: Local<ArrayBuffer>,
  ) {
    unsafe {
      v8__ValueSerializer__TransferArrayBuffer(
        self.get_cxx_value_serializer(),
        transfer_id,
        array_buffer,
      )
    };
  }
}

impl ValueSerializerHelper for CxxValueSerializer {
  fn get_cxx_value_serializer(&mut self) -> &mut CxxValueSerializer {
    self
  }
}

impl<'a, 's> ValueSerializerHelper for ValueSerializerHeap<'a, 's> {
  fn get_cxx_value_serializer(&mut self) -> &mut CxxValueSerializer {
    &mut self.cxx_value_serializer
  }
}

impl<'a, 's> ValueSerializerHelper for ValueSerializer<'a, 's> {
  fn get_cxx_value_serializer(&mut self) -> &mut CxxValueSerializer {
    &mut (*self.value_serializer_heap).cxx_value_serializer
  }
}

pub struct ValueSerializer<'a, 's> {
  value_serializer_heap: Pin<Box<ValueSerializerHeap<'a, 's>>>,
}

/// ValueSerializer is a stack object used as entry-point for an owned and
/// pinned heap object ValueSerializerHeap.
/// The 'a lifetime is the lifetime of the ValueSerializerImpl implementation.
/// The 's lifetime is the lifetime of the HandleScope which is used to retrieve
/// a Local<'s, Context> for the CallbackScopes
impl<'a, 's> ValueSerializer<'a, 's> {
  pub fn new<D: ValueSerializerImpl + 'a>(
    scope: &mut HandleScope<'s>,
    value_serializer_impl: Box<D>,
  ) -> Self {
    // create dummy ValueSerializerHeap 'a, and move to heap + pin to address
    let mut value_serializer_heap = Box::pin(ValueSerializerHeap {
      value_serializer_impl,
      cxx_value_serializer: CxxValueSerializer {
        _cxx_vtable: CxxVTable {
          0: std::ptr::null(),
        },
      },
      cxx_value_serializer_delegate: CxxValueSerializerDelegate {
        _cxx_vtable: CxxVTable {
          0: std::ptr::null(),
        },
      },
      buffer_size: 0,
      context: scope.get_current_context(),
    });

    unsafe {
      v8__ValueSerializer__Delegate__CONSTRUCT(core::mem::transmute(
        &mut (*value_serializer_heap).cxx_value_serializer_delegate,
      ));

      v8__ValueSerializer__CONSTRUCT(
        core::mem::transmute(
          &mut (*value_serializer_heap).cxx_value_serializer,
        ),
        scope.get_isolate_ptr(),
        &mut (*value_serializer_heap).cxx_value_serializer_delegate,
      );
    };

    Self {
      value_serializer_heap,
    }
  }
}

impl<'a, 's> ValueSerializer<'a, 's> {
  pub fn release(mut self) -> Vec<u8> {
    unsafe {
      let mut size: usize = 0;
      let mut ptr: *mut u8 = &mut 0;
      v8__ValueSerializer__Release(
        &mut (*self.value_serializer_heap).cxx_value_serializer,
        &mut ptr,
        &mut size,
      );
      Vec::from_raw_parts(
        ptr as *mut u8,
        size,
        (*self.value_serializer_heap).buffer_size,
      )
    }
  }

  pub fn write_value(
    &mut self,
    context: Local<Context>,
    value: Local<Value>,
  ) -> Option<bool> {
    (*self.value_serializer_heap).write_value(context, value)
  }
}
