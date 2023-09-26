// Copyright 2019-2021 the Deno authors. All rights reserved. MIT license.

use std::ffi::c_void;
use std::ops::DerefMut;
use std::ptr::null_mut;

use crate::array_buffer::boxed_slice_deleter_callback;
use crate::array_buffer::vec_deleter_callback;
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
  fn v8__BackingStore__EmptyBackingStore(shared: bool) -> *mut BackingStore;
}

impl SharedArrayBuffer {
  /// Create a new SharedArrayBuffer. Allocate |byte_length| bytes.
  /// Allocated memory will be owned by a created SharedArrayBuffer and
  /// will be deallocated when it is garbage-collected,
  /// unless the object is externalized.
  #[inline(always)]
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

  #[inline(always)]
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

  /// Create a new, empty SharedArrayBuffer.
  #[inline(always)]
  pub fn empty<'s>(
    scope: &mut HandleScope<'s>,
  ) -> Local<'s, SharedArrayBuffer> {
    // SAFETY: This is a v8-provided empty backing store
    let backing_store =
      unsafe { UniqueRef::from_raw(v8__BackingStore__EmptyBackingStore(true)) };
    Self::with_backing_store(scope, &backing_store.make_shared())
  }

  /// Data length in bytes.
  #[inline(always)]
  pub fn byte_length(&self) -> usize {
    unsafe { v8__SharedArrayBuffer__ByteLength(self) }
  }

  /// Get a shared pointer to the backing store of this array buffer. This
  /// pointer coordinates the lifetime management of the internal storage
  /// with any live ArrayBuffers on the heap, even across isolates. The embedder
  /// should not attempt to manage lifetime of the storage through other means.
  #[inline(always)]
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
  #[inline(always)]
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
  #[inline(always)]
  pub fn new_backing_store_from_boxed_slice(
    data: Box<[u8]>,
  ) -> UniqueRef<BackingStore> {
    let byte_length = data.len();
    let data_ptr = Box::into_raw(data) as *mut c_void;
    unsafe {
      UniqueRef::from_raw(v8__SharedArrayBuffer__NewBackingStore__with_data(
        data_ptr,
        byte_length,
        boxed_slice_deleter_callback,
        null_mut(),
      ))
    }
  }

  /// Returns a new standalone BackingStore that takes over the ownership of
  /// the given buffer.
  ///
  /// The destructor of the BackingStore frees owned buffer memory.
  ///
  /// The result can be later passed to SharedArrayBuffer::New. The raw pointer
  /// to the buffer must not be passed again to any V8 API function.
  #[inline(always)]
  pub fn new_backing_store_from_vec(
    mut data: Vec<u8>,
  ) -> UniqueRef<BackingStore> {
    let byte_length = data.len();
    let data_ptr = data.as_mut_ptr() as *mut c_void;
    std::mem::forget(data);
    unsafe {
      UniqueRef::from_raw(v8__SharedArrayBuffer__NewBackingStore__with_data(
        data_ptr,
        byte_length,
        vec_deleter_callback,
        null_mut(),
      ))
    }
  }

  /// Returns a new standalone shared BackingStore backed by an object that dereferences
  /// to a mutable slice of bytes. The object is dereferenced once, and the resulting slice's
  /// memory is used for the lifetime of the buffer.
  ///
  /// As this buffer may be shared, the underlying object must be [`Send`] + [`Sync`].
  #[inline(always)]
  pub fn new_backing_store_from_bytes<T>(bytes: T) -> UniqueRef<BackingStore>
  where
    T: DerefMut<Target = [u8]> + Send + Sync,
  {
    // First we move the object into a box so it is pinned in place
    let alloc = Box::new(bytes);
    Self::new_backing_store_from_boxed_bytes(alloc)
  }

  /// Returns a new standalone shared BackingStore backed by an object that dereferences
  /// to a mutable slice of bytes. The object is dereferenced once, and the resulting slice's
  /// memory is used for the lifetime of the buffer.
  ///
  /// As this buffer may be shared, the underlying object must be [`Send`] + [`Sync`].
  #[inline(always)]
  pub fn new_backing_store_from_boxed_bytes<T>(
    mut bytes: Box<T>,
  ) -> UniqueRef<BackingStore>
  where
    T: DerefMut<Target = [u8]> + Send + Sync,
  {
    // We need to be very careful here not to move the data out of the box as that may
    // invalidate the slice's pointer. You can imagine a SmallVec that provides a pointer to
    // a slice inside of itself -- moving that SmallVec would invalidate the slice!
    let slice = bytes.deref_mut();
    let len = slice.len();
    let slice = slice.as_mut_ptr();
    let ptr = Box::into_raw(bytes) as *const c_void;

    extern "C" fn drop_box<T>(
      _ptr: *mut c_void,
      _len: usize,
      data: *mut c_void,
    ) {
      // SAFETY: We know that data is a raw Box from above
      unsafe { drop(Box::<T>::from_raw(data as _)) }
    }

    // SAFETY: We are extending the lifetime of a slice, but we're locking away the box that we
    // derefed from so there's no way to get another mutable reference.
    unsafe {
      Self::new_backing_store_from_ptr(slice as _, len, drop_box::<T>, ptr as _)
    }
  }

  /// Returns a new standalone shared BackingStore backed by given ptr.
  ///
  /// SAFETY: This API consumes raw pointers so is inherently
  /// unsafe. Usually you should use new_backing_store_from_boxed_slice.
  #[inline(always)]
  pub unsafe fn new_backing_store_from_ptr(
    data_ptr: *mut c_void,
    byte_length: usize,
    deleter_callback: BackingStoreDeleterCallback,
    deleter_data: *mut c_void,
  ) -> UniqueRef<BackingStore> {
    UniqueRef::from_raw(v8__SharedArrayBuffer__NewBackingStore__with_data(
      data_ptr,
      byte_length,
      deleter_callback,
      deleter_data,
    ))
  }
}
