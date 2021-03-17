// Copyright 2019-2021 the Deno authors. All rights reserved. MIT license.

use std::cell::Cell;
use std::ffi::c_void;
use std::ops::Deref;
use std::ptr::null_mut;
use std::slice;

use crate::support::long;
use crate::support::Opaque;
use crate::support::Shared;
use crate::support::SharedPtrBase;
use crate::support::SharedRef;
use crate::support::UniquePtr;
use crate::support::UniqueRef;
use crate::ArrayBuffer;
use crate::HandleScope;
use crate::Isolate;
use crate::Local;

extern "C" {
  fn v8__ArrayBuffer__Allocator__NewDefaultAllocator() -> *mut Allocator;
  fn v8__ArrayBuffer__Allocator__NewRustAllocator(
    handle: *const c_void,
    vtable: *const RustAllocatorVtable<c_void>,
  ) -> *mut Allocator;
  fn v8__ArrayBuffer__Allocator__DELETE(this: *mut Allocator);
  fn v8__ArrayBuffer__New__with_byte_length(
    isolate: *mut Isolate,
    byte_length: usize,
  ) -> *const ArrayBuffer;
  fn v8__ArrayBuffer__New__with_backing_store(
    isolate: *mut Isolate,
    backing_store: *const SharedRef<BackingStore>,
  ) -> *const ArrayBuffer;
  fn v8__ArrayBuffer__Detach(this: *const ArrayBuffer);
  fn v8__ArrayBuffer__IsDetachable(this: *const ArrayBuffer) -> bool;
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
    ptr: *const SharedPtrBase<BackingStore>,
  ) -> SharedPtrBase<BackingStore>;
  fn std__shared_ptr__v8__BackingStore__CONVERT__std__unique_ptr(
    unique_ptr: UniquePtr<BackingStore>,
  ) -> SharedPtrBase<BackingStore>;
  fn std__shared_ptr__v8__BackingStore__get(
    ptr: *const SharedPtrBase<BackingStore>,
  ) -> *mut BackingStore;
  fn std__shared_ptr__v8__BackingStore__reset(
    ptr: *mut SharedPtrBase<BackingStore>,
  );
  fn std__shared_ptr__v8__BackingStore__use_count(
    ptr: *const SharedPtrBase<BackingStore>,
  ) -> long;

  fn std__shared_ptr__v8__ArrayBuffer__Allocator__COPY(
    ptr: *const SharedPtrBase<Allocator>,
  ) -> SharedPtrBase<Allocator>;
  fn std__shared_ptr__v8__ArrayBuffer__Allocator__CONVERT__std__unique_ptr(
    unique_ptr: UniquePtr<Allocator>,
  ) -> SharedPtrBase<Allocator>;
  fn std__shared_ptr__v8__ArrayBuffer__Allocator__get(
    ptr: *const SharedPtrBase<Allocator>,
  ) -> *mut Allocator;
  fn std__shared_ptr__v8__ArrayBuffer__Allocator__reset(
    ptr: *mut SharedPtrBase<Allocator>,
  );
  fn std__shared_ptr__v8__ArrayBuffer__Allocator__use_count(
    ptr: *const SharedPtrBase<Allocator>,
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
#[derive(Debug)]
pub struct Allocator(Opaque);

/// A wrapper around the V8 Allocator class.
#[repr(C)]
pub struct RustAllocatorVtable<T> {
  pub allocate: unsafe extern "C" fn(handle: &T, len: usize) -> *mut c_void,
  pub allocate_uninitialized:
    unsafe extern "C" fn(handle: &T, len: usize) -> *mut c_void,
  pub free: unsafe extern "C" fn(handle: &T, data: *mut c_void, len: usize),
  pub reallocate: unsafe extern "C" fn(
    handle: &T,
    data: *mut c_void,
    old_length: usize,
    new_length: usize,
  ) -> *mut c_void,
  pub drop: unsafe extern "C" fn(handle: *const T),
}

impl Shared for Allocator {
  fn clone(ptr: &SharedPtrBase<Self>) -> SharedPtrBase<Self> {
    unsafe { std__shared_ptr__v8__ArrayBuffer__Allocator__COPY(ptr) }
  }
  fn from_unique_ptr(unique_ptr: UniquePtr<Self>) -> SharedPtrBase<Self> {
    unsafe {
      std__shared_ptr__v8__ArrayBuffer__Allocator__CONVERT__std__unique_ptr(
        unique_ptr,
      )
    }
  }
  fn get(ptr: &SharedPtrBase<Self>) -> *const Self {
    unsafe { std__shared_ptr__v8__ArrayBuffer__Allocator__get(ptr) }
  }
  fn reset(ptr: &mut SharedPtrBase<Self>) {
    unsafe { std__shared_ptr__v8__ArrayBuffer__Allocator__reset(ptr) }
  }
  fn use_count(ptr: &SharedPtrBase<Self>) -> long {
    unsafe { std__shared_ptr__v8__ArrayBuffer__Allocator__use_count(ptr) }
  }
}

/// malloc/free based convenience allocator.
pub fn new_default_allocator() -> UniqueRef<Allocator> {
  unsafe {
    UniqueRef::from_raw(v8__ArrayBuffer__Allocator__NewDefaultAllocator())
  }
}

/// Creates an allocator managed by Rust code.
///
/// Marked `unsafe` because the caller must ensure that `handle` is valid and matches what `vtable` expects.
pub unsafe fn new_rust_allocator<T: Sized + Send + Sync + 'static>(
  handle: *const T,
  vtable: &'static RustAllocatorVtable<T>,
) -> UniqueRef<Allocator> {
  UniqueRef::from_raw(v8__ArrayBuffer__Allocator__NewRustAllocator(
    handle as *const c_void,
    vtable as *const RustAllocatorVtable<T>
      as *const RustAllocatorVtable<c_void>,
  ))
}

#[test]
fn test_rust_allocator() {
  use std::sync::atomic::{AtomicUsize, Ordering};
  use std::sync::Arc;

  unsafe extern "C" fn allocate(_: &AtomicUsize, _: usize) -> *mut c_void {
    unimplemented!()
  }
  unsafe extern "C" fn allocate_uninitialized(
    _: &AtomicUsize,
    _: usize,
  ) -> *mut c_void {
    unimplemented!()
  }
  unsafe extern "C" fn free(_: &AtomicUsize, _: *mut c_void, _: usize) {
    unimplemented!()
  }
  unsafe extern "C" fn reallocate(
    _: &AtomicUsize,
    _: *mut c_void,
    _: usize,
    _: usize,
  ) -> *mut c_void {
    unimplemented!()
  }
  unsafe extern "C" fn drop(x: *const AtomicUsize) {
    let arc = Arc::from_raw(x);
    arc.store(42, Ordering::SeqCst);
  }

  let retval = Arc::new(AtomicUsize::new(0));

  let vtable: &'static RustAllocatorVtable<AtomicUsize> =
    &RustAllocatorVtable {
      allocate,
      allocate_uninitialized,
      free,
      reallocate,
      drop,
    };
  unsafe { new_rust_allocator(Arc::into_raw(retval.clone()), vtable) };
  assert_eq!(retval.load(Ordering::SeqCst), 42);
  assert_eq!(Arc::strong_count(&retval), 1);
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
#[derive(Debug)]
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
  type Target = [Cell<u8>];

  /// Returns a [u8] slice refencing the data in the backing store.
  fn deref(&self) -> &Self::Target {
    let data = self.data() as *mut Cell<u8>;
    let len = self.byte_length();
    unsafe { slice::from_raw_parts(data, len) }
  }
}

impl Drop for BackingStore {
  fn drop(&mut self) {
    unsafe { v8__BackingStore__DELETE(self) };
  }
}

impl Shared for BackingStore {
  fn clone(ptr: &SharedPtrBase<Self>) -> SharedPtrBase<Self> {
    unsafe { std__shared_ptr__v8__BackingStore__COPY(ptr) }
  }
  fn from_unique_ptr(unique_ptr: UniquePtr<Self>) -> SharedPtrBase<Self> {
    unsafe {
      std__shared_ptr__v8__BackingStore__CONVERT__std__unique_ptr(unique_ptr)
    }
  }
  fn get(ptr: &SharedPtrBase<Self>) -> *const Self {
    unsafe { std__shared_ptr__v8__BackingStore__get(ptr) }
  }
  fn reset(ptr: &mut SharedPtrBase<Self>) {
    unsafe { std__shared_ptr__v8__BackingStore__reset(ptr) }
  }
  fn use_count(ptr: &SharedPtrBase<Self>) -> long {
    unsafe { std__shared_ptr__v8__BackingStore__use_count(ptr) }
  }
}

impl ArrayBuffer {
  /// Create a new ArrayBuffer. Allocate |byte_length| bytes.
  /// Allocated memory will be owned by a created ArrayBuffer and
  /// will be deallocated when it is garbage-collected,
  /// unless the object is externalized.
  pub fn new<'s>(
    scope: &mut HandleScope<'s>,
    byte_length: usize,
  ) -> Local<'s, ArrayBuffer> {
    unsafe {
      scope.cast_local(|sd| {
        v8__ArrayBuffer__New__with_byte_length(
          sd.get_isolate_ptr(),
          byte_length,
        )
      })
    }
    .unwrap()
  }

  pub fn with_backing_store<'s>(
    scope: &mut HandleScope<'s>,
    backing_store: &SharedRef<BackingStore>,
  ) -> Local<'s, ArrayBuffer> {
    unsafe {
      scope.cast_local(|sd| {
        v8__ArrayBuffer__New__with_backing_store(
          sd.get_isolate_ptr(),
          backing_store,
        )
      })
    }
    .unwrap()
  }

  /// Data length in bytes.
  pub fn byte_length(&self) -> usize {
    unsafe { v8__ArrayBuffer__ByteLength(self) }
  }

  /// Returns true if this ArrayBuffer may be detached.
  pub fn is_detachable(&self) -> bool {
    unsafe { v8__ArrayBuffer__IsDetachable(self) }
  }

  /// Detaches this ArrayBuffer and all its views (typed arrays).
  /// Detaching sets the byte length of the buffer and all typed arrays to zero,
  /// preventing JavaScript from ever accessing underlying backing store.
  /// ArrayBuffer should have been externalized and must be detachable.
  pub fn detach(&self) {
    // V8 terminates when the ArrayBuffer is not detachable. Non-detachable
    // buffers are buffers that are in use by WebAssembly or asm.js.
    if self.is_detachable() {
      unsafe { v8__ArrayBuffer__Detach(self) }
    }
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
    scope: &mut Isolate,
    byte_length: usize,
  ) -> UniqueRef<BackingStore> {
    unsafe {
      UniqueRef::from_raw(v8__ArrayBuffer__NewBackingStore__with_byte_length(
        scope,
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

  /// Returns a new standalone BackingStore that is backed by the given buffer.
  /// The returned BackingStore will not take ownership of the buffer, and will
  /// thus not free the backing buffer on destruction. It is the callers'
  /// responsibility to free the backing buffer after the BackingStore has been
  /// destroyed.
  ///
  /// The result can be later passed to ArrayBuffer::New. The raw pointer
  /// to the buffer must not be passed again to any V8 API function.
  /// 
  /// # Safety
  ///
  /// Behavior is undefined if the combination of data_ptr and byte_length do
  /// not point to a valid [u8]. Behaviour is also undefined if 
  /// backing_store_deleter_callback and deleter_data are not valid.
  pub unsafe fn new_backing_store_from_raw_pointer(
    data_ptr: *mut c_void,
    byte_length: usize,
    backing_store_deleter_callback: BackingStoreDeleterCallback,
    deleter_data: *mut c_void,
  ) -> UniqueRef<BackingStore> {
    UniqueRef::from_raw(v8__ArrayBuffer__NewBackingStore__with_data(
      data_ptr,
      byte_length,
      backing_store_deleter_callback,
      user_data,
    ))
  }
}
