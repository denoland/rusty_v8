use crate::support::SharedRef;
use crate::BackingStore;
use crate::Isolate;
use crate::Local;
use crate::SharedArrayBuffer;
use crate::ToLocal;

extern "C" {
  fn v8__SharedArrayBuffer__New(
    isolate: *mut Isolate,
    byte_length: usize,
  ) -> *mut SharedArrayBuffer;
  fn v8__SharedArrayBuffer__New__DEPRECATED(
    isolate: *mut Isolate,
    data_ptr: *mut std::ffi::c_void,
    data_length: usize,
  ) -> *mut SharedArrayBuffer;
  fn v8__SharedArrayBuffer__ByteLength(
    self_: *const SharedArrayBuffer,
  ) -> usize;
  fn v8__SharedArrayBuffer__GetBackingStore(
    self_: *const SharedArrayBuffer,
  ) -> SharedRef<BackingStore>;
}

impl SharedArrayBuffer {
  /// Create a new SharedArrayBuffer. Allocate |byte_length| bytes.
  /// Allocated memory will be owned by a created SharedArrayBuffer and
  /// will be deallocated when it is garbage-collected,
  /// unless the object is externalized.
  pub fn new<'sc>(
    scope: &mut impl ToLocal<'sc>,
    byte_length: usize,
  ) -> Option<Local<'sc, SharedArrayBuffer>> {
    unsafe {
      Local::from_raw(v8__SharedArrayBuffer__New(scope.isolate(), byte_length))
    }
  }

  /// DEPRECATED
  /// Use the version that takes a BackingStore.
  /// See http://crbug.com/v8/9908.
  ///
  ///
  /// Create a new SharedArrayBuffer over an existing memory block.  The created
  /// array buffer is immediately in externalized state unless otherwise
  /// specified. The memory block will not be reclaimed when a created
  /// SharedArrayBuffer is garbage-collected.
  #[allow(non_snake_case)]
  pub unsafe fn new_DEPRECATED<'sc>(
    scope: &mut impl ToLocal<'sc>,
    data_ptr: *mut std::ffi::c_void,
    data_length: usize,
  ) -> Local<'sc, SharedArrayBuffer> {
    Local::from_raw(v8__SharedArrayBuffer__New__DEPRECATED(
      scope.isolate(),
      data_ptr,
      data_length,
    ))
    .unwrap()
  }

  /// Data length in bytes.
  pub fn byte_length(&self) -> usize {
    unsafe { v8__SharedArrayBuffer__ByteLength(self) }
  }

  /// Get a shared pointer to the backing store of this array buffer. This
  /// pointer coordinates the lifetime management of the internal storage
  /// with any live ArrayBuffers on the heap, even across isolates. The embedder
  /// should not attempt to manage lifetime of the storage through other means.
  pub fn get_backing_store(&self) -> SharedRef<BackingStore> {
    unsafe { v8__SharedArrayBuffer__GetBackingStore(self) }
  }
}
