// Copyright 2019-2020 the Deno authors. All rights reserved. MIT license.

use std::ffi::c_void;
use std::ops::Deref;
use std::ops::DerefMut;
use std::ptr::null_mut;
use std::slice;

use crate::support::long;
use crate::support::Opaque;
use crate::support::Shared;
use crate::support::SharedRef;
use crate::support::UniqueRef;
use crate::ArrayBuffer;
use crate::InIsolate;
use crate::Isolate;
use crate::Local;
use crate::ToLocal;

extern "C" {
  fn v8__ArrayBuffer__Allocator__NewDefaultAllocator() -> *mut Allocator;
  fn v8__ArrayBuffer__Allocator__DELETE(this: *mut Allocator);
  fn v8__ArrayBuffer__New__with_byte_length(
    isolate: *mut Isolate,
    byte_length: usize,
  ) -> *const ArrayBuffer;
  fn v8__ArrayBuffer__New__with_backing_store(
    isolate: *mut Isolate,
    backing_store: *const SharedRef<BackingStore>,
  ) -> *const ArrayBuffer;
  fn v8__ArrayBuffer__ByteLength(this: *const ArrayBuffer) -> usize;
  fn v8__ArrayBuffer__GetBackingStore(
    this: *const ArrayBuffer,
  ) -> SharedRef<BackingStore>;
  fn v8__ArrayBuffer__NewBackingStore__with_byte_length(
    isolate: *mut Isolate,
    byte_length: usize,
  ) -> *mut BackingStore;
  fn v8__ArrayBuffer__NewBackingStore__with_data(
    data: *mut c_void,
    byte_length: usize,
    deleter: BackingStoreDeleterCallback,
    deleter_data: *mut c_void,
  ) -> *mut BackingStore;

  fn v8__BackingStore__Data(this: *const BackingStore) -> *mut c_void;
  fn v8__BackingStore__ByteLength(this: *const BackingStore) -> usize;
  fn v8__BackingStore__IsShared(this: *const BackingStore) -> bool;
  fn v8__BackingStore__DELETE(this: *mut BackingStore);

  fn std__shared_ptr__v8__BackingStore__COPY(
    ptr: *const SharedRef<BackingStore>,
  ) -> SharedRef<BackingStore>;
  fn std__shared_ptr__v8__BackingStore__CONVERT__std__unique_ptr(
    unique: UniqueRef<BackingStore>,
  ) -> SharedRef<BackingStore>;
  fn std__shared_ptr__v8__BackingStore__get(
    ptr: *const SharedRef<BackingStore>,
  ) -> *mut BackingStore;
  fn std__shared_ptr__v8__BackingStore__reset(
    ptr: *mut SharedRef<BackingStore>,
  );
  fn std__shared_ptr__v8__BackingStore__use_count(
    ptr: *const SharedRef<BackingStore>,
  ) -> long;

  fn std__shared_ptr__v8__ArrayBuffer__Allocator__COPY(
    ptr: *const SharedRef<Allocator>,
  ) -> SharedRef<Allocator>;
  fn std__shared_ptr__v8__ArrayBuffer__Allocator__CONVERT__std__unique_ptr(
    unique: UniqueRef<Allocator>,
  ) -> SharedRef<Allocator>;
  fn std__shared_ptr__v8__ArrayBuffer__Allocator__get(
    ptr: *const SharedRef<Allocator>,
  ) -> *mut Allocator;
  fn std__shared_ptr__v8__ArrayBuffer__Allocator__reset(
    ptr: *mut SharedRef<Allocator>,
  );
  fn std__shared_ptr__v8__ArrayBuffer__Allocator__use_count(
    ptr: *const SharedRef<Allocator>,
  ) -> long;
}

/// A thread-safe allocator that V8 uses to allocate |ArrayBuffer|'s memory.
/// The allocator is a global V8 setting. It has to be set via
/// Isolate::CreateParams.
///
/// Memory allocated through this allocator by V8 is accounted for as external
/// memory by V8. Note that V8 keeps track of the memory for all internalized
/// |ArrayBuffer|s. Responsibility for tracking external memory (using
/// Isolate::AdjustAmountOfExternalAllocatedMemory) is handed over to the
/// embedder upon externalization and taken over upon internalization (creating
/// an internalized buffer from an existing buffer).
///
/// Note that it is unsafe to call back into V8 from any of the allocator
/// functions.
///
/// This is called v8::ArrayBuffer::Allocator in C++. Rather than use the
/// namespace array_buffer, which will contain only the Allocator we opt in Rust
/// to allow it to live in the top level: v8::Allocator
#[repr(C)]
pub struct Allocator(Opaque);

impl Shared for Allocator {
  fn clone(ptr: *const SharedRef<Self>) -> SharedRef<Self> {
    unsafe { std__shared_ptr__v8__ArrayBuffer__Allocator__COPY(ptr) }
  }
  fn from_unique(unique: UniqueRef<Self>) -> SharedRef<Self> {
    unsafe {
      std__shared_ptr__v8__ArrayBuffer__Allocator__CONVERT__std__unique_ptr(
        unique,
      )
    }
  }
  fn deref(ptr: *const SharedRef<Self>) -> *mut Self {
    unsafe { std__shared_ptr__v8__ArrayBuffer__Allocator__get(ptr) }
  }
  fn reset(ptr: *mut SharedRef<Self>) {
    unsafe { std__shared_ptr__v8__ArrayBuffer__Allocator__reset(ptr) }
  }
  fn use_count(ptr: *const SharedRef<Self>) -> long {
    unsafe { std__shared_ptr__v8__ArrayBuffer__Allocator__use_count(ptr) }
  }
}

/// malloc/free based convenience allocator.
pub fn new_default_allocator() -> SharedRef<Allocator> {
  unsafe {
    UniqueRef::from_raw(v8__ArrayBuffer__Allocator__NewDefaultAllocator())
  }
  .make_shared()
}

#[test]
fn test_default_allocator() {
  new_default_allocator();
}

impl Drop for Allocator {
  fn drop(&mut self) {
    unsafe { v8__ArrayBuffer__Allocator__DELETE(self) };
  }
}

pub type BackingStoreDeleterCallback = unsafe extern "C" fn(
  data: *mut c_void,
  byte_length: usize,
  deleter_data: *mut c_void,
);

pub unsafe extern "C" fn backing_store_deleter_callback(
  data: *mut c_void,
  _byte_length: usize,
  _deleter_data: *mut c_void,
) {
  let b = Box::from_raw(data);
  drop(b)
}

/// A wrapper around the backing store (i.e. the raw memory) of an array buffer.
/// See a document linked in http://crbug.com/v8/9908 for more information.
///
/// The allocation and destruction of backing stores is generally managed by
/// V8. Clients should always use standard C++ memory ownership types (i.e.
/// std::unique_ptr and std::shared_ptr) to manage lifetimes of backing stores
/// properly, since V8 internal objects may alias backing stores.
///
/// This object does not keep the underlying |ArrayBuffer::Allocator| alive by
/// default. Use Isolate::CreateParams::array_buffer_allocator_shared when
/// creating the Isolate to make it hold a reference to the allocator itself.
#[repr(C)]
pub struct BackingStore([usize; 6]);

unsafe impl Send for BackingStore {}

impl BackingStore {
  /// Return a pointer to the beginning of the memory block for this backing
  /// store. The pointer is only valid as long as this backing store object
  /// lives.
  pub fn data(&self) -> *mut c_void {
    unsafe { v8__BackingStore__Data(self as *const _ as *mut Self) }
  }

  /// The length (in bytes) of this backing store.
  pub fn byte_length(&self) -> usize {
    unsafe { v8__BackingStore__ByteLength(self) }
  }

  /// Indicates whether the backing store was created for an ArrayBuffer or
  /// a SharedArrayBuffer.
  pub fn is_shared(&self) -> bool {
    unsafe { v8__BackingStore__IsShared(self) }
  }
}

impl Deref for BackingStore {
  type Target = [u8];

  /// Returns a [u8] slice refencing the data in the backing store.
  fn deref(&self) -> &[u8] {
    unsafe { slice::from_raw_parts(self.data() as *mut u8, self.byte_length()) }
  }
}

impl DerefMut for BackingStore {
  /// Returns a mutable [u8] slice refencing the data in the backing store.
  fn deref_mut(&mut self) -> &mut [u8] {
    unsafe {
      slice::from_raw_parts_mut(self.data() as *mut u8, self.byte_length())
    }
  }
}

impl Drop for BackingStore {
  fn drop(&mut self) {
    unsafe { v8__BackingStore__DELETE(self) };
  }
}

impl Shared for BackingStore {
  fn clone(ptr: *const SharedRef<Self>) -> SharedRef<Self> {
    unsafe { std__shared_ptr__v8__BackingStore__COPY(ptr) }
  }
  fn from_unique(unique: UniqueRef<Self>) -> SharedRef<Self> {
    unsafe {
      std__shared_ptr__v8__BackingStore__CONVERT__std__unique_ptr(unique)
    }
  }
  fn deref(ptr: *const SharedRef<Self>) -> *mut Self {
    unsafe { std__shared_ptr__v8__BackingStore__get(ptr) }
  }
  fn reset(ptr: *mut SharedRef<Self>) {
    unsafe { std__shared_ptr__v8__BackingStore__reset(ptr) }
  }
  fn use_count(ptr: *const SharedRef<Self>) -> long {
    unsafe { std__shared_ptr__v8__BackingStore__use_count(ptr) }
  }
}

impl ArrayBuffer {
  /// Create a new ArrayBuffer. Allocate |byte_length| bytes.
  /// Allocated memory will be owned by a created ArrayBuffer and
  /// will be deallocated when it is garbage-collected,
  /// unless the object is externalized.
  pub fn new<'sc>(
    scope: &mut impl ToLocal<'sc>,
    byte_length: usize,
  ) -> Local<'sc, ArrayBuffer> {
    let isolate = scope.isolate();
    let ptr =
      unsafe { v8__ArrayBuffer__New__with_byte_length(isolate, byte_length) };
    unsafe { scope.to_local(ptr) }.unwrap()
  }

  pub fn with_backing_store<'sc>(
    scope: &mut impl ToLocal<'sc>,
    backing_store: &SharedRef<BackingStore>,
  ) -> Local<'sc, ArrayBuffer> {
    let isolate = scope.isolate();
    let ptr = unsafe {
      v8__ArrayBuffer__New__with_backing_store(isolate, backing_store)
    };
    unsafe { scope.to_local(ptr) }.unwrap()
  }

  /// Data length in bytes.
  pub fn byte_length(&self) -> usize {
    unsafe { v8__ArrayBuffer__ByteLength(self) }
  }

  /// Get a shared pointer to the backing store of this array buffer. This
  /// pointer coordinates the lifetime management of the internal storage
  /// with any live ArrayBuffers on the heap, even across isolates. The embedder
  /// should not attempt to manage lifetime of the storage through other means.
  pub fn get_backing_store(&self) -> SharedRef<BackingStore> {
    unsafe { v8__ArrayBuffer__GetBackingStore(self) }
  }

  /// Returns a new standalone BackingStore that is allocated using the array
  /// buffer allocator of the isolate. The result can be later passed to
  /// ArrayBuffer::New.
  ///
  /// If the allocator returns nullptr, then the function may cause GCs in the
  /// given isolate and re-try the allocation. If GCs do not help, then the
  /// function will crash with an out-of-memory error.
  pub fn new_backing_store(
    scope: &mut impl InIsolate,
    byte_length: usize,
  ) -> UniqueRef<BackingStore> {
    unsafe {
      UniqueRef::from_raw(v8__ArrayBuffer__NewBackingStore__with_byte_length(
        scope.isolate(),
        byte_length,
      ))
    }
  }

  /// Returns a new standalone BackingStore that takes over the ownership of
  /// the given buffer.
  ///
  /// The destructor of the BackingStore frees owned buffer memory.
  ///
  /// The result can be later passed to ArrayBuffer::New. The raw pointer
  /// to the buffer must not be passed again to any V8 API function.
  pub fn new_backing_store_from_boxed_slice(
    data: Box<[u8]>,
  ) -> UniqueRef<BackingStore> {
    let byte_length = data.len();
    let data_ptr = Box::into_raw(data) as *mut c_void;
    unsafe {
      UniqueRef::from_raw(v8__ArrayBuffer__NewBackingStore__with_data(
        data_ptr,
        byte_length,
        backing_store_deleter_callback,
        null_mut(),
      ))
    }
  }
}
