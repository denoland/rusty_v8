// Copyright 2019-2021 the Deno authors. All rights reserved. MIT license.

use std::ffi::c_void;
use std::ptr::null_mut;

use crate::array_buffer::backing_store_deleter_callback;
use crate::support::SharedRef;
use crate::support::UniqueRef;
use crate::BackingStore;
use crate::BackingStoreDeleterCallback;
use crate::HandleScope;
use crate::Isolate;
use crate::Local;
use crate::SharedArrayBuffer;

extern "C" {
  fn v8__SharedArrayBuffer__New__with_byte_length(
    isolate: *mut Isolate,
    byte_length: usize,
  ) -> *const SharedArrayBuffer;
  fn v8__SharedArrayBuffer__New__with_backing_store(
    isolate: *mut Isolate,
    backing_store: *const SharedRef<BackingStore>,
  ) -> *const SharedArrayBuffer;
  fn v8__SharedArrayBuffer__ByteLength(this: *const SharedArrayBuffer)
    -> usize;
  fn v8__SharedArrayBuffer__GetBackingStore(
    this: *const SharedArrayBuffer,
  ) -> SharedRef<BackingStore>;
  fn v8__SharedArrayBuffer__NewBackingStore__with_byte_length(
    isolate: *mut Isolate,
    byte_length: usize,
  ) -> *mut BackingStore;
  fn v8__SharedArrayBuffer__NewBackingStore__with_data(
    data: *mut c_void,
    byte_length: usize,
    deleter: BackingStoreDeleterCallback,
    deleter_data: *mut c_void,
  ) -> *mut BackingStore;
}

impl SharedArrayBuffer {
  /// Create a new SharedArrayBuffer. Allocate |byte_length| bytes.
  /// Allocated memory will be owned by a created SharedArrayBuffer and
  /// will be deallocated when it is garbage-collected,
  /// unless the object is externalized.
  pub fn new<'s>(
    scope: &mut HandleScope<'s>,
    byte_length: usize,
  ) -> Option<Local<'s, SharedArrayBuffer>> {
    unsafe {
      scope.cast_local(|sd| {
        v8__SharedArrayBuffer__New__with_byte_length(
          sd.get_isolate_ptr(),
          byte_length,
        )
      })
    }
  }

  pub fn with_backing_store<'s>(
    scope: &mut HandleScope<'s>,
    backing_store: &SharedRef<BackingStore>,
  ) -> Local<'s, SharedArrayBuffer> {
    unsafe {
      scope.cast_local(|sd| {
        v8__SharedArrayBuffer__New__with_backing_store(
          sd.get_isolate_ptr(),
          backing_store,
        )
      })
    }
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

  /// Returns a new standalone BackingStore that is allocated using the array
  /// buffer allocator of the isolate. The result can be later passed to
  /// ArrayBuffer::New.
  ///
  /// If the allocator returns nullptr, then the function may cause GCs in the
  /// given isolate and re-try the allocation. If GCs do not help, then the
  /// function will crash with an out-of-memory error.
  pub fn new_backing_store(
    scope: &mut Isolate,
    byte_length: usize,
  ) -> UniqueRef<BackingStore> {
    unsafe {
      UniqueRef::from_raw(
        v8__SharedArrayBuffer__NewBackingStore__with_byte_length(
          scope,
          byte_length,
        ),
      )
    }
  }

  /// Returns a new standalone BackingStore that takes over the ownership of
  /// the given buffer.
  ///
  /// The destructor of the BackingStore frees owned buffer memory.
  ///
  /// The result can be later passed to SharedArrayBuffer::New. The raw pointer
  /// to the buffer must not be passed again to any V8 API function.
  pub fn new_backing_store_from_boxed_slice(
    data: Box<[u8]>,
  ) -> UniqueRef<BackingStore> {
    let byte_length = data.len();
    let data_ptr = Box::into_raw(data) as *mut c_void;
    unsafe {
      UniqueRef::from_raw(v8__SharedArrayBuffer__NewBackingStore__with_data(
        data_ptr,
        byte_length,
        backing_store_deleter_callback,
        null_mut(),
      ))
    }
  }
}
