use crate::array_buffer::backing_store_deleter_callback;
use crate::support::SharedRef;
use crate::BackingStore;
use crate::BackingStoreDeleterCallback;
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
  fn v8__SharedArrayBuffer__NewBackingStore_FromRaw(
    data: *mut std::ffi::c_void,
    byte_length: usize,
    deleter: BackingStoreDeleterCallback,
  ) -> SharedRef<BackingStore>;
  fn v8__SharedArrayBuffer__New__backing_store(
    isolate: *mut Isolate,
    backing_store: *mut SharedRef<BackingStore>,
  ) -> *mut SharedArrayBuffer;
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

  pub fn new_with_backing_store<'sc>(
    scope: &mut impl ToLocal<'sc>,
    backing_store: &mut SharedRef<BackingStore>,
  ) -> Local<'sc, SharedArrayBuffer> {
    let isolate = scope.isolate();
    let ptr = unsafe {
      v8__SharedArrayBuffer__New__backing_store(isolate, &mut *backing_store)
    };
    unsafe { scope.to_local(ptr) }.unwrap()
  }

  /// Returns a new standalone BackingStore that takes over the ownership of
  /// the given buffer. The destructor of the BackingStore invokes the given
  /// deleter callback.
  ///
  /// The result can be later passed to SharedArrayBuffer::New. The raw pointer
  /// to the buffer must not be passed again to any V8 API function.
  pub unsafe fn new_backing_store_from_boxed_slice(
    data: Box<[u8]>,
  ) -> SharedRef<BackingStore> {
    let byte_length = data.len();
    let data_ptr = Box::into_raw(data) as *mut std::ffi::c_void;
    v8__SharedArrayBuffer__NewBackingStore_FromRaw(
      data_ptr,
      byte_length,
      backing_store_deleter_callback,
    )
  }
}
